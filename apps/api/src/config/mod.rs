use anyhow::Result;
use config::{Config as ConfigFile, Environment, File, Source};
use serde::Deserialize;
use std::{env, path::PathBuf};

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub supabase_url: String,
    pub supabase_key: String,
    pub pinecone_api_key: String,
    pub pinecone_environment: String,
    pub pinecone_index: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        use tracing::{debug, info, warn};

        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        info!("Loading configuration for environment: {}", run_mode);
        debug!("Starting configuration loading process");

        let config_builder = ConfigFile::builder()
            // Start with base config
            .add_source(File::from(PathBuf::from("config/base.toml")).required(false))
            // Add environment specific config
            .add_source(
                File::from(PathBuf::from(format!("config/{}.toml", run_mode))).required(false),
            )
            // Add local overrides
            .add_source(File::from(PathBuf::from("config/local.toml")).required(false))
            // Add environment variables with prefix "APP_"
            .add_source(Environment::with_prefix("APP").separator("_"));

        // Clone the config for debugging before consuming it
        let debug_config = config_builder.build_cloned()?;

        // Log all configuration sources for debugging
        debug!("Configuration sources loaded from files and environment:");
        if let Ok(sources) = debug_config.collect() {
            for (key, _) in sources.iter() {
                if key.starts_with("pinecone")
                    || key.starts_with("supabase")
                    || key == "host"
                    || key == "port"
                {
                    let value = debug_config.get::<String>(key).unwrap_or_default();
                    let display_value = if key.contains("key") || key.contains("api") {
                        if value.len() > 10 {
                            format!(
                                "{}...{} (length: {})",
                                &value[0..4],
                                &value[value.len() - 4..],
                                value.len()
                            )
                        } else if !value.is_empty() {
                            "[non-empty value]".to_string()
                        } else {
                            "[empty]".to_string()
                        }
                    } else {
                        value
                    };
                    debug!("  {} = {}", key, display_value);
                }
            }
        }

        // Build the final config
        let mut config: Config = config_builder.build()?.try_deserialize()?;

        // Override with environment variables if they exist
        if let Ok(port) = env::var("APP_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                info!(
                    "Using port from APP_PORT environment variable: {}",
                    port_num
                );
                config.port = port_num;
            } else {
                warn!("Invalid APP_PORT environment variable value: {}", port);
            }
        } else {
            info!("Using default port from config: {}", config.port);
        }

        // Override sensitive values with environment variables
        if let Ok(value) = env::var("APP_SUPABASE_URL") {
            info!("Using Supabase URL from environment variable");
            config.supabase_url = value;
        }

        if let Ok(value) = env::var("APP_SUPABASE_KEY") {
            info!("Using Supabase key from environment variable");
            config.supabase_key = value;
        }

        if let Ok(value) = env::var("APP_PINECONE_API_KEY") {
            let display_value = if value.len() > 10 {
                format!(
                    "{}...{} (length: {})",
                    &value[0..4],
                    &value[value.len() - 4..],
                    value.len()
                )
            } else {
                "[redacted]".to_string()
            };
            info!(
                "Using Pinecone API key from environment variable: {}",
                display_value
            );
            config.pinecone_api_key = value;
        } else {
            debug!("APP_PINECONE_API_KEY not found in environment, using value from config file");
        }

        if let Ok(value) = env::var("APP_PINECONE_ENV") {
            info!(
                "Using Pinecone environment from environment variable: '{}'",
                value
            );
            config.pinecone_environment = value;
        } else {
            debug!(
                "APP_PINECONE_ENV not found in environment, using value from config file: '{}'",
                config.pinecone_environment
            );
        }

        if let Ok(value) = env::var("APP_PINECONE_INDEX_NAME") {
            info!(
                "Using Pinecone index from environment variable: '{}'",
                value
            );
            config.pinecone_index = value;
        } else {
            debug!(
                "APP_PINECONE_INDEX_NAME not found in environment, using value from config file: '{}'",
                config.pinecone_index
            );
        }

        // Validate configuration values
        if config.pinecone_api_key.is_empty() || config.pinecone_api_key.contains("your") {
            warn!("Pinecone API key appears to be invalid or empty");
        }

        if config.pinecone_environment.is_empty() || config.pinecone_environment.contains("your") {
            warn!(
                "Pinecone environment appears to be invalid or empty: '{}'",
                config.pinecone_environment
            );
        }

        if config.pinecone_index.is_empty() || config.pinecone_index.contains("your") {
            warn!(
                "Pinecone index appears to be invalid or empty: '{}'",
                config.pinecone_index
            );
        }

        // Log final Pinecone configuration
        info!(
            "Final Pinecone configuration - Environment: '{}', Index: '{}'",
            config.pinecone_environment, config.pinecone_index
        );

        debug!(
            "Expected Pinecone URL will be: https://{}.svc.{}.pinecone.io",
            config.pinecone_index, config.pinecone_environment
        );

        Ok(config)
    }
}
