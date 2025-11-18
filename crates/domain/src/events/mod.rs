pub mod order_events;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Base event envelope wrapping all domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub aggregate_type: String,
    pub event_type: String,
    pub event_version: i32,
    pub payload: serde_json::Value,
    pub metadata: EventMetadata,
    pub timestamp: DateTime<Utc>,
    pub sequence_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub correlation_id: Uuid,
    pub causation_id: Uuid,
    pub user_id: Option<Uuid>,
}

impl EventMetadata {
    /// Create new metadata with generated correlation and causation IDs
    pub fn new() -> Self {
        let id = Uuid::new_v4();
        Self {
            correlation_id: id,
            causation_id: id,
            user_id: None,
        }
    }

    /// Create metadata with specific correlation ID
    pub fn with_correlation(correlation_id: Uuid) -> Self {
        Self {
            correlation_id,
            causation_id: Uuid::new_v4(),
            user_id: None,
        }
    }

    /// Add user ID to metadata
    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for all domain events
pub trait DomainEvent: Serialize + for<'de> Deserialize<'de> {
    /// Get the event type name
    fn event_type() -> &'static str;

    /// Get the event version
    fn event_version() -> i32 {
        1
    }

    /// Convert event to envelope
    fn to_envelope(
        &self,
        aggregate_id: Uuid,
        aggregate_type: &str,
        metadata: EventMetadata,
    ) -> Result<EventEnvelope, serde_json::Error> {
        Ok(EventEnvelope {
            event_id: Uuid::new_v4(),
            aggregate_id,
            aggregate_type: aggregate_type.to_string(),
            event_type: Self::event_type().to_string(),
            event_version: Self::event_version(),
            payload: serde_json::to_value(self)?,
            metadata,
            timestamp: Utc::now(),
            sequence_number: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_metadata_new() {
        let metadata = EventMetadata::new();
        assert_eq!(metadata.correlation_id, metadata.causation_id);
        assert!(metadata.user_id.is_none());
    }

    #[test]
    fn test_event_metadata_with_correlation() {
        let correlation_id = Uuid::new_v4();
        let metadata = EventMetadata::with_correlation(correlation_id);
        assert_eq!(metadata.correlation_id, correlation_id);
        assert_ne!(metadata.correlation_id, metadata.causation_id);
    }

    #[test]
    fn test_event_metadata_with_user() {
        let user_id = Uuid::new_v4();
        let metadata = EventMetadata::new().with_user(user_id);
        assert_eq!(metadata.user_id, Some(user_id));
    }
}
