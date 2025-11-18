use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisError};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::ReadModelError;

/// Redis cache for order views
pub struct RedisCache {
    conn: ConnectionManager,
    ttl_seconds: usize,
}

impl RedisCache {
    /// Create new Redis cache
    pub async fn new(redis_url: &str, ttl_seconds: usize) -> Result<Self, ReadModelError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| ReadModelError::CacheError(format!("Failed to create Redis client: {}", e)))?;

        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| ReadModelError::CacheError(format!("Failed to connect to Redis: {}", e)))?;

        info!("Redis cache initialized with TTL: {} seconds", ttl_seconds);
        Ok(Self { conn, ttl_seconds })
    }

    /// Get value from cache
    pub async fn get<T: DeserializeOwned>(&self, key: &Uuid) -> Option<T> {
        let cache_key = format!("order:{}", key);

        match self.conn.clone().get::<_, String>(&cache_key).await {
            Ok(value) => {
                debug!("Cache hit for key: {}", cache_key);
                match serde_json::from_str::<T>(&value) {
                    Ok(data) => Some(data),
                    Err(e) => {
                        error!("Failed to deserialize cached value for {}: {}", cache_key, e);
                        None
                    }
                }
            }
            Err(e) => {
                if !matches!(e.kind(), redis::ErrorKind::TypeError) {
                    debug!("Cache miss for key: {}", cache_key);
                } else {
                    warn!("Redis error for key {}: {}", cache_key, e);
                }
                None
            }
        }
    }

    /// Set value in cache
    pub async fn set<T: Serialize>(&self, key: &Uuid, value: &T) {
        let cache_key = format!("order:{}", key);

        match serde_json::to_string(value) {
            Ok(json) => {
                let result: Result<(), RedisError> = self
                    .conn
                    .clone()
                    .set_ex(&cache_key, json, self.ttl_seconds as u64)
                    .await;

                match result {
                    Ok(_) => {
                        debug!("Cached value for key: {} with TTL: {}s", cache_key, self.ttl_seconds);
                    }
                    Err(e) => {
                        error!("Failed to set cache for key {}: {}", cache_key, e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to serialize value for cache key {}: {}", cache_key, e);
            }
        }
    }

    /// Delete value from cache
    pub async fn delete(&self, key: &Uuid) {
        let cache_key = format!("order:{}", key);

        let result: Result<(), RedisError> = self.conn.clone().del(&cache_key).await;

        match result {
            Ok(_) => {
                debug!("Deleted cache for key: {}", cache_key);
            }
            Err(e) => {
                error!("Failed to delete cache for key {}: {}", cache_key, e);
            }
        }
    }

    /// Invalidate cache (delete) for a key
    pub async fn invalidate(&self, key: &Uuid) {
        self.delete(key).await;
    }

    /// Check if cache is available (health check)
    pub async fn ping(&self) -> Result<(), ReadModelError> {
        let result: Result<String, RedisError> = redis::cmd("PING")
            .query_async(&mut self.conn.clone())
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(ReadModelError::CacheError(format!("Redis ping failed: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_cache_operations() {
        let cache = RedisCache::new("redis://localhost:6379", 300)
            .await
            .expect("Failed to connect to Redis");

        let key = Uuid::new_v4();
        let value = serde_json::json!({"test": "data"});

        // Set
        cache.set(&key, &value).await;

        // Get
        let cached: Option<serde_json::Value> = cache.get(&key).await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap(), value);

        // Delete
        cache.delete(&key).await;

        // Verify deleted
        let cached: Option<serde_json::Value> = cache.get(&key).await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_ping() {
        let cache = RedisCache::new("redis://localhost:6379", 300)
            .await
            .expect("Failed to connect to Redis");

        let result = cache.ping().await;
        assert!(result.is_ok());
    }
}
