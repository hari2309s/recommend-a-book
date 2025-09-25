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
    // Create a properly configured Swagger UI
    let mut config = SwaggerConfig::new(["/api-doc/openapi.json"]);

    // Configure the Swagger UI options
    config = config
        .try_it_out_enabled(true)
        .display_request_duration(true);

    // Return the configured Swagger UI
    SwaggerUi::new("/swagger-ui/{_:.*}").config(config)
}

/// Configure OpenAPI documentation JSON endpoint
pub fn openapi_route() -> actix_web::Resource {
    web::resource("/api-doc/openapi.json").route(web::get().to(|| async {
        // Return the OpenAPI document with proper CORS headers
        HttpResponse::Ok()
            .append_header(("Access-Control-Allow-Origin", "*"))
            .append_header(("Access-Control-Allow-Methods", "GET, OPTIONS"))
            .json(ApiDoc::openapi())
    }))
}

/// Redirect from /swagger-ui to /swagger-ui/ to handle missing trailing slash
pub fn swagger_redirect_route() -> actix_web::Resource {
    web::resource("/swagger-ui").route(web::get().to(|| async {
        HttpResponse::Found()
            .append_header(("Location", "/swagger-ui/"))
            .finish()
    }))
}
