use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use domain::events::order_events::{OrderCreatedEvent, OrderItem};
use domain::events::EventEnvelope;
use messaging::producer::EventPublisher;
use saga::coordinator::SagaCoordinator;
use saga::repository::PostgresSagaRepository;

use crate::sagas::{OrderProcessingSaga, OrderSagaData};

pub struct SagaEventConsumer {
    consumer: StreamConsumer,
    coordinator: Arc<SagaCoordinator<PostgresSagaRepository>>,
    order_saga: Arc<OrderProcessingSaga>,
}

impl SagaEventConsumer {
    pub fn new(
        brokers: &str,
        group_id: &str,
        coordinator: Arc<SagaCoordinator<PostgresSagaRepository>>,
        event_publisher: Arc<EventPublisher>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", brokers)
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", "earliest")
            .set("session.timeout.ms", "6000")
            .create()?;

        consumer.subscribe(&["order-events"])?;

        let order_saga = Arc::new(OrderProcessingSaga::new(event_publisher));

        Ok(Self {
            consumer,
            coordinator,
            order_saga,
        })
    }

    pub async fn start(self: Arc<Self>) {
        info!("Starting saga event consumer...");

        loop {
            match self.consumer.recv().await {
                Ok(msg) => {
                    if let Some(payload) = msg.payload() {
                        if let Err(e) = self.process_message(payload).await {
                            error!(error = %e, "Error processing message");
                        }
                    }
                }
                Err(e) => {
                    error!(kafka_error = %e, "Kafka consumer error");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn process_message(&self, payload: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let envelope: EventEnvelope = serde_json::from_slice(payload)?;

        info!(
            event_type = %envelope.event_type,
            aggregate_id = %envelope.aggregate_id,
            "Received event"
        );

        match envelope.event_type.as_str() {
            "OrderCreated" => {
                self.handle_order_created(&envelope).await?;
            }
            _ => {
                // Ignore other events
            }
        }

        Ok(())
    }

    async fn handle_order_created(
        &self,
        envelope: &EventEnvelope,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!(
            order_id = %envelope.aggregate_id,
            "Handling OrderCreated event - starting saga"
        );

        // Deserialize the event
        let event: OrderCreatedEvent = serde_json::from_value(envelope.payload.clone())?;

        // Create saga data
        let saga_data = OrderSagaData {
            order_id: event.order_id,
            customer_id: event.customer_id,
            items: event.items.clone(),
            total_amount: event.total_amount,
            currency: event.currency.clone(),
            payment_method: "credit_card".to_string(), // Default for now
            correlation_id: envelope.metadata.correlation_id,
        };

        let saga_id = Uuid::new_v4();
        let saga_data_json = serde_json::to_value(&saga_data)?;

        // Start the saga
        let state = self
            .coordinator
            .start_saga(&*self.order_saga, saga_id, saga_data_json)
            .await?;

        info!(
            saga_id = %saga_id,
            order_id = %event.order_id,
            "Saga started successfully"
        );

        // Run the saga to completion
        match self.coordinator.run_saga(&*self.order_saga, state).await {
            Ok(final_state) => {
                info!(
                    saga_id = %saga_id,
                    status = %final_state.status,
                    "Saga execution completed"
                );
            }
            Err(e) => {
                error!(
                    saga_id = %saga_id,
                    error = %e,
                    "Saga execution failed"
                );
                return Err(Box::new(e));
            }
        }

        Ok(())
    }
}
