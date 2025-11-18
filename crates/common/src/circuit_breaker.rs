use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use async_trait::async_trait;
use crate::metrics::{record_circuit_breaker_state, record_circuit_breaker_transition, CircuitBreakerState as MetricsState};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Simple circuit breaker implementation for external service calls
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    last_failure_time: Arc<AtomicU64>,
    state: Arc<RwLock<CircuitBreakerState>>,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            half_open_timeout: Duration::from_secs(30),
        }
    }
}

impl CircuitBreaker {
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        // Initialize metrics
        record_circuit_breaker_state(&name, MetricsState::Closed);

        Self {
            name,
            config,
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(AtomicU64::new(0)),
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
        }
    }

    /// Execute a function with circuit breaker protection
    pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        // Check if we should attempt the call
        if !self.check_state().await {
            return Err(CircuitBreakerError::Open);
        }

        // Execute the function with timeout
        let start = std::time::Instant::now();
        let result = tokio::time::timeout(self.config.timeout, f).await;

        match result {
            Ok(Ok(value)) => {
                self.on_success().await;
                tracing::debug!(
                    service = %self.name,
                    duration_ms = %start.elapsed().as_millis(),
                    "Circuit breaker call succeeded"
                );
                Ok(value)
            }
            Ok(Err(err)) => {
                self.on_failure().await;
                tracing::warn!(
                    service = %self.name,
                    duration_ms = %start.elapsed().as_millis(),
                    "Circuit breaker call failed"
                );
                Err(CircuitBreakerError::CallFailed(err))
            }
            Err(_) => {
                self.on_failure().await;
                tracing::error!(
                    service = %self.name,
                    timeout_secs = %self.config.timeout.as_secs(),
                    "Circuit breaker call timed out"
                );
                Err(CircuitBreakerError::Timeout)
            }
        }
    }

    /// Check current state and transition if needed
    /// Returns true if the call should proceed, false if circuit is open
    async fn check_state(&self) -> bool {
        let mut state = self.state.write().await;

        match *state {
            CircuitBreakerState::Closed => {
                // Normal operation
                true
            }
            CircuitBreakerState::Open => {
                // Check if we should transition to half-open
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);

                if now - last_failure > self.config.half_open_timeout.as_secs() {
                    *state = CircuitBreakerState::HalfOpen;
                    self.success_count.store(0, Ordering::Relaxed);
                    record_circuit_breaker_transition(&self.name, MetricsState::Open, MetricsState::HalfOpen);
                    record_circuit_breaker_state(&self.name, MetricsState::HalfOpen);
                    tracing::info!(service = %self.name, "Circuit breaker transitioned to HalfOpen");
                    true
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Allow limited requests
                true
            }
        }
    }

    /// Handle successful call
    async fn on_success(&self) {
        let mut state = self.state.write().await;

        match *state {
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitBreakerState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if success_count >= self.config.success_threshold {
                    *state = CircuitBreakerState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    record_circuit_breaker_transition(&self.name, MetricsState::HalfOpen, MetricsState::Closed);
                    record_circuit_breaker_state(&self.name, MetricsState::Closed);
                    tracing::info!(service = %self.name, "Circuit breaker transitioned to Closed");
                }
            }
            CircuitBreakerState::Open => {
                // Should not happen, but reset if it does
            }
        }
    }

    /// Handle failed call
    async fn on_failure(&self) {
        let mut state = self.state.write().await;
        let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_failure_time.store(now, Ordering::Relaxed);

        match *state {
            CircuitBreakerState::Closed => {
                if failure_count >= self.config.failure_threshold {
                    *state = CircuitBreakerState::Open;
                    record_circuit_breaker_transition(&self.name, MetricsState::Closed, MetricsState::Open);
                    record_circuit_breaker_state(&self.name, MetricsState::Open);
                    tracing::warn!(service = %self.name, failures = %failure_count, "Circuit breaker opened");
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Any failure in half-open state reopens the circuit
                *state = CircuitBreakerState::Open;
                self.failure_count.store(1, Ordering::Relaxed);
                record_circuit_breaker_transition(&self.name, MetricsState::HalfOpen, MetricsState::Open);
                record_circuit_breaker_state(&self.name, MetricsState::Open);
                tracing::warn!(service = %self.name, "Circuit breaker reopened from HalfOpen");
            }
            CircuitBreakerState::Open => {
                // Already open
            }
        }
    }

    /// Get current state (for testing/monitoring)
    pub async fn get_state(&self) -> CircuitBreakerState {
        *self.state.read().await
    }

    /// Reset the circuit breaker (useful for testing)
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.last_failure_time.store(0, Ordering::Relaxed);
        record_circuit_breaker_state(&self.name, MetricsState::Closed);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError<E> {
    #[error("Circuit breaker is open")]
    Open,

    #[error("Call timed out")]
    Timeout,

    #[error("Call failed: {0}")]
    CallFailed(E),
}

/// Trait for services that can be protected by a circuit breaker
#[async_trait]
pub trait CircuitBreakerProtected {
    type Error;

    async fn with_circuit_breaker<F, T>(
        &self,
        name: &str,
        f: F,
    ) -> Result<T, CircuitBreakerError<Self::Error>>
    where
        F: std::future::Future<Output = Result<T, Self::Error>> + Send,
        T: Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, thiserror::Error)]
    #[error("Test error")]
    struct TestError;

    #[tokio::test]
    async fn test_circuit_breaker_success() {
        let cb = CircuitBreaker::new(
            "test-service".to_string(),
            CircuitBreakerConfig::default(),
        );

        let result = cb.call(async { Ok::<_, TestError>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_circuit_breaker_failure() {
        let cb = CircuitBreaker::new(
            "test-service".to_string(),
            CircuitBreakerConfig::default(),
        );

        let result = cb.call(async { Err::<i32, _>(TestError) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_timeout() {
        let cb = CircuitBreaker::new(
            "test-service".to_string(),
            CircuitBreakerConfig {
                timeout: Duration::from_millis(100),
                ..Default::default()
            },
        );

        let result = cb
            .call(async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                Ok::<_, TestError>(42)
            })
            .await;

        assert!(matches!(result, Err(CircuitBreakerError::Timeout)));
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::new(
            "test-service".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 3,
                ..Default::default()
            },
        );

        // Cause multiple failures
        for _ in 0..5 {
            let _ = cb.call(async { Err::<i32, _>(TestError) }).await;
        }

        // Check state is open
        let state = cb.get_state().await;
        assert_eq!(state, CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(
            "test-service".to_string(),
            CircuitBreakerConfig::default(),
        );

        // Cause failure
        let _ = cb.call(async { Err::<i32, _>(TestError) }).await;

        // Reset
        cb.reset().await;

        // Should work again
        let result = cb.call(async { Ok::<_, TestError>(42) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_config_default() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }
}
