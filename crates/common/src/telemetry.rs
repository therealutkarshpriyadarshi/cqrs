use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Telemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub service_name: String,
    pub log_level: String,
    pub jaeger_endpoint: Option<String>,
    pub enable_jaeger: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "cqrs-service".to_string(),
            log_level: "info".to_string(),
            jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
            enable_jaeger: false,
        }
    }
}

/// Initialize tracing/logging for the application with optional Jaeger support
pub fn init_telemetry(config: TelemetryConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Set up global propagator for trace context
    global::set_text_map_propagator(TraceContextPropagator::new());

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .json();

    // Build subscriber with or without Jaeger tracing
    if config.enable_jaeger {
        let tracer = opentelemetry_jaeger::new_agent_pipeline()
            .with_service_name(&config.service_name)
            .with_endpoint(config.jaeger_endpoint.unwrap_or_else(|| "localhost:6831".to_string()))
            .install_batch(opentelemetry_sdk::runtime::Tokio)?;

        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(telemetry_layer)
            .init();

        tracing::info!(
            "Telemetry initialized with Jaeger tracing for service: {}",
            config.service_name
        );
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();

        tracing::info!(
            "Telemetry initialized without Jaeger for service: {}",
            config.service_name
        );
    }

    Ok(())
}

/// Initialize basic telemetry without Jaeger (backwards compatibility)
pub fn init_basic_telemetry(log_level: &str) {
    let config = TelemetryConfig {
        service_name: "cqrs-service".to_string(),
        log_level: log_level.to_string(),
        jaeger_endpoint: None,
        enable_jaeger: false,
    };

    let _ = init_telemetry(config);
}

/// Shutdown telemetry gracefully
pub fn shutdown_telemetry() {
    global::shutdown_tracer_provider();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, "cqrs-service");
        assert_eq!(config.log_level, "info");
        assert!(!config.enable_jaeger);
    }

    #[test]
    fn test_init_basic_telemetry() {
        // This test just ensures the function can be called
        // In a real test, we'd verify the tracing is set up correctly
        // but that's difficult to test in isolation
        init_basic_telemetry("debug");
    }
}
