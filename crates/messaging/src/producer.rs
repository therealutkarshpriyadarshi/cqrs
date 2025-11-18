use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use serde::Serialize;
use std::time::Duration;
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PublisherError {
    #[error("Failed to create Kafka producer: {0}")]
    ProducerCreation(String),

    #[error("Failed to serialize event: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Failed to publish event: {0}")]
    PublishFailed(String),
}

/// Kafka event publisher for publishing domain events
pub struct EventPublisher {
    producer: FutureProducer,
    topic: String,
}

impl EventPublisher {
    /// Create a new EventPublisher
    ///
    /// # Arguments
    /// * `brokers` - Comma-separated list of Kafka brokers (e.g., "localhost:9092")
    /// * `topic` - The topic to publish events to
    ///
    /// # Example
    /// ```no_run
    /// use messaging::EventPublisher;
    ///
    /// let publisher = EventPublisher::new("localhost:9092", "order-events".to_string())
    ///     .expect("Failed to create publisher");
    /// ```
    pub fn new(brokers: &str, topic: String) -> Result<Self, PublisherError> {
        info!("Creating Kafka producer for brokers: {}", brokers);

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("compression.type", "snappy")
            .set("acks", "all") // Wait for all replicas to acknowledge
            .set("retries", "3") // Retry failed sends
            .create()
            .map_err(|e| PublisherError::ProducerCreation(e.to_string()))?;

        info!("Kafka producer created successfully for topic: {}", topic);

        Ok(Self { producer, topic })
    }

    /// Publish an event to Kafka
    ///
    /// # Arguments
    /// * `key` - The partition key (usually aggregate ID)
    /// * `event` - The event to publish (must be serializable)
    ///
    /// # Example
    /// ```no_run
    /// use messaging::EventPublisher;
    /// use uuid::Uuid;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct OrderCreated {
    ///     order_id: Uuid,
    /// }
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let publisher = EventPublisher::new("localhost:9092", "order-events".to_string())?;
    /// let order_id = Uuid::new_v4();
    /// let event = OrderCreated { order_id };
    ///
    /// publisher.publish(order_id, &event).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn publish<T: Serialize>(
        &self,
        key: Uuid,
        event: &T,
    ) -> Result<(), PublisherError> {
        let payload = serde_json::to_string(event)?;
        let key_str = key.to_string();

        let record = FutureRecord::to(&self.topic)
            .key(&key_str)
            .payload(&payload);

        match self
            .producer
            .send(record, Timeout::After(Duration::from_secs(5)))
            .await
        {
            Ok((partition, offset)) => {
                info!(
                    "Event published successfully to topic '{}', partition {}, offset {}",
                    self.topic, partition, offset
                );
                Ok(())
            }
            Err((err, _)) => {
                warn!("Failed to publish event: {}", err);
                Err(PublisherError::PublishFailed(err.to_string()))
            }
        }
    }

    /// Publish multiple events in batch
    ///
    /// # Arguments
    /// * `events` - Vector of (key, event) tuples
    pub async fn publish_batch<T: Serialize>(
        &self,
        events: Vec<(Uuid, T)>,
    ) -> Result<(), PublisherError> {
        for (key, event) in events {
            self.publish(key, &event).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestEvent {
        message: String,
    }

    #[test]
    fn test_event_publisher_creation_with_invalid_brokers() {
        // This should succeed (creation doesn't validate connection)
        let result = EventPublisher::new("", "test-topic".to_string());
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_serialize_event() {
        let event = TestEvent {
            message: "test".to_string(),
        };
        let serialized = serde_json::to_string(&event).unwrap();
        assert!(serialized.contains("test"));
    }
}
