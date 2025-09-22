use crate::{
    config,
    error::Result,
    ml::huggingface_embedder::HuggingFaceEmbedder,
    models::{Book, ErrorResponse, HealthResponse, RecommendationRequest, RecommendationResponse},
    routes::api_routes,
    services::{Pinecone, RecommendationService},
};
use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use anyhow::Context;
use log::info;
use std::net::TcpListener;

use utoipa::OpenApi;
use utoipa_swagger_ui::{Config as SwaggerConfig, SwaggerUi};

/// API Documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::health::health_check,
        crate::handlers::recommendations::get_recommendations,
        crate::handlers::prewarm::prewarm,
    ),
    components(
        schemas(
            Book,
            RecommendationRequest,
            RecommendationResponse,
            HealthResponse,
            ErrorResponse
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Recommendations", description = "Book recommendation endpoints"),
        (name = "System", description = "System management endpoints for performance optimization")
    ),
    info(
        title = "Book Recommendation API",
        version = "1.0.0",
        description = "A REST API for getting book recommendations using machine learning embeddings and vector similarity search.",
        contact(
            name = "API Support",
            email = "support@example.com"
        )
    ),
    servers(
        (url = "https://recommend-a-book-api.onrender.com", description = "Production server"),
        (url = "/", description = "Local development server")
    )
)]
pub struct ApiDoc;

pub struct Application {
    port: u16,
    host: String,
    config: config::Config,
}

impl Application {
    /// Create a new application instance
    pub fn new(config: &config::Config) -> Self {
        Self {
            port: config.port,
            host: config.host.clone(),
            config: config.clone(),
        }
    }

    /// Build and run the server
    pub async fn run(&self) -> Result<()> {
        // Always bind to 0.0.0.0 for Docker/Render compatibility
        let bind_address = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&bind_address)?;
        info!("Starting server at http://{}:{}", self.host, self.port);
        info!(
            "Swagger UI available at: http://{}:{}/swagger-ui/",
            self.host, self.port
        );

        self.run_with_listener(listener).await
    }

    /// Run the server with a specific TCP listener
    /// This is useful for testing where we want to use a random port
    /// Run the server with a specific TCP listener
    /// This is useful for testing where we want to use a random port
    ///
    /// The server is configured with optimized settings for production use
    pub async fn run_with_listener(&self, listener: TcpListener) -> Result<()> {
        // Initialize Pinecone client asynchronously
        let pinecone = Pinecone::new(
            &self.config.pinecone_api_key,
            &self.config.pinecone_environment,
            &self.config.pinecone_index,
        )
        .await
        .context("Failed to initialize Pinecone client")?;

        // Initialize ML model
        let sentence_encoder = HuggingFaceEmbedder::new()
            .await
            .context("Failed to initialize sentence encoder")?;

        // Create shareable recommendation service
        let recommendation_service =
            web::Data::new(RecommendationService::new(sentence_encoder, pinecone));

        // Create a new HTTP server with optimized configuration
        HttpServer::new(move || {
            // Configure CORS with optimized settings
            let cors = Cors::default()
                .allowed_origin("https://recommend-a-book-frontend.vercel.app")
                .allowed_origin("http://localhost:3000")
                .allowed_methods(vec!["GET", "POST", "OPTIONS"])
                .allowed_headers(vec!["Content-Type", "Accept"])
                .max_age(3600);

            // Configure Swagger UI
            let swagger_ui = SwaggerUi::new("/swagger-ui/{_:.*}")
                .config(SwaggerConfig::new(["/api-doc/openapi.json"]));

            // Create app with enhanced logging and performance settings
            App::new()
                .wrap(cors)
                // Use a more informative logger format for better debugging
                .wrap(Logger::new("%r %s %b %{User-Agent}i %D ms"))
                // Add request payload limit
                .app_data(web::JsonConfig::default().limit(1024 * 1024)) // 1MB limit
                .app_data(recommendation_service.clone())
                // Enable compression for responses
                .wrap(actix_web::middleware::Compress::default())
                // Add trailing slash handling
                .wrap(actix_web::middleware::NormalizePath::new(
                    actix_web::middleware::TrailingSlash::Trim,
                ))
                .service(api_routes())
                .route(
                    "/api-doc/openapi.json",
                    web::get().to(|| async { HttpResponse::Ok().json(ApiDoc::openapi()) }),
                )
                .service(swagger_ui)
        })
        .listen(listener)?
        // Configure server with worker settings for better performance
        .workers(std::cmp::max(2, num_cpus::get()))
        // Add backlog configuration for better connection handling
        .backlog(2048)
        // Increase max connection rate for better performance under load
        .max_connection_rate(256)
        // Add keep-alive timeout
        .keep_alive(std::time::Duration::from_secs(75))
        .run()
        .await?;

        Ok(())
    }
}
