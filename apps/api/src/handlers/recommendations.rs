use crate::{
    error::ApiError,
    models::{RecommendationRequest, SearchHistory, SearchHistoryRequest},
    services::{RecommendationService, SearchHistoryService},
};
use actix_web::{
    post,
    web::{self, Json},
    HttpResponse,
};
use tracing::error;
use uuid::Uuid;

pub fn recommendations_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/recommendations")
            .service(get_recommendations)
            .service(get_search_history),
    );
}

#[post("/")]
async fn get_recommendations(
    request: Json<RecommendationRequest>,
    recommendation_service: web::Data<RecommendationService>,
    search_history_service: web::Data<SearchHistoryService>,
) -> Result<HttpResponse, ApiError> {
    let user_id = request.user_id.unwrap_or_else(Uuid::new_v4);
    let top_k = request.top_k;

    if request.query.trim().is_empty() {
        return Err(ApiError::InvalidInput("Query cannot be empty".to_string()));
    }

    let recommendations = recommendation_service
        .get_recommendations(&request.query, top_k)
        .await?;

    // Save search history
    let history = SearchHistory {
        id: None,
        user_id,
        query: request.query.clone(),
        recommendations: recommendations.clone(),
        created_at: None,
    };

    if let Err(e) = search_history_service.save_search(&history).await {
        error!("Failed to save search history: {}", e);
        // Continue even if saving history fails
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "recommendations": recommendations,
        "user_id": user_id
    })))
}

#[post("/history")]
async fn get_search_history(
    request: Json<SearchHistoryRequest>,
    search_history_service: web::Data<SearchHistoryService>,
) -> Result<HttpResponse, ApiError> {
    let user_id = Uuid::parse_str(&request.user_id)
        .map_err(|e| ApiError::InvalidInput(format!("Invalid user ID: {}", e)))?;

    let history = search_history_service
        .get_search_history(user_id, Some(request.limit))
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "history": history })))
}
