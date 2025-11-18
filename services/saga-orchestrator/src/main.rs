use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use common::config::Config;
use messaging::producer::EventPublisher;
use saga::coordinator::SagaCoordinator;
use saga::repository::PostgresSagaRepository;
use sqlx::postgres::PgPoolOptions;

mod event_consumer;
mod sagas;

use event_consumer::SagaEventConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Saga Orchestrator Service...");

    // Load configuration
    dotenv::dotenv().ok();
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

    Ok(())
}
