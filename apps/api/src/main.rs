use crate::error::Result;
use log::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod config;
mod error;
mod handlers;
mod ml;
mod models;
mod routes;
mod services;

#[actix_web::main]
async fn main() -> Result<()> {
    // Load configuration
    dotenv::dotenv().ok();

    // Setup logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // Default to info level if RUST_LOG is not set
                "recommend_a_book_api=info,actix_web=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Loading configuration...");
    let config = config::Config::load()?;

    // Create and run application
    let application = app::Application::new(&config);
    application.run().await
}
