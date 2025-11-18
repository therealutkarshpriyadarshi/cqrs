use axum::{
    routing::get,
    Router,
};
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))

        // Order queries
        .route("/api/v1/orders/:id", get(handlers::get_order::get_order_handler))
        .route("/api/v1/orders/number/:order_number", get(handlers::get_by_number::get_order_by_number_handler))
        .route("/api/v1/customers/:customer_id/orders", get(handlers::list_customer_orders::list_customer_orders_handler))
        .route("/api/v1/orders/status/:status", get(handlers::list_by_status::list_orders_by_status_handler))

        // Middleware
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
