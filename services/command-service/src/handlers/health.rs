use axum::{http::StatusCode, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Health check endpoint
pub async fn health_check() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            service: "command-service".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let (status, response) = health_check().await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response.status, "healthy");
        assert_eq!(response.service, "command-service");
    }
}
