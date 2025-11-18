use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use read_model::OrderView;
use tracing::{error, info};

use crate::state::AppState;

/// Get an order by order number
pub async fn get_order_by_number_handler(
    State(state): State<AppState>,
    Path(order_number): Path<String>,
) -> Result<Json<OrderView>, (StatusCode, String)> {
    info!("Searching for order by number: {}", order_number);

    match state.repository.search_by_order_number(&order_number).await {
        Ok(Some(order)) => {
            // Update cache
            state.cache.set(&order.order_id, &order).await;

            info!("Successfully found order: {} ({})", order_number, order.order_id);
            Ok(Json(order))
        }
        Ok(None) => {
            info!("Order not found with number: {}", order_number);
            Err((
                StatusCode::NOT_FOUND,
                format!("Order not found with number: {}", order_number),
            ))
        }
        Err(e) => {
            error!("Failed to search order by number {}: {}", order_number, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to search order: {}", e),
            ))
        }
    }
}
