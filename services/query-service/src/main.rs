use anyhow::Result;
use common::telemetry::{TelemetryConfig, init_telemetry, shutdown_telemetry};
use std::net::SocketAddr;

mod handlers;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize telemetry with Jaeger support
    let enable_jaeger = std::env::var("ENABLE_JAEGER")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let telemetry_config = TelemetryConfig {
        service_name: "query-service".to_string(),
        log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        jaeger_endpoint: std::env::var("JAEGER_ENDPOINT").ok(),
        enable_jaeger,
    };

    init_telemetry(telemetry_config)?;

    tracing::info!("Starting Query Service with Phase 5 features...");
    tracing::info!("Distributed tracing: {}", if enable_jaeger { "enabled" } else { "disabled" });

    // Configuration from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/cqrs_events".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let cache_ttl: usize = std::env::var("CACHE_TTL_SECONDS")
        .unwrap_or_else(|_| "300".to_string())
        .parse()
        .unwrap_or(300);
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse()
        .unwrap_or(8081);

    tracing::info!("Configuration:");
    tracing::info!("  Database URL: {}", database_url);
    tracing::info!("  Redis URL: {}", redis_url);
    tracing::info!("  Cache TTL: {} seconds", cache_ttl);
    tracing::info!("  Port: {}", port);

    // Initialize application state
    let state = AppState::new(&database_url, &redis_url, cache_ttl).await?;

    // Build router
    let app = routes::create_router(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Query service listening on {}", addr);

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
