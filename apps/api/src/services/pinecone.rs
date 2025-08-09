use anyhow::Result;
use ndarray::Array1;
use reqwest::{header::HeaderMap, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PineconeClient {
    client: Client,
    api_key: String,
    index_name: String,
    base_url: String,
}

#[derive(Debug, Serialize)]
pub struct QueryRequest {
    pub namespace: Option<String>,
    pub vector: Vec<f32>,
    pub top_k: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<HashMap<String, String>>,
    pub include_values: bool,
    pub include_metadata: bool,
}

#[derive(Debug, Deserialize)]
pub struct QueryMatch {
    pub id: String,
    pub score: f32,
    pub values: Option<Vec<f32>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub matches: Vec<QueryMatch>,
    pub namespace: String,
}

#[derive(Debug, Serialize)]
pub struct UpsertRequest {
    pub vectors: Vec<Vector>,
    pub namespace: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vector {
    pub id: String,
    pub values: Vec<f32>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct DeleteRequest {
    pub ids: Vec<String>,
    pub namespace: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FetchRequest {
    pub ids: Vec<String>,
    pub namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FetchResponse {
    pub namespace: String,
    pub vectors: HashMap<String, Vector>,
}

#[derive(Debug, Deserialize)]
pub struct IndexStats {
    pub dimension: usize,
    pub index_fullness: f32,
    pub namespaces: HashMap<String, NamespaceStats>,
    pub total_vector_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct NamespaceStats {
    pub vector_count: usize,
}

impl PineconeClient {
    pub fn new(api_key: &str, index_name: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Api-Key", api_key.parse().unwrap());
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");

        let base_url = format!("https://{}-xxxxx.svc.environment.pinecone.io", index_name);

        Self {
            client,
            api_key: api_key.to_string(),
            index_name: index_name.to_string(),
            base_url,
        }
    }

    pub async fn query(&self, vector: Array1<f32>, top_k: usize) -> Result<QueryResponse> {
        let request = QueryRequest {
            namespace: None,
            vector: vector.to_vec(),
            top_k,
            filter: None,
            include_values: false,
            include_metadata: true,
        };

        let response = self
            .client
            .post(format!("{}/query", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Pinecone query failed: {}", error_text);
        }

        let query_response = response.json().await?;
        Ok(query_response)
    }

    pub async fn upsert(&self, vectors: Vec<Vector>) -> Result<()> {
        let request = UpsertRequest {
            vectors,
            namespace: None,
        };

        let response = self
            .client
            .post(format!("{}/vectors/upsert", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Pinecone upsert failed: {}", error_text);
        }

        Ok(())
    }

    pub async fn delete_vectors(&self, ids: Vec<String>) -> Result<()> {
        let request = DeleteRequest {
            ids,
            namespace: None,
        };

        let response = self
            .client
            .post(format!("{}/vectors/delete", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Pinecone delete failed: {}", error_text);
        }

        Ok(())
    }

    pub async fn fetch_vectors(&self, ids: Vec<String>) -> Result<FetchResponse> {
        let request = FetchRequest {
            ids,
            namespace: None,
        };

        let response = self
            .client
            .post(format!("{}/vectors/fetch", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Pinecone fetch failed: {}", error_text);
        }

        let fetch_response = response.json().await?;
        Ok(fetch_response)
    }

    pub async fn describe_index_stats(&self) -> Result<IndexStats> {
        let response = self
            .client
            .post(format!("{}/describe_index_stats", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Pinecone describe index stats failed: {}", error_text);
        }

        let stats = response.json().await?;
        Ok(stats)
    }
}
