use crate::{
    error::ApiError,
    models::{RecommendationRequest, SearchHistory, SearchHistoryRequest},
    services::{recommendation::RecommendationService, search_history::SearchHistoryService},
};
use actix_web::{
    post,
    web::{self, Json, ServiceConfig},
    HttpResponse,
};
use log::error;
use uuid::Uuid;

pub fn config(cfg: &mut ServiceConfig) {
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
) -> HttpResponse {
    let user_id = request.user_id.unwrap_or_else(Uuid::new_v4);
    let top_k = request.top_k.unwrap_or(10);

    if request.query.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Query cannot be empty"
        }));
    }

    match recommendation_service
        .get_recommendations(&request.query, top_k)
        .await
    {
        Ok(recommendations) => {
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

            HttpResponse::Ok().json(serde_json::json!({
                "recommendations": recommendations,
                "user_id": user_id
            }))
        }
        Err(e) => {
            error!("Error getting recommendations: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch recommendations"
            }))
        }
    }
}

#[post("/history")]
async fn get_search_history(
    request: Json<SearchHistoryRequest>,
    search_history_service: web::Data<SearchHistoryService>,
) -> Result<HttpResponse, ApiError> {
    let user_id = Uuid::parse_str(&request.user_id)
        .map_err(|e| ApiError::InvalidInput(format!("Invalid user ID: {}", e)))?;

    match search_history_service
        .get_search_history(user_id, request.limit)
        .await
    {
        Ok(history) => Ok(HttpResponse::Ok().json(serde_json::json!({ "history": history }))),
        Err(e) => match e {
            ApiError::InvalidInput(_) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            }))),
            _ => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch search history"
            }))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use uuid::Uuid;

    #[actix_web::test]
    async fn test_empty_query() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(RecommendationService::mock()))
                .app_data(web::Data::new(SearchHistoryService::mock()))
                .configure(config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/recommendations")
            .set_json(RecommendationRequest {
                query: "".to_string(),
                top_k: None,
                user_id: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_valid_recommendation_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(RecommendationService::mock()))
                .app_data(web::Data::new(SearchHistoryService::mock()))
                .configure(config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/recommendations")
            .set_json(RecommendationRequest {
                query: "test query".to_string(),
                top_k: Some(5),
                user_id: Some(Uuid::new_v4()),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_search_history_request() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(SearchHistoryService::mock()))
                .configure(config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/recommendations/history")
            .set_json(SearchHistoryRequest {
                user_id: Uuid::new_v4().to_string(),
                limit: Some(10),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
