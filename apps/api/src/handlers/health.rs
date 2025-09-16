use crate::models::HealthResponse;
use actix_web::{get, HttpResponse};

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    ),
    summary = "Check service health",
    description = "Returns the current status and timestamp of the service"
)]
#[get("/health")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
