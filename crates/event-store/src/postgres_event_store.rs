use super::{Event, EventStore, EventStoreError};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use tracing::{debug, error, info};
use uuid::Uuid;

/// PostgreSQL implementation of the event store
pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    /// Create a new PostgreSQL event store
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get the database pool (useful for testing)
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl EventStore for PostgresEventStore {
    async fn append_events(
        &self,
        aggregate_id: Uuid,
        expected_version: i64,
        events: Vec<Event>,
    ) -> Result<(), EventStoreError> {
        if events.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // Check current version (optimistic locking)
        let current_version: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(version) FROM events WHERE aggregate_id = $1",
        )
        .bind(aggregate_id)
        .fetch_optional(&mut *tx)
        .await?;

        let current = current_version.unwrap_or(0);

        debug!(
            "Appending events for aggregate {}: expected_version={}, current_version={}",
            aggregate_id, expected_version, current
        );

        if current != expected_version {
            error!(
                "Concurrency conflict for aggregate {}: expected {}, got {}",
                aggregate_id, expected_version, current
            );
            return Err(EventStoreError::ConcurrencyConflict {
                expected: expected_version,
                actual: current,
            });
        }

        // Insert events
        for (i, event) in events.iter().enumerate() {
            let version = expected_version + i as i64 + 1;

            sqlx::query(
                r#"
                INSERT INTO events (
                    event_id, aggregate_id, aggregate_type, event_type,
                    event_version, payload, metadata, version, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(event.event_id)
            .bind(aggregate_id)
            .bind(&event.aggregate_type)
            .bind(&event.event_type)
            .bind(event.event_version)
            .bind(&event.payload)
            .bind(&event.metadata)
            .bind(version)
            .bind(event.created_at)
            .execute(&mut *tx)
            .await?;

            debug!(
                "Inserted event {} for aggregate {} at version {}",
                event.event_id, aggregate_id, version
            );
        }

        tx.commit().await?;

        info!(
            "Successfully appended {} events for aggregate {} at version {}",
            events.len(),
            aggregate_id,
            expected_version + events.len() as i64
        );

        Ok(())
    }

    async fn load_events(&self, aggregate_id: Uuid) -> Result<Vec<Event>, EventStoreError> {
        debug!("Loading events for aggregate {}", aggregate_id);

        let rows = sqlx::query(
            r#"
            SELECT event_id, aggregate_id, aggregate_type, event_type,
                   event_version, payload, metadata, version as sequence_number, created_at
            FROM events
            WHERE aggregate_id = $1
            ORDER BY version ASC
            "#,
        )
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await?;

        let events: Vec<Event> = rows
            .iter()
            .map(|row| Event {
                event_id: row.get("event_id"),
                aggregate_id: row.get("aggregate_id"),
                aggregate_type: row.get("aggregate_type"),
                event_type: row.get("event_type"),
                event_version: row.get("event_version"),
                payload: row.get("payload"),
                metadata: row.get("metadata"),
                sequence_number: row.get("sequence_number"),
                created_at: row.get("created_at"),
            })
            .collect();

        debug!("Loaded {} events for aggregate {}", events.len(), aggregate_id);

        Ok(events)
    }

    async fn load_events_from_version(
        &self,
        aggregate_id: Uuid,
        from_version: i64,
    ) -> Result<Vec<Event>, EventStoreError> {
        debug!(
            "Loading events for aggregate {} from version {}",
            aggregate_id, from_version
        );

        let rows = sqlx::query(
            r#"
            SELECT event_id, aggregate_id, aggregate_type, event_type,
                   event_version, payload, metadata, version as sequence_number, created_at
            FROM events
            WHERE aggregate_id = $1 AND version > $2
            ORDER BY version ASC
            "#,
        )
        .bind(aggregate_id)
        .bind(from_version)
        .fetch_all(&self.pool)
        .await?;

        let events: Vec<Event> = rows
            .iter()
            .map(|row| Event {
                event_id: row.get("event_id"),
                aggregate_id: row.get("aggregate_id"),
                aggregate_type: row.get("aggregate_type"),
                event_type: row.get("event_type"),
                event_version: row.get("event_version"),
                payload: row.get("payload"),
                metadata: row.get("metadata"),
                sequence_number: row.get("sequence_number"),
                created_at: row.get("created_at"),
            })
            .collect();

        debug!(
            "Loaded {} events for aggregate {} from version {}",
            events.len(),
            aggregate_id,
            from_version
        );

        Ok(events)
    }

    async fn get_current_version(&self, aggregate_id: Uuid) -> Result<i64, EventStoreError> {
        let version: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(version) FROM events WHERE aggregate_id = $1",
        )
        .bind(aggregate_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(version.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    // Note: Integration tests are in tests/integration/
    // Run with: cargo test --test event_store_tests -- --ignored

    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_append_and_load_events() {
        // This test would require a test database
        // For now, we'll skip it in regular test runs
    }
}
