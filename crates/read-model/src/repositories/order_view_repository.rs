use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::ReadModelError;

/// Read model representation of an order
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrderView {
    pub order_id: Uuid,
    pub customer_id: Uuid,
    pub order_number: String,
    pub status: String,
    pub total_amount: f64,
    pub currency: String,
    pub items: serde_json::Value,
    pub shipping_address: Option<serde_json::Value>,
    pub tracking_number: Option<String>,
    pub carrier: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

/// Repository for querying order views
#[async_trait]
pub trait OrderViewRepository: Send + Sync {
    /// Get a single order by ID
    async fn get_by_id(&self, order_id: Uuid) -> Result<Option<OrderView>, ReadModelError>;

    /// List orders for a customer
    async fn list_by_customer(
        &self,
        customer_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrderView>, ReadModelError>;

    /// List orders by status
    async fn list_by_status(
        &self,
        status: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrderView>, ReadModelError>;

    /// Search orders by order number
    async fn search_by_order_number(
        &self,
        order_number: &str,
    ) -> Result<Option<OrderView>, ReadModelError>;

    /// Count total orders for a customer
    async fn count_by_customer(&self, customer_id: Uuid) -> Result<i64, ReadModelError>;
}

/// PostgreSQL implementation of OrderViewRepository
pub struct PostgresOrderViewRepository {
    pool: PgPool,
}

impl PostgresOrderViewRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrderViewRepository for PostgresOrderViewRepository {
    async fn get_by_id(&self, order_id: Uuid) -> Result<Option<OrderView>, ReadModelError> {
        let order = sqlx::query_as::<_, OrderView>(
            r#"
            SELECT
                order_id, customer_id, order_number, status,
                total_amount, currency, items, shipping_address,
                tracking_number, carrier, created_at, updated_at, version
            FROM order_views
            WHERE order_id = $1
            "#,
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(order)
    }

    async fn list_by_customer(
        &self,
        customer_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrderView>, ReadModelError> {
        let orders = sqlx::query_as::<_, OrderView>(
            r#"
            SELECT
                order_id, customer_id, order_number, status,
                total_amount, currency, items, shipping_address,
                tracking_number, carrier, created_at, updated_at, version
            FROM order_views
            WHERE customer_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(customer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(orders)
    }

    async fn list_by_status(
        &self,
        status: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<OrderView>, ReadModelError> {
        let orders = sqlx::query_as::<_, OrderView>(
            r#"
            SELECT
                order_id, customer_id, order_number, status,
                total_amount, currency, items, shipping_address,
                tracking_number, carrier, created_at, updated_at, version
            FROM order_views
            WHERE status = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(orders)
    }

    async fn search_by_order_number(
        &self,
        order_number: &str,
    ) -> Result<Option<OrderView>, ReadModelError> {
        let order = sqlx::query_as::<_, OrderView>(
            r#"
            SELECT
                order_id, customer_id, order_number, status,
                total_amount, currency, items, shipping_address,
                tracking_number, carrier, created_at, updated_at, version
            FROM order_views
            WHERE order_number = $1
            "#,
        )
        .bind(order_number)
        .fetch_optional(&self.pool)
        .await?;

        Ok(order)
    }

    async fn count_by_customer(&self, customer_id: Uuid) -> Result<i64, ReadModelError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM order_views
            WHERE customer_id = $1
            "#,
        )
        .bind(customer_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_view_serialization() {
        let order = OrderView {
            order_id: Uuid::new_v4(),
            customer_id: Uuid::new_v4(),
            order_number: "ORD-123".to_string(),
            status: "CREATED".to_string(),
            total_amount: 99.99,
            currency: "USD".to_string(),
            items: serde_json::json!([]),
            shipping_address: None,
            tracking_number: None,
            carrier: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        let json = serde_json::to_string(&order).unwrap();
        let deserialized: OrderView = serde_json::from_str(&json).unwrap();

        assert_eq!(order.order_id, deserialized.order_id);
        assert_eq!(order.order_number, deserialized.order_number);
    }
}
