use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{RecommendationRequest, RecommendationResponse, SearchHistoryResponse};
use crate::services::{RecommendationService, SearchHistoryService};

type AppState = (Arc<RecommendationService>, Arc<SearchHistoryService>);

pub async fn get_recommendations(
    State((recommendation_service, search_history_service)): State<AppState>,
    Json(payload): Json<RecommendationRequest>,
) -> Result<Json<RecommendationResponse>, (StatusCode, String)> {
    if payload.query.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Query is required".to_string()));
    }

    let top_k = payload.top_k.unwrap_or(10);

    match recommendation_service
        .get_recommendations(&payload.query, top_k)
        .await
    {
        Ok(recommendations) => {
            let effective_user_id = payload
                .user_id
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            // Save search history
            if let Err(e) = search_history_service
                .save_search(&effective_user_id, &payload.query, &recommendations)
                .await
            {
                tracing::error!("Failed to save search history: {}", e);
                // Don't fail the request if history saving fails
            }

            Ok(Json(RecommendationResponse {
                recommendations,
                user_id: effective_user_id,
            }))
        }
        Err(e) => {
            tracing::error!("Error fetching recommendations: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch recommendations".to_string(),
            ))
        }
    }
}

pub async fn get_search_history(
    State((_, search_history_service)): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SearchHistoryResponse>, (StatusCode, String)> {
    let user_id = params
        .get("user_id")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "user_id is required".to_string()))?;

    match search_history_service.get_search_history(user_id).await {
        Ok(history) => Ok(Json(SearchHistoryResponse { history })),
        Err(e) => {
            tracing::error!("Error fetching search history: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch search history".to_string(),
            ))
        }
    }
}
