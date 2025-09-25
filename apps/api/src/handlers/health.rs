use crate::models::HealthResponse;
use crate::services::RecommendationService;
use actix_web::{get, options, web, HttpResponse};
use log::debug;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service is healthy and background prewarming has been triggered", body = HealthResponse),
    ),
    summary = "Check service health and trigger background prewarming",
    description = "Returns the current status and timestamp of the service. This endpoint also initiates a background prewarming process to reduce cold start latency for subsequent requests."
)]
#[get("/health")]
pub async fn health_check(
    recommendation_service: web::Data<RecommendationService>,
) -> HttpResponse {
    // Trigger background prewarming without waiting for it to complete
    // This helps mitigate cold starts by initializing services when the health check is called
    tokio::spawn(async move {
        if let Err(e) = recommendation_service.prewarm().await {
            debug!(
                "Background prewarming during health check encountered an issue: {}",
                e
            );
        } else {
            debug!("Background prewarming during health check completed successfully");
        }
    });

    // Add explicit CORS headers to ensure the endpoint can be accessed from different origins
    HttpResponse::Ok()
        .append_header(("Access-Control-Allow-Origin", "*"))
        .append_header(("Access-Control-Allow-Methods", "GET, OPTIONS"))
        .append_header((
            "Access-Control-Allow-Headers",
            "Content-Type, X-Prewarm-Source",
        ))
        .json(serde_json::json!({
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "prewarm": "background"
        }))
}

/// OPTIONS handler for the health endpoint to handle CORS preflight requests
#[options("/health")]
pub async fn health_options() -> HttpResponse {
    HttpResponse::Ok()
        .append_header(("Access-Control-Allow-Origin", "*"))
        .append_header(("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
        .append_header((
            "Access-Control-Allow-Headers",
            "Content-Type, X-Prewarm-Source, Authorization",
        ))
        .append_header(("Access-Control-Max-Age", "3600"))
        .finish()
}
