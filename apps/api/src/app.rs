use crate::{
    config::Config,
    error::Result,
    ml::huggingface_embedder::HuggingFaceEmbedder,
    routes::api_routes,
    services::{Pinecone, RecommendationService},
};
use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use anyhow::Context;
use log::info;
use std::net::TcpListener;

pub struct Application {
    port: u16,
    host: String,
    config: Config,
}

impl Application {
    /// Create a new application instance
    pub fn new(config: &Config) -> Self {
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

            App::new()
                .wrap(cors)
                .wrap(Logger::default())
                .app_data(recommendation_service.clone())
                .service(api_routes())
        })
        .listen(listener)?
        .run()
        .await?;

        Ok(())
    }
}
