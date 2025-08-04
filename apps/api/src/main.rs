use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

mod config;
mod handlers;
mod models;
mod services;
mod error;
mod ml;

use config::Config;
use handlers::recommendation_handlers;
use services::{RecommendationService, SearchHistoryService};
use ml::sentence_encoder::SentenceEncoder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::init();

    // Load configuration
    let config = Config::from_env()?;

    // Initialize ML model
    tracing::info!("Loading Universal Sentence Encoder model...");
    let model = SentenceEncoder::load().await?;
    tracing::info!("Universal Sentence Encoder model loaded successfully");

    // Initialize Pinecone
    let pinecone_client = config.create_pinecone_client().await?;
    let pinecone_index = pinecone_client.index(&config.pinecone_index_name);

    // Initialize services
    let recommendation_service = Arc::new(RecommendationService::new(model, pinecone_index));
    let search_history_service = Arc::new(SearchHistoryService::new(config.supabase_client()));

    // Setup routes
    let app = Router::new()
        .route("/api/recommend", post(recommendation_handlers::get_recommendations))
        .route("/api/history", get(recommendation_handlers::get_search_history))
        .layer(CorsLayer::permissive())
        .with_state((recommendation_service, search_history_service));

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("Server running on port {}", config.port);

    axum::serve(listener, app).await?;

    Ok(())
}
