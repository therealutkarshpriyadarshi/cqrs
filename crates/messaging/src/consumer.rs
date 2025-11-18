use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use serde::de::DeserializeOwned;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Debug, Error)]
pub enum ConsumerError {
    #[error("Failed to create Kafka consumer: {0}")]
    ConsumerCreation(#[from] rdkafka::error::KafkaError),

    #[error("Failed to deserialize message: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("Message has no payload")]
    NoPayload,
}

/// Kafka event consumer for consuming events from a topic
pub struct EventConsumer {
    consumer: BaseConsumer,
}

impl EventConsumer {
    /// Create a new Kafka consumer
    pub fn new(
        brokers: &str,
        group_id: &str,
        topics: &[&str],
    ) -> Result<Self, ConsumerError> {
        info!(
            "Creating Kafka consumer with group_id: {}, topics: {:?}",
            group_id, topics
        );

        let consumer: BaseConsumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", brokers)
            .set("enable.auto.commit", "true")
            .set("auto.commit.interval.ms", "5000")
            .set("auto.offset.reset", "earliest")
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "30000")
            .set("heartbeat.interval.ms", "10000")
            .create()?;

        consumer.subscribe(topics)?;

        info!("Kafka consumer created successfully");
        Ok(Self { consumer })
    }

    /// Poll for a message with a timeout
    pub async fn poll(&self, timeout: Duration) -> Result<Option<Vec<u8>>, ConsumerError> {
        // Convert timeout to Option<Duration> for poll
        let poll_timeout = if timeout.as_millis() > 0 {
            Some(timeout)
        } else {
            Some(Duration::from_millis(100))
        };

        match self.consumer.poll(poll_timeout) {
            Some(Ok(message)) => {
                debug!(
                    "Received message from topic: {}, partition: {}, offset: {}",
                    message.topic(),
                    message.partition(),
                    message.offset()
                );

                match message.payload() {
                    Some(payload) => Ok(Some(payload.to_vec())),
                    None => {
                        warn!("Message has no payload");
                        Err(ConsumerError::NoPayload)
                    }
                }
            }
            Some(Err(e)) => {
                error!("Kafka error while polling: {}", e);
                Err(ConsumerError::ConsumerCreation(e))
            }
            None => {
                // Timeout reached with no message
                debug!("No message received within timeout");
                Ok(None)
            }
        }
    }

    /// Poll and deserialize message
    pub async fn poll_message<T: DeserializeOwned>(
        &self,
        timeout: Duration,
    ) -> Result<Option<T>, ConsumerError> {
        match self.poll(timeout).await? {
            Some(payload) => {
                let message = serde_json::from_slice(&payload)?;
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }

    /// Commit the current offsets
    pub fn commit(&self) -> Result<(), ConsumerError> {
        self.consumer
            .commit_consumer_state(rdkafka::consumer::CommitMode::Sync)?;
        Ok(())
    }

    /// Get the underlying BaseConsumer for advanced usage
    pub fn inner(&self) -> &BaseConsumer {
        &self.consumer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consumer_creation_invalid_broker() {
        let result = EventConsumer::new("invalid:9092", "test-group", &["test-topic"]);
        // Should succeed in creation (connection happens on poll)
        assert!(result.is_ok());
    }
}
