use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_histogram_vec, register_int_gauge_vec, CounterVec,
    Encoder, HistogramVec, IntGaugeVec, TextEncoder,
};

lazy_static! {
    // Command metrics
    pub static ref COMMAND_COUNTER: CounterVec = register_counter_vec!(
        "cqrs_commands_total",
        "Total number of commands processed",
        &["command_type", "status"]
    )
    .expect("metric cannot be created");

    pub static ref COMMAND_DURATION: HistogramVec = register_histogram_vec!(
        "cqrs_command_duration_seconds",
        "Command processing duration in seconds",
        &["command_type"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("metric cannot be created");

    // Event metrics
    pub static ref EVENT_COUNTER: CounterVec = register_counter_vec!(
        "cqrs_events_total",
        "Total number of events processed",
        &["event_type", "status"]
    )
    .expect("metric cannot be created");

    pub static ref EVENT_DURATION: HistogramVec = register_histogram_vec!(
        "cqrs_event_duration_seconds",
        "Event processing duration in seconds",
        &["event_type"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("metric cannot be created");

    // Query metrics
    pub static ref QUERY_COUNTER: CounterVec = register_counter_vec!(
        "cqrs_queries_total",
        "Total number of queries processed",
        &["query_type", "status"]
    )
    .expect("metric cannot be created");

    pub static ref QUERY_DURATION: HistogramVec = register_histogram_vec!(
        "cqrs_query_duration_seconds",
        "Query processing duration in seconds",
        &["query_type"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    )
    .expect("metric cannot be created");

    // Saga metrics
    pub static ref SAGA_COUNTER: CounterVec = register_counter_vec!(
        "cqrs_sagas_total",
        "Total number of sagas processed",
        &["saga_type", "status"]
    )
    .expect("metric cannot be created");

    pub static ref SAGA_DURATION: HistogramVec = register_histogram_vec!(
        "cqrs_saga_duration_seconds",
        "Saga processing duration in seconds",
        &["saga_type"],
        vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0]
    )
    .expect("metric cannot be created");

    pub static ref SAGA_COMPENSATION_COUNTER: CounterVec = register_counter_vec!(
        "cqrs_saga_compensations_total",
        "Total number of saga compensations",
        &["saga_type", "step"]
    )
    .expect("metric cannot be created");

    // Cache metrics
    pub static ref CACHE_HIT_COUNTER: CounterVec = register_counter_vec!(
        "cqrs_cache_requests_total",
        "Total number of cache requests",
        &["cache_type", "status"]
    )
    .expect("metric cannot be created");

    // Circuit breaker metrics
    pub static ref CIRCUIT_BREAKER_STATE: IntGaugeVec = register_int_gauge_vec!(
        "cqrs_circuit_breaker_state",
        "Circuit breaker state (0=closed, 1=open, 2=half-open)",
        &["service"]
    )
    .expect("metric cannot be created");

    pub static ref CIRCUIT_BREAKER_COUNTER: CounterVec = register_counter_vec!(
        "cqrs_circuit_breaker_total",
        "Total number of circuit breaker state changes",
        &["service", "from_state", "to_state"]
    )
    .expect("metric cannot be created");

    // Event store metrics
    pub static ref EVENT_STORE_OPERATIONS: CounterVec = register_counter_vec!(
        "cqrs_event_store_operations_total",
        "Total number of event store operations",
        &["operation", "status"]
    )
    .expect("metric cannot be created");

    pub static ref EVENT_STORE_DURATION: HistogramVec = register_histogram_vec!(
        "cqrs_event_store_duration_seconds",
        "Event store operation duration in seconds",
        &["operation"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    )
    .expect("metric cannot be created");

    // Projection lag metrics
    pub static ref PROJECTION_LAG: HistogramVec = register_histogram_vec!(
        "cqrs_projection_lag_seconds",
        "Projection lag behind event stream in seconds",
        &["projection_type"],
        vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 300.0]
    )
    .expect("metric cannot be created");

    // Idempotency metrics
    pub static ref IDEMPOTENCY_CHECK: CounterVec = register_counter_vec!(
        "cqrs_idempotency_checks_total",
        "Total number of idempotency checks",
        &["status"]
    )
    .expect("metric cannot be created");
}

/// Get all metrics in Prometheus text format
pub fn gather_metrics() -> Result<String, Box<dyn std::error::Error>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}

/// Helper function to record command execution
pub fn record_command(command_type: &str, success: bool, duration_secs: f64) {
    let status = if success { "success" } else { "error" };
    COMMAND_COUNTER
        .with_label_values(&[command_type, status])
        .inc();
    COMMAND_DURATION
        .with_label_values(&[command_type])
        .observe(duration_secs);
}

/// Helper function to record event processing
pub fn record_event(event_type: &str, success: bool, duration_secs: f64) {
    let status = if success { "success" } else { "error" };
    EVENT_COUNTER
        .with_label_values(&[event_type, status])
        .inc();
    EVENT_DURATION
        .with_label_values(&[event_type])
        .observe(duration_secs);
}

/// Helper function to record query execution
pub fn record_query(query_type: &str, success: bool, duration_secs: f64) {
    let status = if success { "success" } else { "error" };
    QUERY_COUNTER
        .with_label_values(&[query_type, status])
        .inc();
    QUERY_DURATION
        .with_label_values(&[query_type])
        .observe(duration_secs);
}

/// Helper function to record saga execution
pub fn record_saga(saga_type: &str, success: bool, duration_secs: f64) {
    let status = if success { "success" } else { "error" };
    SAGA_COUNTER
        .with_label_values(&[saga_type, status])
        .inc();
    SAGA_DURATION
        .with_label_values(&[saga_type])
        .observe(duration_secs);
}

/// Helper function to record saga compensation
pub fn record_saga_compensation(saga_type: &str, step: &str) {
    SAGA_COMPENSATION_COUNTER
        .with_label_values(&[saga_type, step])
        .inc();
}

/// Helper function to record cache hit/miss
pub fn record_cache_request(cache_type: &str, hit: bool) {
    let status = if hit { "hit" } else { "miss" };
    CACHE_HIT_COUNTER
        .with_label_values(&[cache_type, status])
        .inc();
}

/// Helper function to record circuit breaker state
pub fn record_circuit_breaker_state(service: &str, state: CircuitBreakerState) {
    let state_value = match state {
        CircuitBreakerState::Closed => 0,
        CircuitBreakerState::Open => 1,
        CircuitBreakerState::HalfOpen => 2,
    };
    CIRCUIT_BREAKER_STATE
        .with_label_values(&[service])
        .set(state_value);
}

/// Helper function to record circuit breaker state change
pub fn record_circuit_breaker_transition(service: &str, from: CircuitBreakerState, to: CircuitBreakerState) {
    CIRCUIT_BREAKER_COUNTER
        .with_label_values(&[service, &format!("{:?}", from), &format!("{:?}", to)])
        .inc();
}

/// Helper function to record event store operation
pub fn record_event_store_operation(operation: &str, success: bool, duration_secs: f64) {
    let status = if success { "success" } else { "error" };
    EVENT_STORE_OPERATIONS
        .with_label_values(&[operation, status])
        .inc();
    EVENT_STORE_DURATION
        .with_label_values(&[operation])
        .observe(duration_secs);
}

/// Helper function to record projection lag
pub fn record_projection_lag(projection_type: &str, lag_secs: f64) {
    PROJECTION_LAG
        .with_label_values(&[projection_type])
        .observe(lag_secs);
}

/// Helper function to record idempotency check
pub fn record_idempotency_check(duplicate: bool) {
    let status = if duplicate { "duplicate" } else { "new" };
    IDEMPOTENCY_CHECK.with_label_values(&[status]).inc();
}

#[derive(Debug, Clone, Copy)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gather_metrics() {
        let result = gather_metrics();
        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(metrics.contains("cqrs_"));
    }

    #[test]
    fn test_record_command() {
        record_command("CreateOrder", true, 0.5);
        let metrics = gather_metrics().unwrap();
        assert!(metrics.contains("cqrs_commands_total"));
    }

    #[test]
    fn test_record_event() {
        record_event("OrderCreated", true, 0.1);
        let metrics = gather_metrics().unwrap();
        assert!(metrics.contains("cqrs_events_total"));
    }

    #[test]
    fn test_circuit_breaker_state() {
        let state = CircuitBreakerState::Open;
        record_circuit_breaker_state("payment-service", state);
        let metrics = gather_metrics().unwrap();
        assert!(metrics.contains("cqrs_circuit_breaker_state"));
    }
}
