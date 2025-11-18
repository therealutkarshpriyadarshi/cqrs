use std::sync::Arc;
use tracing::info;
use common::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use common::config::Config;
use common::telemetry::{TelemetryConfig, init_telemetry, shutdown_telemetry};
use messaging::producer::EventPublisher;
use saga::coordinator::SagaCoordinator;
use saga::repository::PostgresSagaRepository;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

mod event_consumer;
mod sagas;

use event_consumer::SagaEventConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    dotenv::dotenv().ok();

    // Initialize telemetry with Jaeger support
    let enable_jaeger = std::env::var("ENABLE_JAEGER")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let telemetry_config = TelemetryConfig {
        service_name: "saga-orchestrator".to_string(),
        log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        jaeger_endpoint: std::env::var("JAEGER_ENDPOINT").ok(),
        enable_jaeger,
    };

    init_telemetry(telemetry_config)?;

    info!("Starting Saga Orchestrator Service with Phase 5 features...");
    info!("Distributed tracing: {}", if enable_jaeger { "enabled" } else { "disabled" });

    let config = Config::from_env()?;

    // Create database connection pool
    info!("Connecting to PostgreSQL at {}", config.database_url);
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    info!("Database connection established");

    // Create saga repository
    let saga_repository = Arc::new(PostgresSagaRepository::new(pool));

    // Create saga coordinator
    let coordinator = Arc::new(SagaCoordinator::new(saga_repository));

    // Create Kafka event publisher
    info!("Connecting to Kafka at {}", config.kafka_brokers);
    let event_publisher = Arc::new(EventPublisher::new(
        &config.kafka_brokers,
        "order-events".to_string(),
    )?);

    info!("Kafka connection established");

    // Create and start event consumer
    let consumer = Arc::new(SagaEventConsumer::new(
        &config.kafka_brokers,
        "saga-orchestrator-group",
        coordinator.clone(),
        event_publisher,
    )?);

    info!("Saga Orchestrator Service started successfully");
    info!("Listening for events on topic: order-events");

    // Start consuming events
    consumer.start().await;

    // Shutdown telemetry gracefully
    shutdown_telemetry();

    info!("Saga Orchestrator Service stopped");

    Ok(())
}
