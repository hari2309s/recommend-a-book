use actix_web::{web, Scope};

use crate::handlers::{health_check, recommendations_config};

/// Configure all routes for the API
pub fn api_routes() -> Scope {
    web::scope("/api")
        .service(health_check)
        .configure(recommendations_config)
}
