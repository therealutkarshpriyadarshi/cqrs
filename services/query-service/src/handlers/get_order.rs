use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use read_model::OrderView;
use tracing::{error, info};
use uuid::Uuid;

use crate::state::AppState;

/// Get a single order by ID
pub async fn get_order_handler(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderView>, (StatusCode, String)> {
    info!("Fetching order: {}", order_id);

    // Try cache first
    if let Some(cached) = state.cache.get::<OrderView>(&order_id).await {
        info!("Cache hit for order: {}", order_id);
        return Ok(Json(cached));
    }

    info!("Cache miss for order: {}, querying database", order_id);

    // Query database
    match state.repository.get_by_id(order_id).await {
        Ok(Some(order)) => {
            // Update cache
            state.cache.set(&order_id, &order).await;

            info!("Successfully retrieved order: {}", order_id);
            Ok(Json(order))
        }
        Ok(None) => {
            info!("Order not found: {}", order_id);
            Err((StatusCode::NOT_FOUND, format!("Order not found: {}", order_id)))
        }
        Err(e) => {
            error!("Failed to fetch order {}: {}", order_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch order: {}", e),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_id_parsing() {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let parsed = Uuid::parse_str(&id_str).unwrap();
        assert_eq!(id, parsed);
    }
}
