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
    pub huggingface_api_key: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        // Set required environment variables for test
        env::set_var("APP_HOST", "127.0.0.1");
        env::set_var("APP_PORT", "3000");
        env::set_var("APP_ENVIRONMENT", "test");
        env::set_var("APP_FRONTEND_URL", "http://localhost:3000");
        env::set_var("APP_DATABASE_URL", "postgres://localhost:5432/test");
        env::set_var("APP_SUPABASE_URL", "https://test.supabase.co");
        env::set_var("APP_SUPABASE_KEY", "test-key");
        env::set_var("APP_PINECONE_API_KEY", "pinecone-test-key");
        env::set_var("APP_PINECONE_ENVIRONMENT", "us-east-1");
        env::set_var("APP_PINECONE_INDEX", "test-index");
        env::set_var("APP_HUGGINGFACE_API_KEY", "hf-test-key");

        let config = Config::load().unwrap();

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert_eq!(config.environment, "test");
        assert_eq!(config.frontend_url, "http://localhost:3000");
        assert_eq!(config.database_url, "postgres://localhost:5432/test");
        assert_eq!(config.supabase_url, "https://test.supabase.co");
        assert_eq!(config.supabase_key, "test-key");
        assert_eq!(config.pinecone_api_key, "pinecone-test-key");
        assert_eq!(config.pinecone_environment, "us-east-1");
        assert_eq!(config.pinecone_index, "test-index");
        assert_eq!(config.huggingface_api_key, "hf-test-key");
    }

    #[test]
    fn test_port_override() {
        env::set_var("PORT", "4000");
        let config = Config::load().unwrap();
        assert_eq!(config.port, 4000);
    }
}
