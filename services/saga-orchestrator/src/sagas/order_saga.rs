use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use domain::events::inventory_events::{
    InventoryItem, InventoryReleasedEvent, InventoryReservedEvent,
};
use domain::events::order_events::{OrderConfirmedEvent, OrderItem};
use domain::events::payment_events::{PaymentAuthorizedEvent, PaymentVoidedEvent};
use domain::events::{DomainEvent, EventEnvelope, EventMetadata};
use messaging::producer::EventPublisher;
use saga::errors::{Result, SagaError};
use saga::step::{SagaStep, StepContext, StepExecutor};
use saga::{Saga, SagaState};

/// Data passed to the order processing saga
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSagaData {
    pub order_id: Uuid,
    pub customer_id: Uuid,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
    pub currency: String,
    pub payment_method: String,
    pub correlation_id: Uuid,
}

/// Order Processing Saga
///
/// Steps:
/// 1. Reserve Inventory → Compensate: Release Inventory
/// 2. Authorize Payment → Compensate: Void Authorization
/// 3. Confirm Order → Compensate: Cancel Order
pub struct OrderProcessingSaga {
    executors: HashMap<String, Box<dyn StepExecutor>>,
}

impl OrderProcessingSaga {
    pub fn new(event_publisher: Arc<EventPublisher>) -> Self {
        let mut executors: HashMap<String, Box<dyn StepExecutor>> = HashMap::new();

        executors.insert(
            "reserve_inventory".to_string(),
            Box::new(ReserveInventoryStep::new(event_publisher.clone())),
        );

        executors.insert(
            "authorize_payment".to_string(),
            Box::new(AuthorizePaymentStep::new(event_publisher.clone())),
        );

        executors.insert(
            "confirm_order".to_string(),
            Box::new(ConfirmOrderStep::new(event_publisher.clone())),
        );

        Self { executors }
    }
}

#[async_trait]
impl Saga for OrderProcessingSaga {
    fn saga_type(&self) -> &str {
        "OrderProcessingSaga"
    }

    fn step_executors(&self) -> &HashMap<String, Box<dyn StepExecutor>> {
        &self.executors
    }

    async fn create_state(&self, saga_id: Uuid, data: serde_json::Value) -> Result<SagaState> {
        let steps = vec![
            SagaStep::new("reserve_inventory".to_string(), 3),
            SagaStep::new("authorize_payment".to_string(), 3),
            SagaStep::new("confirm_order".to_string(), 3),
        ];

        Ok(SagaState::new(
            saga_id,
            self.saga_type().to_string(),
            steps,
            data,
        ))
    }
}

// ============================================================================
// Step 1: Reserve Inventory
// ============================================================================

struct ReserveInventoryStep {
    event_publisher: Arc<EventPublisher>,
}

impl ReserveInventoryStep {
    fn new(event_publisher: Arc<EventPublisher>) -> Self {
        Self { event_publisher }
    }
}

#[async_trait]
impl StepExecutor for ReserveInventoryStep {
    async fn execute(&self, context: &StepContext) -> Result<serde_json::Value> {
        info!(saga_id = %context.saga_id, "Executing: Reserve Inventory");

        let saga_data: OrderSagaData = serde_json::from_value(context.data.clone())
            .map_err(|e| SagaError::InternalError(format!("Failed to parse saga data: {}", e)))?;

        // Convert OrderItems to InventoryItems
        let inventory_items: Vec<InventoryItem> = saga_data
            .items
            .iter()
            .map(|item| InventoryItem {
                product_id: item.product_id,
                sku: item.sku.clone(),
                quantity: item.quantity,
            })
            .collect();

        // Create inventory reserved event
        let reservation_id = Uuid::new_v4();
        let event = InventoryReservedEvent {
            reservation_id,
            order_id: saga_data.order_id,
            items: inventory_items,
            reserved_at: Utc::now(),
        };

        // Create event envelope
        let metadata = EventMetadata::with_correlation(saga_data.correlation_id);
        let envelope = event
            .to_envelope(saga_data.order_id, "Order", metadata)
            .map_err(|e| SagaError::InternalError(format!("Failed to create envelope: {}", e)))?;

        // Publish event
        self.event_publisher
            .publish(saga_data.order_id, &envelope)
            .await
            .map_err(|e| {
                SagaError::StepExecutionFailed(format!("Failed to publish event: {}", e))
            })?;

        info!(
            saga_id = %context.saga_id,
            reservation_id = %reservation_id,
            "Inventory reserved successfully"
        );

        Ok(serde_json::json!({
            "reservation_id": reservation_id,
            "items_reserved": event.items.len()
        }))
    }

    async fn compensate(&self, context: &StepContext) -> Result<()> {
        info!(saga_id = %context.saga_id, "Compensating: Release Inventory");

        let saga_data: OrderSagaData = serde_json::from_value(context.data.clone())
            .map_err(|e| SagaError::InternalError(format!("Failed to parse saga data: {}", e)))?;

        // Get reservation_id from step result
        let step_result = context.data.get("result");
        let reservation_id = if let Some(result) = step_result {
            result
                .get("reservation_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4)
        } else {
            Uuid::new_v4()
        };

        let inventory_items: Vec<InventoryItem> = saga_data
            .items
            .iter()
            .map(|item| InventoryItem {
                product_id: item.product_id,
                sku: item.sku.clone(),
                quantity: item.quantity,
            })
            .collect();

        // Create inventory released event (compensation)
        let event = InventoryReleasedEvent {
            reservation_id,
            order_id: saga_data.order_id,
            items: inventory_items,
            released_at: Utc::now(),
            reason: "Saga compensation - order processing failed".to_string(),
        };

        let metadata = EventMetadata::with_correlation(saga_data.correlation_id);
        let envelope = event
            .to_envelope(saga_data.order_id, "Order", metadata)
            .map_err(|e| SagaError::CompensationFailed(format!("Failed to create envelope: {}", e)))?;

        self.event_publisher
            .publish(saga_data.order_id, &envelope)
            .await
            .map_err(|e| {
                SagaError::CompensationFailed(format!("Failed to publish event: {}", e))
            })?;

        info!(saga_id = %context.saga_id, "Inventory released successfully");

        Ok(())
    }
}

// ============================================================================
// Step 2: Authorize Payment
// ============================================================================

struct AuthorizePaymentStep {
    event_publisher: Arc<EventPublisher>,
}

impl AuthorizePaymentStep {
    fn new(event_publisher: Arc<EventPublisher>) -> Self {
        Self { event_publisher }
    }
}

#[async_trait]
impl StepExecutor for AuthorizePaymentStep {
    async fn execute(&self, context: &StepContext) -> Result<serde_json::Value> {
        info!(saga_id = %context.saga_id, "Executing: Authorize Payment");

        let saga_data: OrderSagaData = serde_json::from_value(context.data.clone())
            .map_err(|e| SagaError::InternalError(format!("Failed to parse saga data: {}", e)))?;

        let payment_id = Uuid::new_v4();
        let authorization_code = format!("AUTH-{}", Uuid::new_v4().simple());

        // Create payment authorized event
        let event = PaymentAuthorizedEvent {
            payment_id,
            order_id: saga_data.order_id,
            amount: saga_data.total_amount,
            currency: saga_data.currency.clone(),
            payment_method: saga_data.payment_method.clone(),
            authorization_code: authorization_code.clone(),
            authorized_at: Utc::now(),
        };

        let metadata = EventMetadata::with_correlation(saga_data.correlation_id);
        let envelope = event
            .to_envelope(saga_data.order_id, "Order", metadata)
            .map_err(|e| SagaError::InternalError(format!("Failed to create envelope: {}", e)))?;

        self.event_publisher
            .publish(saga_data.order_id, &envelope)
            .await
            .map_err(|e| {
                SagaError::StepExecutionFailed(format!("Failed to publish event: {}", e))
            })?;

        info!(
            saga_id = %context.saga_id,
            payment_id = %payment_id,
            authorization_code = %authorization_code,
            "Payment authorized successfully"
        );

        Ok(serde_json::json!({
            "payment_id": payment_id,
            "authorization_code": authorization_code,
            "amount": saga_data.total_amount
        }))
    }

    async fn compensate(&self, context: &StepContext) -> Result<()> {
        info!(saga_id = %context.saga_id, "Compensating: Void Payment Authorization");

        let saga_data: OrderSagaData = serde_json::from_value(context.data.clone())
            .map_err(|e| SagaError::InternalError(format!("Failed to parse saga data: {}", e)))?;

        // Get payment_id from step result
        let step_result = context.data.get("result");
        let payment_id = if let Some(result) = step_result {
            result
                .get("payment_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4)
        } else {
            Uuid::new_v4()
        };

        // Create payment voided event (compensation)
        let event = PaymentVoidedEvent {
            payment_id,
            order_id: saga_data.order_id,
            amount: saga_data.total_amount,
            currency: saga_data.currency.clone(),
            reason: "Saga compensation - order processing failed".to_string(),
            voided_at: Utc::now(),
        };

        let metadata = EventMetadata::with_correlation(saga_data.correlation_id);
        let envelope = event
            .to_envelope(saga_data.order_id, "Order", metadata)
            .map_err(|e| SagaError::CompensationFailed(format!("Failed to create envelope: {}", e)))?;

        self.event_publisher
            .publish(saga_data.order_id, &envelope)
            .await
            .map_err(|e| {
                SagaError::CompensationFailed(format!("Failed to publish event: {}", e))
            })?;

        info!(saga_id = %context.saga_id, "Payment authorization voided successfully");

        Ok(())
    }
}

// ============================================================================
// Step 3: Confirm Order
// ============================================================================

struct ConfirmOrderStep {
    event_publisher: Arc<EventPublisher>,
}

impl ConfirmOrderStep {
    fn new(event_publisher: Arc<EventPublisher>) -> Self {
        Self { event_publisher }
    }
}

#[async_trait]
impl StepExecutor for ConfirmOrderStep {
    async fn execute(&self, context: &StepContext) -> Result<serde_json::Value> {
        info!(saga_id = %context.saga_id, "Executing: Confirm Order");

        let saga_data: OrderSagaData = serde_json::from_value(context.data.clone())
            .map_err(|e| SagaError::InternalError(format!("Failed to parse saga data: {}", e)))?;

        // Create order confirmed event
        let event = OrderConfirmedEvent {
            order_id: saga_data.order_id,
            confirmed_at: Utc::now(),
        };

        let metadata = EventMetadata::with_correlation(saga_data.correlation_id);
        let envelope = event
            .to_envelope(saga_data.order_id, "Order", metadata)
            .map_err(|e| SagaError::InternalError(format!("Failed to create envelope: {}", e)))?;

        self.event_publisher
            .publish(saga_data.order_id, &envelope)
            .await
            .map_err(|e| {
                SagaError::StepExecutionFailed(format!("Failed to publish event: {}", e))
            })?;

        info!(
            saga_id = %context.saga_id,
            order_id = %saga_data.order_id,
            "Order confirmed successfully"
        );

        Ok(serde_json::json!({
            "order_id": saga_data.order_id,
            "confirmed": true
        }))
    }

    async fn compensate(&self, context: &StepContext) -> Result<()> {
        info!(saga_id = %context.saga_id, "Compensating: Cancel Order Confirmation");

        // In a real implementation, this would publish an OrderCancelled event
        // For now, we just log it
        info!(
            saga_id = %context.saga_id,
            "Order confirmation compensation completed (would cancel order)"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_saga_data_serialization() {
        let data = OrderSagaData {
            order_id: Uuid::new_v4(),
            customer_id: Uuid::new_v4(),
            items: vec![],
            total_amount: 99.99,
            currency: "USD".to_string(),
            payment_method: "credit_card".to_string(),
            correlation_id: Uuid::new_v4(),
        };

        let json = serde_json::to_value(&data).unwrap();
        let deserialized: OrderSagaData = serde_json::from_value(json).unwrap();

        assert_eq!(data.order_id, deserialized.order_id);
        assert_eq!(data.total_amount, deserialized.total_amount);
    }
}
