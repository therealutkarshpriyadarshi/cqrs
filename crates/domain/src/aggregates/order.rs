use crate::events::order_events::*;
use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Created,
    Confirmed,
    Cancelled,
    Shipped,
    Delivered,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Created => "CREATED",
            OrderStatus::Confirmed => "CONFIRMED",
            OrderStatus::Cancelled => "CANCELLED",
            OrderStatus::Shipped => "SHIPPED",
            OrderStatus::Delivered => "DELIVERED",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderAggregate {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub order_number: String,
    pub status: OrderStatus,
    pub items: Vec<OrderItem>,
    pub total_amount: f64,
    pub version: i64,
}

impl OrderAggregate {
    /// Create new order aggregate from command
    pub fn create(
        customer_id: Uuid,
        items: Vec<OrderItem>,
    ) -> Result<(Self, OrderCreatedEvent), OrderError> {
        if items.is_empty() {
            return Err(OrderError::NoItems);
        }

        // Validate all items have positive quantities and prices
        for item in &items {
            if item.quantity == 0 {
                return Err(OrderError::InvalidQuantity);
            }
            if item.unit_price <= 0.0 {
                return Err(OrderError::InvalidPrice);
            }
        }

        let order_id = Uuid::new_v4();
        let total_amount = items.iter().map(|i| i.total_price()).sum();
        let order_number = format!("ORD-{}", Uuid::new_v4().simple());

        let event = OrderCreatedEvent {
            order_id,
            customer_id,
            order_number: order_number.clone(),
            items: items.clone(),
            total_amount,
            currency: "USD".to_string(),
            created_at: Utc::now(),
        };

        let aggregate = Self {
            id: order_id,
            customer_id,
            order_number,
            status: OrderStatus::Created,
            items,
            total_amount,
            version: 0,
        };

        Ok((aggregate, event))
    }

    /// Create a new empty aggregate (for event sourcing reconstruction)
    pub fn new() -> Self {
        Self {
            id: Uuid::nil(),
            customer_id: Uuid::nil(),
            order_number: String::new(),
            status: OrderStatus::Created,
            items: Vec::new(),
            total_amount: 0.0,
            version: 0,
        }
    }

    /// Apply OrderCreated event to rebuild state
    pub fn apply_order_created(&mut self, event: &OrderCreatedEvent) {
        self.id = event.order_id;
        self.customer_id = event.customer_id;
        self.order_number = event.order_number.clone();
        self.items = event.items.clone();
        self.total_amount = event.total_amount;
        self.status = OrderStatus::Created;
        self.version += 1;
    }

    /// Apply OrderConfirmed event
    pub fn apply_order_confirmed(&mut self, _event: &OrderConfirmedEvent) {
        self.status = OrderStatus::Confirmed;
        self.version += 1;
    }

    /// Apply OrderCancelled event
    pub fn apply_order_cancelled(&mut self, _event: &OrderCancelledEvent) {
        self.status = OrderStatus::Cancelled;
        self.version += 1;
    }

    /// Apply OrderShipped event
    pub fn apply_order_shipped(&mut self, _event: &OrderShippedEvent) {
        self.status = OrderStatus::Shipped;
        self.version += 1;
    }

    /// Apply OrderDelivered event
    pub fn apply_order_delivered(&mut self, _event: &OrderDeliveredEvent) {
        self.status = OrderStatus::Delivered;
        self.version += 1;
    }

    /// Confirm order
    pub fn confirm(&self) -> Result<OrderConfirmedEvent, OrderError> {
        match self.status {
            OrderStatus::Created => Ok(OrderConfirmedEvent {
                order_id: self.id,
                confirmed_at: Utc::now(),
            }),
            _ => Err(OrderError::InvalidStatus {
                current: self.status.as_str(),
                operation: "confirm",
            }),
        }
    }

    /// Cancel order
    pub fn cancel(&self, reason: String) -> Result<OrderCancelledEvent, OrderError> {
        match self.status {
            OrderStatus::Shipped | OrderStatus::Delivered => Err(OrderError::CannotCancel),
            OrderStatus::Cancelled => Err(OrderError::AlreadyCancelled),
            _ => Ok(OrderCancelledEvent {
                order_id: self.id,
                reason,
                cancelled_at: Utc::now(),
            }),
        }
    }

    /// Ship order
    pub fn ship(
        &self,
        tracking_number: String,
        carrier: String,
    ) -> Result<OrderShippedEvent, OrderError> {
        match self.status {
            OrderStatus::Confirmed => Ok(OrderShippedEvent {
                order_id: self.id,
                tracking_number,
                carrier,
                shipped_at: Utc::now(),
            }),
            OrderStatus::Cancelled => Err(OrderError::OrderCancelled),
            _ => Err(OrderError::InvalidStatus {
                current: self.status.as_str(),
                operation: "ship",
            }),
        }
    }

    /// Mark order as delivered
    pub fn deliver(&self) -> Result<OrderDeliveredEvent, OrderError> {
        match self.status {
            OrderStatus::Shipped => Ok(OrderDeliveredEvent {
                order_id: self.id,
                delivered_at: Utc::now(),
            }),
            _ => Err(OrderError::InvalidStatus {
                current: self.status.as_str(),
                operation: "deliver",
            }),
        }
    }
}

impl Default for OrderAggregate {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum OrderError {
    #[error("Order must have at least one item")]
    NoItems,

    #[error("Invalid item quantity")]
    InvalidQuantity,

    #[error("Invalid item price")]
    InvalidPrice,

    #[error("Invalid order status '{current}' for operation '{operation}'")]
    InvalidStatus {
        current: &'static str,
        operation: &'static str,
    },

    #[error("Cannot cancel shipped or delivered order")]
    CannotCancel,

    #[error("Order already cancelled")]
    AlreadyCancelled,

    #[error("Order is cancelled")]
    OrderCancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_order_success() {
        let customer_id = Uuid::new_v4();
        let items = vec![OrderItem::new(
            Uuid::new_v4(),
            "SKU-001".to_string(),
            2,
            10.0,
        )];

        let result = OrderAggregate::create(customer_id, items);
        assert!(result.is_ok());

        let (aggregate, event) = result.unwrap();
        assert_eq!(aggregate.customer_id, customer_id);
        assert_eq!(aggregate.status, OrderStatus::Created);
        assert_eq!(aggregate.total_amount, 20.0);
        assert_eq!(event.total_amount, 20.0);
    }

    #[test]
    fn test_create_order_no_items() {
        let customer_id = Uuid::new_v4();
        let items = vec![];

        let result = OrderAggregate::create(customer_id, items);
        assert!(matches!(result, Err(OrderError::NoItems)));
    }

    #[test]
    fn test_create_order_invalid_quantity() {
        let customer_id = Uuid::new_v4();
        let items = vec![OrderItem::new(
            Uuid::new_v4(),
            "SKU-001".to_string(),
            0,
            10.0,
        )];

        let result = OrderAggregate::create(customer_id, items);
        assert!(matches!(result, Err(OrderError::InvalidQuantity)));
    }

    #[test]
    fn test_create_order_invalid_price() {
        let customer_id = Uuid::new_v4();
        let items = vec![OrderItem::new(
            Uuid::new_v4(),
            "SKU-001".to_string(),
            1,
            -10.0,
        )];

        let result = OrderAggregate::create(customer_id, items);
        assert!(matches!(result, Err(OrderError::InvalidPrice)));
    }

    #[test]
    fn test_confirm_order() {
        let customer_id = Uuid::new_v4();
        let items = vec![OrderItem::new(
            Uuid::new_v4(),
            "SKU-001".to_string(),
            1,
            10.0,
        )];

        let (aggregate, _) = OrderAggregate::create(customer_id, items).unwrap();
        let result = aggregate.confirm();
        assert!(result.is_ok());
    }

    #[test]
    fn test_cancel_created_order() {
        let customer_id = Uuid::new_v4();
        let items = vec![OrderItem::new(
            Uuid::new_v4(),
            "SKU-001".to_string(),
            1,
            10.0,
        )];

        let (aggregate, _) = OrderAggregate::create(customer_id, items).unwrap();
        let result = aggregate.cancel("Customer request".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cannot_cancel_shipped_order() {
        let customer_id = Uuid::new_v4();
        let items = vec![OrderItem::new(
            Uuid::new_v4(),
            "SKU-001".to_string(),
            1,
            10.0,
        )];

        let (mut aggregate, _) = OrderAggregate::create(customer_id, items).unwrap();
        aggregate.status = OrderStatus::Shipped;

        let result = aggregate.cancel("Customer request".to_string());
        assert!(matches!(result, Err(OrderError::CannotCancel)));
    }

    #[test]
    fn test_apply_order_created_event() {
        let mut aggregate = OrderAggregate::new();
        let event = OrderCreatedEvent {
            order_id: Uuid::new_v4(),
            customer_id: Uuid::new_v4(),
            order_number: "ORD-123".to_string(),
            items: vec![],
            total_amount: 100.0,
            currency: "USD".to_string(),
            created_at: Utc::now(),
        };

        aggregate.apply_order_created(&event);
        assert_eq!(aggregate.id, event.order_id);
        assert_eq!(aggregate.customer_id, event.customer_id);
        assert_eq!(aggregate.status, OrderStatus::Created);
        assert_eq!(aggregate.version, 1);
    }

    #[test]
    fn test_apply_order_confirmed_event() {
        let mut aggregate = OrderAggregate::new();
        aggregate.version = 1;

        let event = OrderConfirmedEvent {
            order_id: Uuid::new_v4(),
            confirmed_at: Utc::now(),
        };

        aggregate.apply_order_confirmed(&event);
        assert_eq!(aggregate.status, OrderStatus::Confirmed);
        assert_eq!(aggregate.version, 2);
    }

    #[test]
    fn test_ship_confirmed_order() {
        let customer_id = Uuid::new_v4();
        let items = vec![OrderItem::new(
            Uuid::new_v4(),
            "SKU-001".to_string(),
            1,
            10.0,
        )];

        let (mut aggregate, _) = OrderAggregate::create(customer_id, items).unwrap();
        aggregate.status = OrderStatus::Confirmed;

        let result = aggregate.ship("TRACK-123".to_string(), "FedEx".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_deliver_shipped_order() {
        let customer_id = Uuid::new_v4();
        let items = vec![OrderItem::new(
            Uuid::new_v4(),
            "SKU-001".to_string(),
            1,
            10.0,
        )];

        let (mut aggregate, _) = OrderAggregate::create(customer_id, items).unwrap();
        aggregate.status = OrderStatus::Shipped;

        let result = aggregate.deliver();
        assert!(result.is_ok());
    }
}
