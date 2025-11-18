use anyhow::Result;
use event_store::{EventStore, PostgresEventStore};
use messaging::EventPublisher;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub event_store: Arc<dyn EventStore>,
    pub event_publisher: Arc<EventPublisher>,
}

impl AppState {
    /// Create a new application state
    pub async fn new() -> Result<Self> {
        dotenv::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/cqrs_events".to_string());

        let kafka_brokers = std::env::var("KAFKA_BROKERS")
            .unwrap_or_else(|_| "localhost:9092".to_string());

        let kafka_topic = std::env::var("KAFKA_TOPIC")
            .unwrap_or_else(|_| "order-events".to_string());

        info!("Connecting to database: {}", database_url);
        let pool = PgPool::connect(&database_url).await?;

        info!("Creating event store");
        let event_store = Arc::new(PostgresEventStore::new(pool)) as Arc<dyn EventStore>;

        info!("Creating Kafka event publisher");
        let event_publisher = Arc::new(EventPublisher::new(&kafka_brokers, kafka_topic)?);

        Ok(Self {
            event_store,
            event_publisher,
        })
    }
}
