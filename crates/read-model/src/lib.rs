pub mod cache;
pub mod projections;
pub mod repositories;

pub use cache::RedisCache;
pub use projections::OrderProjection;
pub use repositories::{OrderView, OrderViewRepository, PostgresOrderViewRepository};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadModelError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Order not found: {0}")]
    NotFound(uuid::Uuid),
}
