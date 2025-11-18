use anyhow::Result;
use read_model::{OrderViewRepository, PostgresOrderViewRepository, RedisCache};
use sqlx::PgPool;
use std::sync::Arc;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub repository: Arc<dyn OrderViewRepository>,
    pub cache: Arc<RedisCache>,
}

impl AppState {
    pub async fn new(database_url: &str, redis_url: &str, cache_ttl: usize) -> Result<Self> {
        tracing::info!("Initializing application state...");

        // Connect to database
        tracing::info!("Connecting to database...");
        let pool = PgPool::connect(database_url).await?;
        tracing::info!("Database connected");

        // Create repository
        let repository = Arc::new(PostgresOrderViewRepository::new(pool)) as Arc<dyn OrderViewRepository>;

        // Connect to Redis
        tracing::info!("Connecting to Redis...");
        let cache = Arc::new(RedisCache::new(redis_url, cache_ttl).await?);
        tracing::info!("Redis connected");

        Ok(Self { repository, cache })
    }
}
