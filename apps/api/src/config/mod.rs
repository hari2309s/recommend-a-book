use anyhow::Result;
use config::{Config as ConfigFile, Environment, File};
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
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = ConfigFile::builder()
            // Start with base config
            .add_source(File::from(PathBuf::from("config/base.toml")).required(false))
            // Add environment specific config
            .add_source(
                File::from(PathBuf::from(format!("config/{}.toml", run_mode))).required(false),
            )
            // Add local overrides
            .add_source(File::from(PathBuf::from("config/local.toml")).required(false))
            // Add environment variables with prefix "APP_"
            .add_source(Environment::with_prefix("APP").separator("_"))
            .build()?;

        let mut config: Config = s.try_deserialize()?;

        // Override with environment variables if they exist
        if let Ok(port) = env::var("PORT") {
            config.port = port.parse()?;
        }

        Ok(config)
    }
}
