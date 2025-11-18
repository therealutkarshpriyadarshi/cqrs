use domain::events::order_events::*;
use read_model::OrderProjection;
use serde_json::Value;
use tracing::{error, info, warn};

/// Processes events and updates projections
pub struct EventProcessor {
    projection: OrderProjection,
}

impl EventProcessor {
    pub fn new(projection: OrderProjection) -> Self {
        Self { projection }
    }

    /// Process a single event
    pub async fn process_event(&self, event_type: &str, payload: Value) -> anyhow::Result<()> {
        info!("Processing event: {}", event_type);

        match event_type {
            "OrderCreated" => {
                let event: OrderCreatedEvent = serde_json::from_value(payload)?;
                self.projection.handle_order_created(&event).await?;
                info!("Successfully processed OrderCreated for order_id: {}", event.order_id);
            }
            "OrderConfirmed" => {
                let event: OrderConfirmedEvent = serde_json::from_value(payload)?;
                self.projection.handle_order_confirmed(&event).await?;
                info!("Successfully processed OrderConfirmed for order_id: {}", event.order_id);
            }
            "OrderCancelled" => {
                let event: OrderCancelledEvent = serde_json::from_value(payload)?;
                self.projection.handle_order_cancelled(&event).await?;
                info!("Successfully processed OrderCancelled for order_id: {}", event.order_id);
            }
            "OrderShipped" => {
                let event: OrderShippedEvent = serde_json::from_value(payload)?;
                self.projection.handle_order_shipped(&event).await?;
                info!("Successfully processed OrderShipped for order_id: {}", event.order_id);
            }
            "OrderDelivered" => {
                let event: OrderDeliveredEvent = serde_json::from_value(payload)?;
                self.projection.handle_order_delivered(&event).await?;
                info!("Successfully processed OrderDelivered for order_id: {}", event.order_id);
            }
            _ => {
                warn!("Unknown event type: {}, skipping", event_type);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sqlx::PgPool;
    use uuid::Uuid;

    #[test]
    fn test_processor_creation() {
        let pool = PgPool::connect_lazy("postgresql://test").unwrap();
        let projection = OrderProjection::new(pool);
        let _processor = EventProcessor::new(projection);
    }

    #[test]
    fn test_event_deserialization() {
        let event = OrderCreatedEvent {
            order_id: Uuid::new_v4(),
            customer_id: Uuid::new_v4(),
            order_number: "ORD-123".to_string(),
            items: vec![],
            total_amount: 100.0,
            currency: "USD".to_string(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_value(&event).unwrap();
        let _deserialized: OrderCreatedEvent = serde_json::from_value(json).unwrap();
    }
}
