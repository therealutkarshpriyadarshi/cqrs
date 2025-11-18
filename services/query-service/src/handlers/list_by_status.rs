use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use read_model::OrderView;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize)]
pub struct OrderListResponse {
    pub orders: Vec<OrderView>,
    pub status: String,
    pub limit: i64,
    pub offset: i64,
}

/// List orders by status with pagination
pub async fn list_orders_by_status_handler(
    State(state): State<AppState>,
    Path(status): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<OrderListResponse>, (StatusCode, String)> {
    info!(
        "Listing orders with status: {} (limit: {}, offset: {})",
        status, params.limit, params.offset
    );

    // Validate status
    let status_upper = status.to_uppercase();
    let valid_statuses = ["CREATED", "CONFIRMED", "CANCELLED", "SHIPPED", "DELIVERED"];
    if !valid_statuses.contains(&status_upper.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid status. Must be one of: {:?}", valid_statuses),
        ));
    }

    // Validate pagination params
    if params.limit < 1 || params.limit > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Limit must be between 1 and 100".to_string(),
        ));
    }

    if params.offset < 0 {
        return Err((StatusCode::BAD_REQUEST, "Offset must be >= 0".to_string()));
    }

    // Fetch orders
    match state
        .repository
        .list_by_status(&status_upper, params.limit, params.offset)
        .await
    {
        Ok(orders) => {
            info!(
                "Successfully retrieved {} orders with status: {}",
                orders.len(),
                status_upper
            );

            Ok(Json(OrderListResponse {
                orders,
                status: status_upper,
                limit: params.limit,
                offset: params.offset,
            }))
        }
        Err(e) => {
            error!("Failed to list orders by status {}: {}", status_upper, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list orders: {}", e),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_validation() {
        let valid = ["CREATED", "CONFIRMED", "CANCELLED", "SHIPPED", "DELIVERED"];
        for status in &valid {
            assert!(valid.contains(&status.to_uppercase().as_str()));
        }
    }

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 20);
    }
}
