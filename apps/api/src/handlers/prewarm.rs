//! Prewarm endpoint to address cold start issues on serverless platforms

use crate::{error::Result, services::RecommendationService};
use actix_web::{web, HttpResponse};
use log::{debug, info};
use serde_json::json;

/// Prewarming endpoint to initialize all services and mitigate cold start issues
///
/// This endpoint performs several operations to ensure the application is fully initialized:
/// 1. Initializes the ML embedder
/// 2. Tests connection to Pinecone
/// 3. Executes a sample query to prepare the entire pipeline
///
/// # Returns
///
/// A JSON response indicating the prewarm operation was successful
#[utoipa::path(
    get,
    path = "/api/prewarm",
    tag = "System",
    responses(
        (status = 200, description = "API services successfully prewarmed", body = serde_json::Value),
        (status = 500, description = "Error during prewarming", body = serde_json::Value)
    ),
    summary = "Prewarm API services to mitigate cold starts",
    description = "Initializes all services (ML model, Pinecone, caches) to reduce latency for subsequent requests. Useful after deployment or during periods of inactivity."
)]
#[actix_web::get("/prewarm")]
pub async fn prewarm(
    recommendation_service: web::Data<RecommendationService>,
) -> Result<HttpResponse> {
    info!("Prewarming API services...");

    // Use the dedicated prewarm method to warm up all services
    // This will initialize connections, caches, and perform test queries
    match recommendation_service.prewarm().await {
        Ok(was_first) => {
            info!("Successfully prewarmed API services");
            debug!("Prewarm operation completed all stages: ML model, Pinecone connection, and recommendation pipeline");

            let message = if was_first {
                "API services successfully prewarmed for the first time"
            } else {
                "API services already prewarmed"
            };

            Ok(HttpResponse::Ok().json(json!({
                "status": "ok",
                "message": message,
                "first_prewarm": was_first,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })))
        }
        Err(e) => {
            // Log but don't return an error - the service might still function for other requests
            info!("Prewarm partially completed with warning: {}", e);

            Ok(HttpResponse::Ok().json(json!({
                "status": "partial",
                "message": "API services partially prewarmed",
                "warning": e.to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            })))
        }
    }
}
