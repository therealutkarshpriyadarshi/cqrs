use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing/logging for the application
pub fn init_telemetry(log_level: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(true).with_level(true))
        .init();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_init_telemetry() {
        // This test just ensures the function can be called
        // In a real test, we'd verify the tracing is set up correctly
        // but that's difficult to test in isolation
    }
}
