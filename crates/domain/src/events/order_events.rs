use super::DomainEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderItem {
    pub product_id: Uuid,
    pub sku: String,
    pub quantity: u32,
    pub unit_price: f64,
}

impl OrderItem {
    pub fn new(product_id: Uuid, sku: String, quantity: u32, unit_price: f64) -> Self {
        Self {
            product_id,
            sku,
            quantity,
            unit_price,
        }
    }

    pub fn total_price(&self) -> f64 {
        self.unit_price * self.quantity as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreatedEvent {
    pub order_id: Uuid,
    pub customer_id: Uuid,
    pub order_number: String,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

impl DomainEvent for OrderCreatedEvent {
    fn event_type() -> &'static str {
        "OrderCreated"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderConfirmedEvent {
    pub order_id: Uuid,
    pub confirmed_at: DateTime<Utc>,
}

impl DomainEvent for OrderConfirmedEvent {
    fn event_type() -> &'static str {
        "OrderConfirmed"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCancelledEvent {
    pub order_id: Uuid,
    pub reason: String,
    pub cancelled_at: DateTime<Utc>,
}

impl DomainEvent for OrderCancelledEvent {
    fn event_type() -> &'static str {
        "OrderCancelled"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderShippedEvent {
    pub order_id: Uuid,
    pub tracking_number: String,
    pub carrier: String,
    pub shipped_at: DateTime<Utc>,
}

impl DomainEvent for OrderShippedEvent {
    fn event_type() -> &'static str {
        "OrderShipped"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDeliveredEvent {
    pub order_id: Uuid,
    pub delivered_at: DateTime<Utc>,
}

impl DomainEvent for OrderDeliveredEvent {
    fn event_type() -> &'static str {
        "OrderDelivered"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_item_total_price() {
        let item = OrderItem::new(
            Uuid::new_v4(),
            "SKU-001".to_string(),
            3,
            10.50,
        );
        assert_eq!(item.total_price(), 31.50);
    }

    #[test]
    fn test_order_created_event_type() {
        assert_eq!(OrderCreatedEvent::event_type(), "OrderCreated");
    }

    #[test]
    fn test_order_confirmed_event_type() {
        assert_eq!(OrderConfirmedEvent::event_type(), "OrderConfirmed");
    }

    #[test]
    fn test_order_cancelled_event_type() {
        assert_eq!(OrderCancelledEvent::event_type(), "OrderCancelled");
    }
}
