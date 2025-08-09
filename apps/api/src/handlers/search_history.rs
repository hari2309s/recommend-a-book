use crate::{
    error::ApiError, models::SearchHistoryRequest, services::search_history::SearchHistoryService,
};
use actix_web::{get, web, HttpResponse};
use uuid::Uuid;

#[get("/history")]
pub async fn get_search_history(
    params: web::Query<SearchHistoryRequest>,
    search_history_service: web::Data<SearchHistoryService>,
) -> Result<HttpResponse, ApiError> {
    match params.user_id.to_string().parse::<Uuid>() {
        Ok(user_id) => {
            let history = search_history_service
                .get_search_history(user_id, params.limit)
                .await?;

            Ok(HttpResponse::Ok().json(serde_json::json!({
                "history": history
            })))
        }
        Err(_) => Err(ApiError::InvalidInput("Invalid user ID format".to_string())),
    }
}
