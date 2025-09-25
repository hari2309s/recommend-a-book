use crate::{
    config,
    error::Result,
    ml::huggingface_embedder::HuggingFaceEmbedder,
    models::{Book, ErrorResponse, HealthResponse, RecommendationRequest, RecommendationResponse},
    routes::{api_routes, openapi_route, swagger_redirect_route, swagger_routes},
    services::{Pinecone, RecommendationService},
};
use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use log::{error, info, warn};
use std::net::TcpListener;

use utoipa::OpenApi;

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
        info!("Initializing services with optimized cold start configuration");

        // Initialize service dependencies concurrently to reduce startup time
        let (pinecone_result, sentence_encoder_result) = tokio::join!(
            // Initialize Pinecone client asynchronously with timeout protection
            async {
                let pinecone_future = Pinecone::new(
                    &self.config.pinecone_api_key,
                    &self.config.pinecone_environment,
                    &self.config.pinecone_index,
                );

                match tokio::time::timeout(std::time::Duration::from_secs(30), pinecone_future)
                    .await
                {
                    Ok(result) => result,
                    Err(_) => {
                        warn!("Pinecone initialization timed out after 30s. Will retry on first request");
                        // Return a fallback implementation that will try to initialize on first request
                        Pinecone::new_with_lazy_init(
                            &self.config.pinecone_api_key,
                            &self.config.pinecone_environment,
                            &self.config.pinecone_index,
                        )
                    }
                }
            },
            // Initialize ML model with timeout protection
            async {
                let encoder_future = HuggingFaceEmbedder::new();
                match tokio::time::timeout(std::time::Duration::from_secs(30), encoder_future).await
                {
                    Ok(result) => result,
                    Err(_) => {
                        warn!("HuggingFace initialization timed out after 30s. Will retry on first request");
                        HuggingFaceEmbedder::new_with_deferred_init()
                    }
                }
            }
        );

        // Handle initialization results
        let pinecone = pinecone_result?;
        let sentence_encoder = sentence_encoder_result?;

        // Create shareable recommendation service with optimized configuration
        let recommendation_service =
            web::Data::new(RecommendationService::new(sentence_encoder, pinecone));

        // Start background prewarmer in non-blocking way
        let rs_clone = recommendation_service.clone();
        tokio::spawn(async move {
            info!("Starting background prewarm process");
            if let Err(e) = rs_clone.prewarm().await {
                warn!("Background prewarming encountered an error: {}", e);
            } else {
                info!("Background prewarming completed successfully");
            }
        });

        // Create a new HTTP server with optimized configuration
        HttpServer::new(move || {
            // Configure CORS with optimized settings
            let cors = Cors::default()
                .allowed_origin("https://recommend-a-book-frontend.vercel.app")
                .allowed_origin("http://localhost:3000")
                .allowed_origin("http://127.0.0.1:3000")
                .allowed_methods(vec!["GET", "POST", "OPTIONS", "HEAD", "PUT", "DELETE"])
                .allowed_headers(vec!["Content-Type", "Accept", "Authorization", "X-Requested-With", "X-Prewarm-Source"])
                .expose_headers(vec!["content-disposition", "Content-Length"])
                .supports_credentials()
                .max_age(3600);

            // Import Swagger UI from routes
            let swagger_ui = swagger_routes();

            // Configure app with enhanced logging and performance settings
            App::new()
                .wrap(cors)
                // Use a more informative logger format for better debugging
                .wrap(Logger::new("%r %s %b %{User-Agent}i %D ms"))
                // Add request payload limit
                .app_data(web::JsonConfig::default().limit(1024 * 1024)) // 1MB limit
                // Add increased timeout for cold starts
                .app_data(web::Data::new(
                    web::JsonConfig::default()
                        .limit(1024 * 1024)
                        .error_handler(|err, _req| {
                            error!("JSON payload error: {}", err);
                            let err_str = err.to_string();
                            actix_web::error::InternalError::from_response(
                                err,
                                HttpResponse::BadRequest()
                                    .content_type("application/json")
                                    .json(serde_json::json!({"error": err_str})),
                            )
                            .into()
                        }),
                ))
                .app_data(recommendation_service.clone())
                // Enable compression for responses
                .wrap(actix_web::middleware::Compress::default())
                // Add path normalization without affecting trailing slashes (for Swagger UI compatibility)
                .wrap(actix_web::middleware::NormalizePath::new(
                    actix_web::middleware::TrailingSlash::MergeOnly,
                ))
                // Add security and CORS headers
                .wrap(
                    actix_web::middleware::DefaultHeaders::new()
                        .add(("X-Content-Type-Options", "nosniff"))
                        .add((
                            "Strict-Transport-Security",
                            "max-age=31536000; includeSubDomains",
                        ))
                        .add(("Access-Control-Allow-Origin", "*"))
                        .add(("Access-Control-Allow-Methods", "GET, POST, OPTIONS, PUT, DELETE"))
                        .add(("Access-Control-Allow-Headers", "Content-Type, Accept, Authorization, X-Requested-With, X-Prewarm-Source"))
                        .add(("Access-Control-Max-Age", "3600")),
                )
                .service(swagger_ui)
                .service(openapi_route())
                .service(swagger_redirect_route())
                .service(api_routes())
        })
        .listen(listener)?
        // Configure server with worker settings for better performance
        .workers(std::cmp::max(2, num_cpus::get()))
        // Add backlog configuration for better connection handling
        .backlog(2048)
        // Increase max connection rate for better performance under load
        .max_connection_rate(256)
        // Add keep-alive timeout for better connection reuse
        .keep_alive(std::time::Duration::from_secs(75))
        // Add client timeout to prevent requests from hanging indefinitely
        .client_request_timeout(std::time::Duration::from_secs(60))
        // Set graceful shutdown timeout
        .shutdown_timeout(5)
        .run()
        .await?;

        Ok(())
    }
}
