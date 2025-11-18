use domain::events::order_events::*;
use sqlx::PgPool;
use tracing::{error, info};

use crate::ReadModelError;

/// Handles projecting order events into the read model
pub struct OrderProjection {
    pool: PgPool,
}

impl OrderProjection {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Handle OrderCreated event
    pub async fn handle_order_created(
        &self,
        event: &OrderCreatedEvent,
    ) -> Result<(), ReadModelError> {
        info!(
            "Projecting OrderCreated event for order_id: {}",
            event.order_id
        );

        let result = sqlx::query(
            r#"
            INSERT INTO order_views (
                order_id, customer_id, order_number, status,
                total_amount, currency, items, created_at, updated_at, version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 1)
            ON CONFLICT (order_id) DO NOTHING
            "#,
        )
        .bind(event.order_id)
        .bind(event.customer_id)
        .bind(&event.order_number)
        .bind("CREATED")
        .bind(event.total_amount)
        .bind(&event.currency)
        .bind(serde_json::to_value(&event.items)?)
        .bind(event.created_at)
        .bind(event.created_at)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                info!(
                    "Successfully projected OrderCreated for order_id: {}",
                    event.order_id
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to project OrderCreated for order_id: {}, error: {}",
                    event.order_id, e
                );
                Err(ReadModelError::DatabaseError(e))
            }
        }
    }

    /// Handle OrderConfirmed event
    pub async fn handle_order_confirmed(
        &self,
        event: &OrderConfirmedEvent,
    ) -> Result<(), ReadModelError> {
        info!(
            "Projecting OrderConfirmed event for order_id: {}",
            event.order_id
        );

        sqlx::query(
            r#"
            UPDATE order_views
            SET status = 'CONFIRMED', updated_at = $1, version = version + 1
            WHERE order_id = $2
            "#,
        )
        .bind(event.confirmed_at)
        .bind(event.order_id)
        .execute(&self.pool)
        .await?;

        info!(
            "Successfully projected OrderConfirmed for order_id: {}",
            event.order_id
        );
        Ok(())
    }

    /// Handle OrderCancelled event
    pub async fn handle_order_cancelled(
        &self,
        event: &OrderCancelledEvent,
    ) -> Result<(), ReadModelError> {
        info!(
            "Projecting OrderCancelled event for order_id: {}",
            event.order_id
        );

        sqlx::query(
            r#"
            UPDATE order_views
            SET status = 'CANCELLED', updated_at = $1, version = version + 1
            WHERE order_id = $2
            "#,
        )
        .bind(event.cancelled_at)
        .bind(event.order_id)
        .execute(&self.pool)
        .await?;

        info!(
            "Successfully projected OrderCancelled for order_id: {}",
            event.order_id
        );
        Ok(())
    }

    /// Handle OrderShipped event
    pub async fn handle_order_shipped(
        &self,
        event: &OrderShippedEvent,
    ) -> Result<(), ReadModelError> {
        info!(
            "Projecting OrderShipped event for order_id: {}",
            event.order_id
        );

        sqlx::query(
            r#"
            UPDATE order_views
            SET status = 'SHIPPED',
                tracking_number = $1,
                carrier = $2,
                updated_at = $3,
                version = version + 1
            WHERE order_id = $4
            "#,
        )
        .bind(&event.tracking_number)
        .bind(&event.carrier)
        .bind(event.shipped_at)
        .bind(event.order_id)
        .execute(&self.pool)
        .await?;

        info!(
            "Successfully projected OrderShipped for order_id: {}",
            event.order_id
        );
        Ok(())
    }

    /// Handle OrderDelivered event
    pub async fn handle_order_delivered(
        &self,
        event: &OrderDeliveredEvent,
    ) -> Result<(), ReadModelError> {
        info!(
            "Projecting OrderDelivered event for order_id: {}",
            event.order_id
        );

        sqlx::query(
            r#"
            UPDATE order_views
            SET status = 'DELIVERED', updated_at = $1, version = version + 1
            WHERE order_id = $2
            "#,
        )
        .bind(event.delivered_at)
        .bind(event.order_id)
        .execute(&self.pool)
        .await?;

        info!(
            "Successfully projected OrderDelivered for order_id: {}",
            event.order_id
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_projection_creation() {
        // This test just ensures the projection can be instantiated
        // Real tests would require database access
        let pool = PgPool::connect_lazy("postgresql://test").unwrap();
        let _projection = OrderProjection::new(pool);
    }

    #[test]
    fn test_event_serialization() {
        let event = OrderCreatedEvent {
            order_id: Uuid::new_v4(),
            customer_id: Uuid::new_v4(),
            order_number: "ORD-123".to_string(),
            items: vec![],
            total_amount: 100.0,
            currency: "USD".to_string(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_value(&event.items).unwrap();
        assert!(json.is_array());
    }
}
