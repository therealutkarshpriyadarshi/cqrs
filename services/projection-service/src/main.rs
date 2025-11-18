use anyhow::Result;
use domain::events::order_events::*;
use messaging::EventConsumer;
use read_model::OrderProjection;
use serde_json::Value;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod event_processor;
use event_processor::EventProcessor;

#[derive(serde::Deserialize)]
struct EventEnvelope {
    event_type: String,
    payload: Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "projection_service=info,read_model=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Projection Service...");

    // Configuration from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/cqrs_events".to_string());
    let kafka_brokers = std::env::var("KAFKA_BROKERS")
        .unwrap_or_else(|_| "localhost:9092".to_string());
    let kafka_topic = std::env::var("KAFKA_TOPIC")
        .unwrap_or_else(|_| "order-events".to_string());
    let consumer_group = std::env::var("CONSUMER_GROUP")
        .unwrap_or_else(|_| "projection-service".to_string());

    info!("Configuration:");
    info!("  Database URL: {}", database_url);
    info!("  Kafka Brokers: {}", kafka_brokers);
    info!("  Kafka Topic: {}", kafka_topic);
    info!("  Consumer Group: {}", consumer_group);

    // Connect to database
    info!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;
    info!("Database connected successfully");

    // Create projection
    let projection = OrderProjection::new(pool.clone());

    // Create event processor
    let processor = Arc::new(Mutex::new(EventProcessor::new(projection)));

    // Create Kafka consumer
    info!("Creating Kafka consumer...");
    let consumer = EventConsumer::new(&kafka_brokers, &consumer_group, &[&kafka_topic])?;
    info!("Kafka consumer created successfully");

    // Setup signal handling
    let signals = Signals::new(&[SIGTERM, SIGINT])?;
    let handle = signals.handle();

    let processor_clone = processor.clone();
    let signal_task = tokio::spawn(async move {
        use futures_util::stream::StreamExt;
        let mut signals = signals;
        while let Some(signal) = signals.next().await {
            match signal {
                SIGTERM | SIGINT => {
                    info!("Received shutdown signal, stopping...");
                    break;
                }
                _ => {}
            }
        }
    });

    // Start consuming events
    info!("Starting event consumption loop...");
    let mut running = true;

    while running {
        if signal_task.is_finished() {
            info!("Shutdown signal received, exiting...");
            running = false;
            break;
        }

        match consumer.poll(Duration::from_millis(100)).await {
            Ok(Some(payload)) => {
                // Deserialize event envelope
                match serde_json::from_slice::<EventEnvelope>(&payload) {
                    Ok(envelope) => {
                        let processor = processor.lock().await;
                        if let Err(e) = processor.process_event(&envelope.event_type, envelope.payload).await {
                            error!("Failed to process event: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to deserialize event envelope: {}", e);
                    }
                }
            }
            Ok(None) => {
                // No message, continue polling
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => {
                error!("Error polling Kafka: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    // Cleanup
    info!("Shutting down projection service...");
    handle.close();
    pool.close().await;
    info!("Projection service stopped");

    Ok(())
}
