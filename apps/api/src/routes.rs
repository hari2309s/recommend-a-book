use actix_web::{web, HttpResponse, Scope};
use utoipa::OpenApi;
use utoipa_swagger_ui::{Config as SwaggerConfig, SwaggerUi};

use crate::app::ApiDoc;
use crate::handlers::{
    health_check, health_options, prewarm_endpoint, prewarm_options, recommendations_config,
};

/// Configure all routes for the API
pub fn api_routes() -> Scope {
    web::scope("/api")
        .service(health_check)
        .service(health_options)
        .service(prewarm_endpoint)
        .service(prewarm_options)
        .configure(recommendations_config)
}

/// Configure Swagger UI routes
pub fn swagger_routes() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui/{_:.*}").config(SwaggerConfig::new(["/api-docs/openapi.json"]))
}

/// Configure OpenAPI documentation JSON endpoint
pub fn openapi_route() -> actix_web::Resource {
    web::resource("/api-docs/openapi.json")
        .route(web::get().to(|| async {
            // Return the OpenAPI document
            HttpResponse::Ok()
                .append_header(("Content-Type", "application/json"))
                .json(ApiDoc::openapi())
        }))
        .route(
            web::route()
                .method(actix_web::http::Method::OPTIONS)
                .to(|| async {
                    // Handle OPTIONS requests
                    HttpResponse::Ok().finish()
                }),
        )
}

/// Redirect from /swagger-ui to /swagger-ui/ to handle missing trailing slash
pub fn swagger_redirect_route() -> actix_web::Resource {
    web::resource("/swagger-ui").route(web::get().to(|| async {
        HttpResponse::Found()
            .append_header(("Location", "/swagger-ui/"))
            .finish()
    }))
}
