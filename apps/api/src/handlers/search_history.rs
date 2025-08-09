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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use uuid::Uuid;

    #[actix_web::test]
    async fn test_get_search_history_invalid_user_id() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(SearchHistoryService::mock()))
                .service(get_search_history),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/history?user_id=invalid-uuid")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_get_search_history_success() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(SearchHistoryService::mock()))
                .service(get_search_history),
        )
        .await;

        let user_id = Uuid::new_v4();
        let req = test::TestRequest::get()
            .uri(&format!("/history?user_id={}&limit=10", user_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["history"].is_array());
    }
}
