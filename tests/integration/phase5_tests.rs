use common::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError};
use common::metrics;
use common::telemetry::{TelemetryConfig, init_basic_telemetry};
use event_store::{Event, EventStore, IdempotencyChecker, generate_idempotency_key};
use std::time::Duration;
use uuid::Uuid;

/// Test telemetry initialization
#[test]
fn test_telemetry_initialization() {
    // Should not panic
    init_basic_telemetry("debug");
}

/// Test telemetry config
#[test]
fn test_telemetry_config() {
    let config = TelemetryConfig {
        service_name: "test-service".to_string(),
        log_level: "debug".to_string(),
        jaeger_endpoint: Some("http://localhost:14268".to_string()),
        enable_jaeger: false,
    };

    assert_eq!(config.service_name, "test-service");
    assert_eq!(config.log_level, "debug");
    assert!(!config.enable_jaeger);
}

/// Test metrics gathering
#[test]
fn test_metrics_gathering() {
    // Record some test metrics
    metrics::record_command("CreateOrder", true, 0.5);
    metrics::record_event("OrderCreated", true, 0.1);
    metrics::record_query("GetOrder", true, 0.05);

    // Gather metrics
    let result = metrics::gather_metrics();
    assert!(result.is_ok());

    let metrics_output = result.unwrap();
    assert!(metrics_output.contains("cqrs_commands_total"));
    assert!(metrics_output.contains("cqrs_events_total"));
    assert!(metrics_output.contains("cqrs_queries_total"));
}

/// Test command metrics recording
#[test]
fn test_command_metrics() {
    metrics::record_command("TestCommand", true, 1.23);
    let result = metrics::gather_metrics().unwrap();
    assert!(result.contains("cqrs_commands_total"));
    assert!(result.contains("TestCommand"));
}

/// Test event metrics recording
#[test]
fn test_event_metrics() {
    metrics::record_event("TestEvent", false, 0.456);
    let result = metrics::gather_metrics().unwrap();
    assert!(result.contains("cqrs_events_total"));
    assert!(result.contains("TestEvent"));
}

/// Test saga metrics recording
#[test]
fn test_saga_metrics() {
    metrics::record_saga("OrderProcessingSaga", true, 2.5);
    metrics::record_saga_compensation("OrderProcessingSaga", "ReserveInventory");
    let result = metrics::gather_metrics().unwrap();
    assert!(result.contains("cqrs_sagas_total"));
    assert!(result.contains("cqrs_saga_compensations_total"));
}

/// Test cache metrics recording
#[test]
fn test_cache_metrics() {
    metrics::record_cache_request("order-cache", true);
    metrics::record_cache_request("order-cache", false);
    let result = metrics::gather_metrics().unwrap();
    assert!(result.contains("cqrs_cache_requests_total"));
}

/// Test circuit breaker state recording
#[test]
fn test_circuit_breaker_metrics() {
    metrics::record_circuit_breaker_state("test-service", metrics::CircuitBreakerState::Open);
    metrics::record_circuit_breaker_transition(
        "test-service",
        metrics::CircuitBreakerState::Closed,
        metrics::CircuitBreakerState::Open,
    );
    let result = metrics::gather_metrics().unwrap();
    assert!(result.contains("cqrs_circuit_breaker_state"));
    assert!(result.contains("cqrs_circuit_breaker_total"));
}

/// Test event store operation metrics
#[test]
fn test_event_store_metrics() {
    metrics::record_event_store_operation("append", true, 0.05);
    metrics::record_event_store_operation("load", true, 0.02);
    let result = metrics::gather_metrics().unwrap();
    assert!(result.contains("cqrs_event_store_operations_total"));
    assert!(result.contains("cqrs_event_store_duration_seconds"));
}

/// Test projection lag metrics
#[test]
fn test_projection_lag_metrics() {
    metrics::record_projection_lag("order-projection", 1.5);
    let result = metrics::gather_metrics().unwrap();
    assert!(result.contains("cqrs_projection_lag_seconds"));
}

/// Test idempotency check metrics
#[test]
fn test_idempotency_metrics() {
    metrics::record_idempotency_check(false);
    metrics::record_idempotency_check(true);
    let result = metrics::gather_metrics().unwrap();
    assert!(result.contains("cqrs_idempotency_checks_total"));
}

/// Test circuit breaker success
#[tokio::test]
async fn test_circuit_breaker_success() {
    let cb = CircuitBreaker::new(
        "test-service".to_string(),
        CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(5),
            half_open_timeout: Duration::from_secs(10),
        },
    );

    let result = cb.call(async { Ok::<i32, std::io::Error>(42) }).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

/// Test circuit breaker failure
#[tokio::test]
async fn test_circuit_breaker_failure() {
    let cb = CircuitBreaker::new(
        "test-service-fail".to_string(),
        CircuitBreakerConfig::default(),
    );

    let result = cb
        .call(async { Err::<i32, _>(std::io::Error::new(std::io::ErrorKind::Other, "test error")) })
        .await;
    assert!(result.is_err());
}

/// Test circuit breaker timeout
#[tokio::test]
async fn test_circuit_breaker_timeout() {
    let cb = CircuitBreaker::new(
        "test-service-timeout".to_string(),
        CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            half_open_timeout: Duration::from_secs(10),
        },
    );

    let result = cb
        .call(async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok::<i32, std::io::Error>(42)
        })
        .await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CircuitBreakerError::Timeout));
}

/// Test circuit breaker config
#[test]
fn test_circuit_breaker_config() {
    let config = CircuitBreakerConfig::default();
    assert_eq!(config.failure_threshold, 5);
    assert_eq!(config.success_threshold, 2);
    assert_eq!(config.timeout, Duration::from_secs(60));
}

/// Test idempotency key generation
#[test]
fn test_idempotency_key_generation() {
    let id = Uuid::new_v4();
    let key = generate_idempotency_key(&id, "CreateOrder");
    assert!(key.starts_with("CreateOrder:"));
    assert!(key.contains(&id.to_string()));
}

/// Integration test: Multiple operations with circuit breaker
#[tokio::test]
async fn test_circuit_breaker_multiple_operations() {
    let cb = CircuitBreaker::new(
        "multi-op-service".to_string(),
        CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
            half_open_timeout: Duration::from_secs(5),
        },
    );

    // First successful call
    let result1 = cb.call(async { Ok::<i32, std::io::Error>(1) }).await;
    assert!(result1.is_ok());

    // Second successful call
    let result2 = cb.call(async { Ok::<i32, std::io::Error>(2) }).await;
    assert!(result2.is_ok());

    // Third successful call
    let result3 = cb.call(async { Ok::<i32, std::io::Error>(3) }).await;
    assert!(result3.is_ok());
}

/// Integration test: Metrics for complete order flow
#[test]
fn test_complete_order_flow_metrics() {
    let order_id = Uuid::new_v4();

    // Command received
    metrics::record_command("CreateOrder", true, 0.5);

    // Event stored
    metrics::record_event_store_operation("append", true, 0.05);

    // Event published
    metrics::record_event("OrderCreated", true, 0.1);

    // Saga started
    metrics::record_saga("OrderProcessingSaga", true, 2.0);

    // Query processed
    metrics::record_query("GetOrder", true, 0.02);

    // Cache hit
    metrics::record_cache_request("order-cache", true);

    let result = metrics::gather_metrics().unwrap();
    assert!(result.contains("cqrs_commands_total"));
    assert!(result.contains("cqrs_events_total"));
    assert!(result.contains("cqrs_sagas_total"));
    assert!(result.contains("cqrs_queries_total"));
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test all metrics are initialized
    #[test]
    fn test_all_metrics_initialized() {
        let metrics = metrics::gather_metrics().unwrap();

        // Check that all metric families are present
        assert!(metrics.contains("cqrs_commands_total"));
        assert!(metrics.contains("cqrs_command_duration_seconds"));
        assert!(metrics.contains("cqrs_events_total"));
        assert!(metrics.contains("cqrs_event_duration_seconds"));
        assert!(metrics.contains("cqrs_queries_total"));
        assert!(metrics.contains("cqrs_query_duration_seconds"));
        assert!(metrics.contains("cqrs_sagas_total"));
        assert!(metrics.contains("cqrs_saga_duration_seconds"));
        assert!(metrics.contains("cqrs_cache_requests_total"));
        assert!(metrics.contains("cqrs_circuit_breaker_state"));
        assert!(metrics.contains("cqrs_event_store_operations_total"));
        assert!(metrics.contains("cqrs_projection_lag_seconds"));
        assert!(metrics.contains("cqrs_idempotency_checks_total"));
    }
}
