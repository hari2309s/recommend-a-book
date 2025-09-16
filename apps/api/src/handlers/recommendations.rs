use crate::{
    error::ApiError,
    models::{ErrorResponse, RecommendationRequest, RecommendationResponse},
    services::RecommendationService,
};
use actix_web::{
    post,
    web::{self, Json},
    HttpResponse,
};
use uuid;

pub fn recommendations_config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/recommendations").service(get_recommendations));
}

/// Get book recommendations based on query
#[utoipa::path(
    post,
    path = "/api/recommendations",
    tag = "Recommendations",
    request_body = RecommendationRequest,
    responses(
        (status = 200, description = "Successfully retrieved book recommendations", body = RecommendationResponse),
        (status = 400, description = "Invalid input parameters", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    summary = "Get book recommendations",
    description = "Returns a list of book recommendations based on the provided search query. Uses machine learning to find semantically similar books. Each recommendation includes the book details and a similarity score."
)]
#[post("/")]
pub async fn get_recommendations(
    request: Json<RecommendationRequest>,
    recommendation_service: web::Data<RecommendationService>,
) -> Result<HttpResponse, ApiError> {
    let top_k = request.top_k;

    if request.query.trim().is_empty() {
        return Err(ApiError::InvalidInput("Query cannot be empty".to_string()));
    }

    let books = recommendation_service
        .get_recommendations(&request.query, top_k)
        .await?;

    // Convert Books to BookRecommendations
    let recommendations = books
        .into_iter()
        .enumerate()
        .map(|(index, book)| {
            // Calculate a synthetic similarity score that decreases with position
            // First result has score 1.0, last result approaches 0.5
            let similarity_score = 1.0 - (0.5 * (index as f32 / top_k.max(1) as f32));

            crate::models::BookRecommendation {
                id: format!("rec_{}", uuid::Uuid::new_v4()),
                book,
                similarity_score,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(RecommendationResponse { recommendations }))
}
