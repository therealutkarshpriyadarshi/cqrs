use axum::{
    routing::{get, post, put},
    Router,
};

use crate::handlers::{cancel_order, confirm_order, create_order, deliver_order, health, ship_order};
use crate::state::AppState;

/// Build the application router with all routes
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/api/v1/orders", post(create_order::handle))
        .route("/api/v1/orders/:id/confirm", put(confirm_order::handle))
        .route("/api/v1/orders/:id/cancel", put(cancel_order::handle))
        .route("/api/v1/orders/:id/ship", put(ship_order::handle))
        .route("/api/v1/orders/:id/deliver", put(deliver_order::handle))
        .with_state(state)
}
