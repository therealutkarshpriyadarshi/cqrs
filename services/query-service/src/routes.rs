use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use common::metrics;
use tower_http::trace::TraceLayer;

use crate::handlers;
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

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))
        .route("/metrics", get(metrics_handler))

        // Order queries
        .route("/api/v1/orders/:id", get(handlers::get_order::get_order_handler))
        .route("/api/v1/orders/number/:order_number", get(handlers::get_by_number::get_order_by_number_handler))
        .route("/api/v1/customers/:customer_id/orders", get(handlers::list_customer_orders::list_customer_orders_handler))
        .route("/api/v1/orders/status/:status", get(handlers::list_by_status::list_orders_by_status_handler))

        // Middleware
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
