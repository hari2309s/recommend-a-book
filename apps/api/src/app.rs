use crate::{
    config,
    error::Result,
    ml::huggingface_embedder::HuggingFaceEmbedder,
    models::{
        Book, BookRecommendation, ErrorResponse, HealthResponse, RecommendationRequest,
        RecommendationResponse,
    },
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
    ),
    components(
        schemas(
            Book,
            BookRecommendation,
            RecommendationRequest,
            RecommendationResponse,
            HealthResponse,
            ErrorResponse
        )
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Recommendations", description = "Book recommendation endpoints")
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

        let recommendation_service =
            web::Data::new(RecommendationService::new(sentence_encoder, pinecone));

        HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header();

            // Configure Swagger UI
            let swagger_ui = SwaggerUi::new("/swagger-ui/{_:.*}")
                .config(SwaggerConfig::new(["/api-doc/openapi.json"]));

            App::new()
                .wrap(cors)
                .wrap(Logger::default())
                .app_data(recommendation_service.clone())
                .service(api_routes())
                // Route to serve the OpenAPI JSON
                .route(
                    "/api-doc/openapi.json",
                    web::get().to(|| async { HttpResponse::Ok().json(ApiDoc::openapi()) }),
                )
                // Swagger UI service
                .service(swagger_ui)
        })
        .listen(listener)?
        .run()
        .await?;

        Ok(())
    }
}
