use crate::{Event, EventStore, EventStoreError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Event replay configuration
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Start time for event replay (None = from beginning)
    pub from_timestamp: Option<DateTime<Utc>>,
    /// End time for event replay (None = to current)
    pub to_timestamp: Option<DateTime<Utc>>,
    /// Specific aggregate IDs to replay (None = all aggregates)
    pub aggregate_ids: Option<Vec<Uuid>>,
    /// Specific event types to replay (None = all types)
    pub event_types: Option<Vec<String>>,
    /// Batch size for processing events
    pub batch_size: usize,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            from_timestamp: None,
            to_timestamp: None,
            aggregate_ids: None,
            event_types: None,
            batch_size: 100,
        }
    }
}

/// Statistics for event replay
#[derive(Debug, Clone, Default)]
pub struct ReplayStats {
    pub total_events: usize,
    pub processed_events: usize,
    pub failed_events: usize,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

impl ReplayStats {
    pub fn duration_seconds(&self) -> Option<f64> {
        if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            Some((end - start).num_milliseconds() as f64 / 1000.0)
        } else {
            None
        }
    }
}

/// Event replay service for rebuilding projections or state
pub struct EventReplayService<E: EventStore> {
    event_store: Arc<E>,
    stats: Arc<RwLock<ReplayStats>>,
}

impl<E: EventStore> EventReplayService<E> {
    pub fn new(event_store: Arc<E>) -> Self {
        Self {
            event_store,
            stats: Arc::new(RwLock::new(ReplayStats::default())),
        }
    }

    /// Replay events with a custom event handler
    pub async fn replay_events<F, Fut>(
        &self,
        config: ReplayConfig,
        mut handler: F,
    ) -> Result<ReplayStats, EventStoreError>
    where
        F: FnMut(Event) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        info!("Starting event replay with config: {:?}", config);

        let mut stats = self.stats.write().await;
        stats.start_time = Some(Utc::now());
        stats.processed_events = 0;
        stats.failed_events = 0;
        drop(stats);

        // Get all events based on config
        let events = self.fetch_events(&config).await?;

        let total_events = events.len();
        info!("Found {} events to replay", total_events);

        {
            let mut stats = self.stats.write().await;
            stats.total_events = total_events;
        }

        // Process events in batches
        for chunk in events.chunks(config.batch_size) {
            for event in chunk {
                match handler(event.clone()).await {
                    Ok(_) => {
                        let mut stats = self.stats.write().await;
                        stats.processed_events += 1;
                    }
                    Err(e) => {
                        warn!(
                            event_id = %event.event_id,
                            event_type = %event.event_type,
                            error = %e,
                            "Failed to process event during replay"
                        );
                        let mut stats = self.stats.write().await;
                        stats.failed_events += 1;
                    }
                }
            }
        }

        let mut stats = self.stats.write().await;
        stats.end_time = Some(Utc::now());

        info!(
            processed = stats.processed_events,
            failed = stats.failed_events,
            duration_secs = stats.duration_seconds().unwrap_or(0.0),
            "Event replay completed"
        );

        Ok(stats.clone())
    }

    /// Replay events for a specific aggregate
    pub async fn replay_aggregate<F, Fut>(
        &self,
        aggregate_id: Uuid,
        handler: F,
    ) -> Result<ReplayStats, EventStoreError>
    where
        F: FnMut(Event) -> Fut,
        Fut: std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    {
        let config = ReplayConfig {
            aggregate_ids: Some(vec![aggregate_id]),
            ..Default::default()
        };

        self.replay_events(config, handler).await
    }

    /// Get current replay statistics
    pub async fn get_stats(&self) -> ReplayStats {
        self.stats.read().await.clone()
    }

    /// Fetch events based on replay configuration
    async fn fetch_events(&self, config: &ReplayConfig) -> Result<Vec<Event>, EventStoreError> {
        // If specific aggregate IDs are provided, fetch their events
        if let Some(aggregate_ids) = &config.aggregate_ids {
            let mut all_events = Vec::new();
            for aggregate_id in aggregate_ids {
                let events = self.event_store.load_events(*aggregate_id).await?;
                all_events.extend(events);
            }

            // Filter by timestamp and event type
            Ok(self.filter_events(all_events, config))
        } else {
            // For all aggregates, we'd need a method to fetch all events
            // This is a simplified implementation
            // In a real system, you'd query the database directly
            warn!("Replaying all aggregates requires direct database access");
            Ok(vec![])
        }
    }

    /// Filter events based on configuration
    fn filter_events(&self, events: Vec<Event>, config: &ReplayConfig) -> Vec<Event> {
        events
            .into_iter()
            .filter(|event| {
                // Filter by timestamp
                if let Some(from) = config.from_timestamp {
                    if event.created_at < from {
                        return false;
                    }
                }
                if let Some(to) = config.to_timestamp {
                    if event.created_at > to {
                        return false;
                    }
                }

                // Filter by event type
                if let Some(event_types) = &config.event_types {
                    if !event_types.contains(&event.event_type) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }
}

/// Trait for projections that can be rebuilt from events
#[async_trait]
pub trait Rebuildable: Send + Sync {
    /// Clear all projection data
    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Process a single event
    async fn process_event(&self, event: Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Rebuild the projection from events
    async fn rebuild<E: EventStore>(
        &self,
        replay_service: &EventReplayService<E>,
        config: ReplayConfig,
    ) -> Result<ReplayStats, Box<dyn std::error::Error + Send + Sync>> {
        // Clear existing data
        self.clear().await?;

        // Replay events
        let stats = replay_service
            .replay_events(config, |event| self.process_event(event))
            .await?;

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_config_default() {
        let config = ReplayConfig::default();
        assert!(config.from_timestamp.is_none());
        assert!(config.to_timestamp.is_none());
        assert!(config.aggregate_ids.is_none());
        assert!(config.event_types.is_none());
        assert_eq!(config.batch_size, 100);
    }

    #[test]
    fn test_replay_stats_duration() {
        let mut stats = ReplayStats::default();
        assert!(stats.duration_seconds().is_none());

        stats.start_time = Some(Utc::now());
        stats.end_time = Some(Utc::now());
        assert!(stats.duration_seconds().is_some());
    }
}
