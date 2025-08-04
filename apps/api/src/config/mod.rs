use anyhow::Result;
use dotenvy::dotenv;
use std::env;
use supabase_rs::SupabaseClient;

pub struct Config {
    pub port: u16,
    pub supabase_url: String,
    pub supabase_key: String,
    pub pinecone_api_key: String,
    pub pinecone_index_name: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv().ok();

        Ok(Config {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            supabase_url: env::var("SUPABASE_URL").expect("SUPABASE_URL must be set"),
            supabase_key: env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set"),
            pinecone_api_key: env::var("PINECONE_API_KEY").expect("PINECONE_API_KEY must be set"),
            pinecone_index_name: env::var("PINECONE_INDEX_NAME")
                .expect("PINECONE_INDEX_NAME must be set"),
        })
    }

    pub fn supabase_client(&self) -> SupabaseClient {
        SupabaseClient::new(&self.supabase_url, &self.supabase_key)
    }

    pub async fn create_pinecone_client(&self) -> Result<pinecone_sdk::Client> {
        Ok(pinecone_sdk::Client::new(&self.pinecone_api_key).await?)
    }
}
