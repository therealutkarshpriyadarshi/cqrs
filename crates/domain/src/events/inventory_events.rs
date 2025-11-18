use super::DomainEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Event emitted when inventory is reserved for an order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReservedEvent {
    pub reservation_id: Uuid,
    pub order_id: Uuid,
    pub items: Vec<InventoryItem>,
    pub reserved_at: DateTime<Utc>,
}

impl DomainEvent for InventoryReservedEvent {
    fn event_type() -> &'static str {
        "InventoryReserved"
    }
}

/// Event emitted when reserved inventory is released (compensation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReleasedEvent {
    pub reservation_id: Uuid,
    pub order_id: Uuid,
    pub items: Vec<InventoryItem>,
    pub released_at: DateTime<Utc>,
    pub reason: String,
}

impl DomainEvent for InventoryReleasedEvent {
    fn event_type() -> &'static str {
        "InventoryReleased"
    }
}

/// Event emitted when inventory reservation fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReservationFailedEvent {
    pub order_id: Uuid,
    pub items: Vec<InventoryItem>,
    pub reason: String,
    pub failed_at: DateTime<Utc>,
}

impl DomainEvent for InventoryReservationFailedEvent {
    fn event_type() -> &'static str {
        "InventoryReservationFailed"
    }
}

/// Event emitted when stock is replenished
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockReplenishedEvent {
    pub product_id: Uuid,
    pub sku: String,
    pub quantity: u32,
    pub replenished_at: DateTime<Utc>,
}

impl DomainEvent for StockReplenishedEvent {
    fn event_type() -> &'static str {
        "StockReplenished"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub product_id: Uuid,
    pub sku: String,
    pub quantity: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_reserved_event() {
        let event = InventoryReservedEvent {
            reservation_id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            items: vec![InventoryItem {
                product_id: Uuid::new_v4(),
                sku: "SKU-001".to_string(),
                quantity: 2,
            }],
            reserved_at: Utc::now(),
        };

        assert_eq!(InventoryReservedEvent::event_type(), "InventoryReserved");
        assert_eq!(event.items.len(), 1);
    }

    #[test]
    fn test_inventory_released_event() {
        let event = InventoryReleasedEvent {
            reservation_id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            items: vec![],
            released_at: Utc::now(),
            reason: "Order cancelled".to_string(),
        };

        assert_eq!(InventoryReleasedEvent::event_type(), "InventoryReleased");
        assert_eq!(event.reason, "Order cancelled");
    }
}
