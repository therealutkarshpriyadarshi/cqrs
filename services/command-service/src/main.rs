use common::telemetry::{TelemetryConfig, init_telemetry, shutdown_telemetry};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

mod handlers;
mod routes;
mod state;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize telemetry with Jaeger support
    let enable_jaeger = std::env::var("ENABLE_JAEGER")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let telemetry_config = TelemetryConfig {
        service_name: "command-service".to_string(),
        log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        jaeger_endpoint: std::env::var("JAEGER_ENDPOINT").ok(),
        enable_jaeger,
    };

    init_telemetry(telemetry_config)?;

    tracing::info!("Starting command service with Phase 5 features...");
    tracing::info!("Distributed tracing: {}", if enable_jaeger { "enabled" } else { "disabled" });

    // Initialize application state
    let state = state::AppState::new().await?;

    // Build router with tracing layer
    let app = routes::build_router(state).layer(TraceLayer::new_for_http());

    // Start server
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Command service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .await
        .map_err(|e| {
            tracing::error!("Server error: {}", e);
            e
        })?;

    // Shutdown telemetry gracefully
    shutdown_telemetry();

    Ok(())
}
