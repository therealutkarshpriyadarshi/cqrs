use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Command to create a new order
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateOrderCommand {
    pub customer_id: Uuid,

    #[validate(length(min = 1, message = "Order must have at least one item"))]
    pub items: Vec<CreateOrderItem>,

    #[validate(nested)]
    pub shipping_address: ShippingAddress,
}

/// Order item in the create order command
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateOrderItem {
    pub product_id: Uuid,

    #[validate(length(min = 1, message = "SKU cannot be empty"))]
    pub sku: String,

    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    pub quantity: u32,

    #[validate(range(min = 0.01, message = "Unit price must be greater than 0"))]
    pub unit_price: f64,
}

/// Shipping address for the order
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ShippingAddress {
    #[validate(length(min = 1, message = "Street cannot be empty"))]
    pub street: String,

    #[validate(length(min = 1, message = "City cannot be empty"))]
    pub city: String,

    #[validate(length(min = 1, message = "State cannot be empty"))]
    pub state: String,

    #[validate(length(min = 1, message = "ZIP code cannot be empty"))]
    pub zip: String,

    #[validate(length(min = 2, max = 2, message = "Country code must be 2 characters"))]
    pub country: String,
}

/// Command to confirm an order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmOrderCommand {
    pub order_id: Uuid,
}

/// Command to cancel an order
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CancelOrderCommand {
    pub order_id: Uuid,

    #[validate(length(min = 1, message = "Cancellation reason cannot be empty"))]
    pub reason: String,
}

/// Command to ship an order
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ShipOrderCommand {
    pub order_id: Uuid,

    #[validate(length(min = 1, message = "Tracking number cannot be empty"))]
    pub tracking_number: String,

    #[validate(length(min = 1, message = "Carrier cannot be empty"))]
    pub carrier: String,
}

/// Command to mark an order as delivered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverOrderCommand {
    pub order_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_create_order_command_empty_items_fails() {
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

    #[test]
    fn test_create_order_item_zero_quantity_fails() {
        let item = CreateOrderItem {
            product_id: Uuid::new_v4(),
            sku: "SKU-001".to_string(),
            quantity: 0,
            unit_price: 10.50,
        };

        assert!(item.validate().is_err());
    }

    #[test]
    fn test_create_order_item_zero_price_fails() {
        let item = CreateOrderItem {
            product_id: Uuid::new_v4(),
            sku: "SKU-001".to_string(),
            quantity: 2,
            unit_price: 0.0,
        };

        assert!(item.validate().is_err());
    }

    #[test]
    fn test_shipping_address_invalid_country_code_fails() {
        let address = ShippingAddress {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            state: "IL".to_string(),
            zip: "62701".to_string(),
            country: "USA".to_string(), // Should be 2 characters
        };

        assert!(address.validate().is_err());
    }

    #[test]
    fn test_cancel_order_command_empty_reason_fails() {
        let cmd = CancelOrderCommand {
            order_id: Uuid::new_v4(),
            reason: "".to_string(),
        };

        assert!(cmd.validate().is_err());
    }

    #[test]
    fn test_ship_order_command_validation() {
        let cmd = ShipOrderCommand {
            order_id: Uuid::new_v4(),
            tracking_number: "1Z999AA10123456784".to_string(),
            carrier: "UPS".to_string(),
        };

        assert!(cmd.validate().is_ok());
    }
}

