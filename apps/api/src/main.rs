use crate::error::{ApiError, Result};
use log::{error, info, warn};
use std::time::Duration;
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod config;
mod error;
mod handlers;
mod ml;
mod models;
mod routes;
mod services;

/// Maximum number of startup attempts
const MAX_STARTUP_ATTEMPTS: u8 = 3;

/// Delay between startup attempts in seconds
const STARTUP_RETRY_DELAY_SECS: u64 = 5;

#[actix_web::main]
async fn main() -> Result<()> {
    // Load configuration
    dotenv::dotenv().ok();

    // Setup logging with improved configuration
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // Default to info level if RUST_LOG is not set
                "recommend_a_book_api=info,actix_web=info,tokio=info,sqlx=warn".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting recommend-a-book API server...");

    // Load configuration with retry logic
    let config = load_configuration_with_retry().await?;

    // Create application with enhanced startup process
    info!("🔧 Initializing application services...");
    let application = app::Application::new(&config);

    // Run the application with retry logic for improved cold start handling
    for attempt in 1..=MAX_STARTUP_ATTEMPTS {
        info!(
            "🌐 Starting server (attempt {}/{})...",
            attempt, MAX_STARTUP_ATTEMPTS
        );

        match application.run().await {
            Ok(_) => return Ok(()),
            Err(e) if attempt < MAX_STARTUP_ATTEMPTS => {
                warn!(
                    "⚠️ Server startup attempt {} failed: {}. Retrying in {} seconds...",
                    attempt, e, STARTUP_RETRY_DELAY_SECS
                );
                sleep(Duration::from_secs(STARTUP_RETRY_DELAY_SECS)).await;
            }
            Err(e) => {
                error!("❌ All server startup attempts failed. Last error: {}", e);
                return Err(e);
            }
        }
    }

    // Should never reach here due to the loop structure, but Rust requires a return
    Ok(())
}

/// Attempts to load configuration with retry logic to handle temporary failures
async fn load_configuration_with_retry() -> Result<config::Config> {
    for attempt in 1..=MAX_STARTUP_ATTEMPTS {
        info!(
            "📋 Loading configuration (attempt {}/{})...",
            attempt, MAX_STARTUP_ATTEMPTS
        );

        match config::Config::load() {
            Ok(config) => {
                info!("✅ Configuration loaded successfully");
                return Ok(config);
            }
            Err(e) if attempt < MAX_STARTUP_ATTEMPTS => {
                warn!(
                    "⚠️ Failed to load configuration: {}. Retrying in {} seconds...",
                    e, STARTUP_RETRY_DELAY_SECS
                );
                sleep(Duration::from_secs(STARTUP_RETRY_DELAY_SECS)).await;
            }
            Err(e) => {
                error!(
                    "❌ All configuration loading attempts failed. Last error: {}",
                    e
                );
                return Err(ApiError::ExternalServiceError(e.to_string()));
            }
        }
    }

    // Should never reach here due to the loop structure
    Err(ApiError::ExternalServiceError(
        "Failed to load configuration after all attempts".to_string(),
    ))
}
