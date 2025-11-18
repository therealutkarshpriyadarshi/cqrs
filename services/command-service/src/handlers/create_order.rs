use axum::{extract::State, http::StatusCode, Json};
use domain::{
    aggregates::order::OrderAggregate,
    commands::order_commands::CreateOrderCommand,
    events::{order_events::OrderItem, EventEnvelope, EventMetadata},
};
use event_store::Event;
use serde::Serialize;
use tracing::{error, info};
use uuid::Uuid;
use validator::Validate;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct CreateOrderResponse {
    pub order_id: Uuid,
    pub order_number: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Handle create order command
pub async fn handle(
    State(state): State<AppState>,
    Json(cmd): Json<CreateOrderCommand>,
) -> Result<(StatusCode, Json<CreateOrderResponse>), (StatusCode, Json<ErrorResponse>)> {
    info!("Received create order command for customer: {}", cmd.customer_id);

    // Validate command
    if let Err(e) = cmd.validate() {
        error!("Validation error: {}", e);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Validation error: {}", e),
            }),
        ));
    }

    // Convert command items to domain items
    let items: Vec<OrderItem> = cmd
        .items
        .iter()
        .map(|i| OrderItem {
            product_id: i.product_id,
            sku: i.sku.clone(),
            quantity: i.quantity,
            unit_price: i.unit_price,
        })
        .collect();

    // Create aggregate and generate event
    let (aggregate, event) = match OrderAggregate::create(cmd.customer_id, items) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to create order aggregate: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ));
        }
    };

    // Create event envelope
    let correlation_id = Uuid::new_v4();
    let event_envelope = EventEnvelope::new(
        aggregate.id,
        "Order".to_string(),
        event,
        EventMetadata {
            correlation_id,
            causation_id: correlation_id,
            user_id: None,
        },
    );

    // Convert to event store event
    let store_event = Event {
        event_id: event_envelope.event_id,
        aggregate_id: event_envelope.aggregate_id,
        aggregate_type: event_envelope.aggregate_type.clone(),
        event_type: event_envelope.event_type.clone(),
        event_version: event_envelope.event_version,
        payload: event_envelope.payload.clone(),
        metadata: serde_json::to_value(&event_envelope.metadata).unwrap(),
        sequence_number: 1,
        created_at: event_envelope.timestamp,
    };

    // Persist event to event store
    if let Err(e) = state
        .event_store
        .append_events(aggregate.id, 0, vec![store_event])
        .await
    {
        error!("Failed to append events: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to persist event: {}", e),
            }),
        ));
    }

    // Publish to Kafka
    if let Err(e) = state.event_publisher.publish(aggregate.id, &event_envelope).await {
        error!("Failed to publish event to Kafka: {}", e);
        // Note: Event is already persisted, so we don't fail the request
        // In production, you might want to implement a retry mechanism
    }

    info!("Order created successfully: {}", aggregate.id);

    Ok((
        StatusCode::CREATED,
        Json(CreateOrderResponse {
            order_id: aggregate.id,
            order_number: aggregate.order_number,
            status: "CREATED".to_string(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::commands::order_commands::{CreateOrderItem, ShippingAddress};

    #[test]
    fn test_create_order_command_validation() {
        let cmd = CreateOrderCommand {
            customer_id: Uuid::new_v4(),
            items: vec![CreateOrderItem {
                product_id: Uuid::new_v4(),
                sku: "SKU-001".to_string(),
                quantity: 2,
                unit_price: 10.50,
            }],
            shipping_address: ShippingAddress {
                street: "123 Main St".to_string(),
                city: "Springfield".to_string(),
                state: "IL".to_string(),
                zip: "62701".to_string(),
                country: "US".to_string(),
            },
        };

        assert!(cmd.validate().is_ok());
    }

    #[test]
    fn test_create_order_with_empty_items_fails_validation() {
        let cmd = CreateOrderCommand {
            customer_id: Uuid::new_v4(),
            items: vec![],
            shipping_address: ShippingAddress {
                street: "123 Main St".to_string(),
                city: "Springfield".to_string(),
                state: "IL".to_string(),
                zip: "62701".to_string(),
                country: "US".to_string(),
            },
        };

        assert!(cmd.validate().is_err());
    }
}
