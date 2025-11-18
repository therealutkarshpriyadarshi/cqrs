use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderCommand {
    pub customer_id: Uuid,
    pub items: Vec<CreateOrderItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderItem {
    pub product_id: Uuid,
    pub sku: String,
    pub quantity: u32,
    pub unit_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmOrderCommand {
    pub order_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderCommand {
    pub order_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipOrderCommand {
    pub order_id: Uuid,
    pub tracking_number: String,
    pub carrier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverOrderCommand {
    pub order_id: Uuid,
}
