pub mod postgres_event_store;

pub use postgres_event_store::PostgresEventStore;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: Uuid,
    pub aggregate_id: Uuid,
    pub aggregate_type: String,
    pub event_type: String,
    pub event_version: i32,
    pub payload: serde_json::Value,
    pub metadata: serde_json::Value,
    pub sequence_number: i64,
    pub created_at: DateTime<Utc>,
}

impl Event {
    pub fn new(
        aggregate_id: Uuid,
        aggregate_type: String,
        event_type: String,
        event_version: i32,
        payload: serde_json::Value,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            aggregate_id,
            aggregate_type,
            event_type,
            event_version,
            payload,
            metadata,
            sequence_number: 0,
            created_at: Utc::now(),
        }
    }
}

/// Event store trait for persisting and retrieving events
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append events to the store with optimistic concurrency control
    async fn append_events(
        &self,
        aggregate_id: Uuid,
        expected_version: i64,
        events: Vec<Event>,
    ) -> Result<(), EventStoreError>;

    /// Load all events for an aggregate
    async fn load_events(&self, aggregate_id: Uuid) -> Result<Vec<Event>, EventStoreError>;

    /// Load events from a specific version
    async fn load_events_from_version(
        &self,
        aggregate_id: Uuid,
        from_version: i64,
    ) -> Result<Vec<Event>, EventStoreError>;

    /// Get the current version of an aggregate
    async fn get_current_version(&self, aggregate_id: Uuid) -> Result<i64, EventStoreError>;
}

#[derive(Debug, Error)]
pub enum EventStoreError {
    #[error("Concurrency conflict: expected version {expected}, got {actual}")]
    ConcurrencyConflict { expected: i64, actual: i64 },

    #[error("Aggregate not found: {0}")]
    AggregateNotFound(Uuid),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid version: {0}")]
    InvalidVersion(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let aggregate_id = Uuid::new_v4();
        let event = Event::new(
            aggregate_id,
            "Order".to_string(),
            "OrderCreated".to_string(),
            1,
            serde_json::json!({"test": "data"}),
            serde_json::json!({}),
        );

        assert_eq!(event.aggregate_id, aggregate_id);
        assert_eq!(event.event_type, "OrderCreated");
        assert_eq!(event.event_version, 1);
    }
}
