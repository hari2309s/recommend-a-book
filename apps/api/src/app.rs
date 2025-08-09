use crate::{
    config::Config,
    error::Result,
    routes::api_routes,
    services::{recommendation::RecommendationService, search_history::SearchHistoryService},
};
use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use log::info;
use std::net::TcpListener;

pub struct Application {
    port: u16,
    host: String,
}

impl Application {
    /// Create a new application instance
    pub fn new(config: &Config) -> Self {
        Self {
            port: config.port,
            host: config.host.clone(),
        }
    }

    /// Build and run the server
    pub async fn run(&self) -> Result<()> {
        let address = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&address)?;
        info!("Starting server at http://{}", address);

        self.run_with_listener(listener).await
    }

    /// Run the server with a specific TCP listener
    /// This is useful for testing where we want to use a random port
    pub async fn run_with_listener(&self, listener: TcpListener) -> Result<()> {
        // Initialize services
        let supabase = services::supabase::SupabaseClient::new(
            &self.config.supabase_url,
            &self.config.supabase_key,
        );

        let recommendation_service = web::Data::new(RecommendationService::new(supabase.clone()));
        let search_history_service = web::Data::new(SearchHistoryService::new(supabase));

        HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin()
                .allow_any_method()
                .allow_any_header();

            App::new()
                .wrap(cors)
                .wrap(Logger::default())
                .app_data(recommendation_service.clone())
                .app_data(search_history_service.clone())
                .service(api_routes())
        })
        .listen(listener)?
        .run()
        .await?;

        Ok(())
    }
}
