use crate::{error::ApiError, models::RecommendationRequest, services::RecommendationService};
use actix_web::{
    post,
    web::{self, Json},
    HttpResponse,
};

pub fn recommendations_config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/recommendations").service(get_recommendations));
}

#[post("/")]
async fn get_recommendations(
    request: Json<RecommendationRequest>,
    recommendation_service: web::Data<RecommendationService>,
) -> Result<HttpResponse, ApiError> {
    let top_k = request.top_k;

    if request.query.trim().is_empty() {
        return Err(ApiError::InvalidInput("Query cannot be empty".to_string()));
    }

    let recommendations = recommendation_service
        .get_recommendations(&request.query, top_k)
        .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "recommendations": recommendations,
    })))
}
