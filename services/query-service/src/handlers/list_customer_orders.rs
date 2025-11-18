use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use read_model::OrderView;
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use uuid::Uuid;

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
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// List orders for a customer with pagination
pub async fn list_customer_orders_handler(
    State(state): State<AppState>,
    Path(customer_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<OrderListResponse>, (StatusCode, String)> {
    info!(
        "Listing orders for customer: {} (limit: {}, offset: {})",
        customer_id, params.limit, params.offset
    );

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
        .list_by_customer(customer_id, params.limit, params.offset)
        .await
    {
        Ok(orders) => {
            // Get total count
            let total = state
                .repository
                .count_by_customer(customer_id)
                .await
                .unwrap_or(0);

            info!(
                "Successfully retrieved {} orders for customer: {}",
                orders.len(),
                customer_id
            );

            Ok(Json(OrderListResponse {
                orders,
                total,
                limit: params.limit,
                offset: params.offset,
            }))
        }
        Err(e) => {
            error!("Failed to list orders for customer {}: {}", customer_id, e);
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
    fn test_default_limit() {
        assert_eq!(default_limit(), 20);
    }

    #[test]
    fn test_pagination_params_deserialization() {
        let json = r#"{"limit": 10, "offset": 5}"#;
        let params: PaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.limit, 10);
        assert_eq!(params.offset, 5);
    }
}
