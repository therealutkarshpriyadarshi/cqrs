use super::DomainEvent;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Event emitted when payment is authorized (but not captured)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAuthorizedEvent {
    pub payment_id: Uuid,
    pub order_id: Uuid,
    pub amount: f64,
    pub currency: String,
    pub payment_method: String,
    pub authorization_code: String,
    pub authorized_at: DateTime<Utc>,
}

impl DomainEvent for PaymentAuthorizedEvent {
    fn event_type() -> &'static str {
        "PaymentAuthorized"
    }
}

/// Event emitted when authorized payment is captured
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentCapturedEvent {
    pub payment_id: Uuid,
    pub order_id: Uuid,
    pub amount: f64,
    pub currency: String,
    pub transaction_id: String,
    pub captured_at: DateTime<Utc>,
}

impl DomainEvent for PaymentCapturedEvent {
    fn event_type() -> &'static str {
        "PaymentCaptured"
    }
}

/// Event emitted when payment authorization is voided (compensation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentVoidedEvent {
    pub payment_id: Uuid,
    pub order_id: Uuid,
    pub amount: f64,
    pub currency: String,
    pub reason: String,
    pub voided_at: DateTime<Utc>,
}

impl DomainEvent for PaymentVoidedEvent {
    fn event_type() -> &'static str {
        "PaymentVoided"
    }
}

/// Event emitted when payment fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentFailedEvent {
    pub payment_id: Uuid,
    pub order_id: Uuid,
    pub amount: f64,
    pub currency: String,
    pub reason: String,
    pub failed_at: DateTime<Utc>,
}

impl DomainEvent for PaymentFailedEvent {
    fn event_type() -> &'static str {
        "PaymentFailed"
    }
}

/// Event emitted when payment is refunded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRefundedEvent {
    pub payment_id: Uuid,
    pub order_id: Uuid,
    pub amount: f64,
    pub currency: String,
    pub refund_id: String,
    pub reason: String,
    pub refunded_at: DateTime<Utc>,
}

impl DomainEvent for PaymentRefundedEvent {
    fn event_type() -> &'static str {
        "PaymentRefunded"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_authorized_event() {
        let event = PaymentAuthorizedEvent {
            payment_id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            amount: 99.99,
            currency: "USD".to_string(),
            payment_method: "credit_card".to_string(),
            authorization_code: "AUTH123".to_string(),
            authorized_at: Utc::now(),
        };

        assert_eq!(PaymentAuthorizedEvent::event_type(), "PaymentAuthorized");
        assert_eq!(event.amount, 99.99);
    }

    #[test]
    fn test_payment_voided_event() {
        let event = PaymentVoidedEvent {
            payment_id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            amount: 99.99,
            currency: "USD".to_string(),
            reason: "Order cancelled".to_string(),
            voided_at: Utc::now(),
        };

        assert_eq!(PaymentVoidedEvent::event_type(), "PaymentVoided");
        assert_eq!(event.reason, "Order cancelled");
    }

    #[test]
    fn test_payment_failed_event() {
        let event = PaymentFailedEvent {
            payment_id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            amount: 99.99,
            currency: "USD".to_string(),
            reason: "Insufficient funds".to_string(),
            failed_at: Utc::now(),
        };

        assert_eq!(PaymentFailedEvent::event_type(), "PaymentFailed");
        assert_eq!(event.reason, "Insufficient funds");
    }
}
