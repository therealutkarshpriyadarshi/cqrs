use anyhow::Result;
use common::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use event_store::{EventStore, IdempotencyChecker, PostgresEventStore};
use messaging::EventPublisher;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub event_store: Arc<dyn EventStore>,
    pub event_publisher: Arc<EventPublisher>,
    pub idempotency_checker: Option<Arc<IdempotencyChecker>>,
    pub kafka_circuit_breaker: Arc<CircuitBreaker>,
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

        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let enable_idempotency = std::env::var("ENABLE_IDEMPOTENCY")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        info!("Connecting to database: {}", database_url);
        let pool = PgPool::connect(&database_url).await?;

        info!("Creating event store");
        let event_store = Arc::new(PostgresEventStore::new(pool)) as Arc<dyn EventStore>;

        info!("Creating Kafka event publisher");
        let event_publisher = Arc::new(EventPublisher::new(&kafka_brokers, kafka_topic)?);

        // Initialize idempotency checker if enabled
        let idempotency_checker = if enable_idempotency {
            info!("Initializing idempotency checker with Redis");
            match IdempotencyChecker::new(&redis_url, 3600) {
                Ok(checker) => Some(Arc::new(checker)),
                Err(e) => {
                    tracing::warn!("Failed to initialize idempotency checker: {}. Continuing without idempotency.", e);
                    None
                }
            }
        } else {
            info!("Idempotency checking disabled");
            None
        };

        // Initialize circuit breaker for Kafka
        info!("Initializing circuit breaker for Kafka");
        let kafka_circuit_breaker = Arc::new(CircuitBreaker::new(
            "kafka-publisher".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 5,
                success_threshold: 2,
                timeout: Duration::from_secs(5),
                half_open_timeout: Duration::from_secs(30),
            },
        ));

        Ok(Self {
            event_store,
            event_publisher,
            idempotency_checker,
            kafka_circuit_breaker,
        })
    }
}
