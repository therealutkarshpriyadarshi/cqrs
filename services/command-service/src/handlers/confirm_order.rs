use axum::{extract::{Path, State}, http::StatusCode, Json};
use domain::{
    aggregates::order::OrderAggregate,
    commands::order_commands::ConfirmOrderCommand,
    events::{order_events::*, EventEnvelope, EventMetadata},
};
use event_store::Event;
use serde::Serialize;
use tracing::{error, info};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ConfirmOrderResponse {
    pub order_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Handle confirm order command
pub async fn handle(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Result<(StatusCode, Json<ConfirmOrderResponse>), (StatusCode, Json<ErrorResponse>)> {
    info!("Received confirm order command for order: {}", order_id);

    let cmd = ConfirmOrderCommand { order_id };

    // Load existing events
    let events = match state.event_store.load_events(cmd.order_id).await {
        Ok(events) => events,
        Err(e) => {
            error!("Failed to load events: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to load order: {}", e),
                }),
            ));
        }
    };

    if events.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Order not found".to_string(),
            }),
        ));
    }

    // Rebuild aggregate from events
    let mut aggregate = OrderAggregate::default();
    let mut version = 0i64;

    for event in events {
        version = event.sequence_number;
        match event.event_type.as_str() {
            "OrderCreated" => {
                let domain_event: OrderCreatedEvent =
                    serde_json::from_value(event.payload).unwrap();
                aggregate.apply_order_created(&domain_event);
            }
            "OrderConfirmed" => {
                let domain_event: OrderConfirmedEvent =
                    serde_json::from_value(event.payload).unwrap();
                aggregate.apply_order_confirmed(&domain_event);
            }
            "OrderCancelled" => {
                let domain_event: OrderCancelledEvent =
                    serde_json::from_value(event.payload).unwrap();
                aggregate.apply_order_cancelled(&domain_event);
            }
            _ => {}
        }
    }

    // Execute command
    let event = match aggregate.confirm() {
        Ok(event) => event,
        Err(e) => {
            error!("Failed to confirm order: {}", e);
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
        cmd.order_id,
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
        sequence_number: version + 1,
        created_at: event_envelope.timestamp,
    };

    // Persist event
    if let Err(e) = state
        .event_store
        .append_events(cmd.order_id, version, vec![store_event])
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
    if let Err(e) = state.event_publisher.publish(cmd.order_id, &event_envelope).await {
        error!("Failed to publish event to Kafka: {}", e);
    }

    info!("Order confirmed successfully: {}", cmd.order_id);

    Ok((
        StatusCode::OK,
        Json(ConfirmOrderResponse {
            order_id: cmd.order_id,
            status: "CONFIRMED".to_string(),
        }),
    ))
}
