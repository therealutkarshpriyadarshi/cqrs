use redis::{AsyncCommands, Client, RedisError};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Idempotency key for tracking processed commands/events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyKey {
    pub key: String,
    pub result: Option<serde_json::Value>,
}

/// Redis-based idempotency checker
pub struct IdempotencyChecker {
    client: Client,
    ttl_seconds: u64,
}

impl IdempotencyChecker {
    /// Create a new idempotency checker
    pub fn new(redis_url: &str, ttl_seconds: u64) -> Result<Self, RedisError> {
        let client = Client::open(redis_url)?;
        Ok(Self {
            client,
            ttl_seconds,
        })
    }

    /// Check if a command/event has already been processed
    /// Returns Some(result) if already processed, None if new
    pub async fn check(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<serde_json::Value>, RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = self.format_key(idempotency_key);

        let result: Option<String> = conn.get(&key).await?;

        match result {
            Some(data) => {
                let value: serde_json::Value = serde_json::from_str(&data)
                    .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Invalid JSON", e.to_string())))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Record that a command/event has been processed
    pub async fn record(
        &self,
        idempotency_key: &str,
        result: &serde_json::Value,
    ) -> Result<(), RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = self.format_key(idempotency_key);
        let value = serde_json::to_string(result)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Serialization failed", e.to_string())))?;

        conn.set_ex(&key, value, self.ttl_seconds).await?;

        tracing::debug!(
            idempotency_key = %idempotency_key,
            ttl_seconds = %self.ttl_seconds,
            "Recorded idempotency key"
        );

        Ok(())
    }

    /// Delete an idempotency record (useful for testing)
    pub async fn delete(&self, idempotency_key: &str) -> Result<(), RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = self.format_key(idempotency_key);
        conn.del(&key).await?;
        Ok(())
    }

    /// Check if a key exists
    pub async fn exists(&self, idempotency_key: &str) -> Result<bool, RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = self.format_key(idempotency_key);
        let exists: bool = conn.exists(&key).await?;
        Ok(exists)
    }

    fn format_key(&self, idempotency_key: &str) -> String {
        format!("idempotency:{}", idempotency_key)
    }
}

/// Generate an idempotency key from command ID or event ID
pub fn generate_idempotency_key(id: &Uuid, operation: &str) -> String {
    format!("{}:{}", operation, id)
}

/// Idempotency middleware for command handlers
pub struct IdempotentCommandHandler<H> {
    handler: H,
    checker: IdempotencyChecker,
}

impl<H> IdempotentCommandHandler<H> {
    pub fn new(handler: H, checker: IdempotencyChecker) -> Self {
        Self { handler, checker }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_idempotency_key() {
        let id = Uuid::new_v4();
        let key = generate_idempotency_key(&id, "CreateOrder");
        assert!(key.starts_with("CreateOrder:"));
        assert!(key.contains(&id.to_string()));
    }

    // Note: Integration tests that require Redis would go in tests/integration/
}
