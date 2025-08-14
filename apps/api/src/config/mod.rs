use anyhow::Result;
use config::{Config as ConfigFile, Environment, File, Source};
use serde::Deserialize;
use std::{env, path::PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub environment: String,
    pub frontend_url: String,
    pub database_url: String,
    pub supabase_url: String,
    pub supabase_key: String,
    pub pinecone_api_key: String,
    pub pinecone_environment: String,
    pub pinecone_index: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        use tracing::{info, debug, warn};
        
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        info!("Loading configuration for environment: {}", run_mode);

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
        debug!("Configuration sources:");
        if let Ok(sources) = debug_config.collect() {
            for (key, _) in sources.iter() {
                if key.starts_with("pinecone") {
                    debug!("  {} = {:?}", key, debug_config.get::<String>(key));
                }
            }
        }
        
        // Build the final config
        let mut config: Config = config_builder.build()?.try_deserialize()?;
        
        // Override with environment variables if they exist
        if let Ok(port) = env::var("PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                info!("Using port from PORT environment variable: {}", port_num);
                config.port = port_num;
            } else {
                warn!("Invalid PORT environment variable value: {}", port);
            }
        } else {
            info!("Using default port from config: {}", config.port);
        }
        
        // Log final Pinecone configuration
        debug!("Final Pinecone configuration - Environment: {}, Index: {}", 
               config.pinecone_environment, config.pinecone_index);

        Ok(config)
    }
}
