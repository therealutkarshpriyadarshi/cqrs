use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Router,
};
use common::metrics;

use crate::handlers::{cancel_order, confirm_order, create_order, deliver_order, health, ship_order};
use crate::state::AppState;

/// Prometheus metrics endpoint handler
async fn metrics_handler() -> impl IntoResponse {
    match metrics::gather_metrics() {
        Ok(metrics) => (StatusCode::OK, metrics),
        Err(e) => {
            tracing::error!("Failed to gather metrics: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, String::from("Failed to gather metrics"))
        }
    }
}

/// Build the application router with all routes
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/metrics", get(metrics_handler))
        .route("/api/v1/orders", post(create_order::handle))
        .route("/api/v1/orders/:id/confirm", put(confirm_order::handle))
        .route("/api/v1/orders/:id/cancel", put(cancel_order::handle))
        .route("/api/v1/orders/:id/ship", put(ship_order::handle))
        .route("/api/v1/orders/:id/deliver", put(deliver_order::handle))
        .with_state(state)
}
