use anyhow::Result;
use ndarray::{Array1, ArrayView1};
use reqwest::{header::HeaderMap, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PineconeClient {
    client: Client,
    api_key: String,
    index_name: String,
    base_url: String,
    environment: String,
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

        let base_url = format!("https://{}-{}.svc.pinecone.io", index_name, &api_key[..8]);

        Self {
            client,
            api_key: api_key.to_string(),
            index_name: index_name.to_string(),
            base_url,
            environment: "us-east1-gcp".to_string(),
        }
    }

    pub async fn query_metadata(
        &self,
        field: &str,
        value: &str,
        exact_match: bool,
        top_k: usize,
    ) -> Result<Vec<crate::models::Book>> {
        let mut filter = HashMap::new();
        filter.insert(
            field.to_string(),
            if exact_match {
                value.to_string()
            } else {
                format!("%{}%", value)
            },
        );

        let request = QueryRequest {
            namespace: None,
            vector: vec![0.0; 512], // Dummy vector since we're only using metadata
            top_k,
            filter: Some(filter),
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
            anyhow::bail!("Pinecone metadata query failed: {}", error_text);
        }

        let query_response: QueryResponse = response.json().await?;
        let books = query_response
            .matches
            .into_iter()
            .filter_map(|m| m.metadata.and_then(|md| serde_json::from_value(md).ok()))
            .collect();

        Ok(books)
    }

    pub async fn query_vector(
        &self,
        embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<crate::models::Book>> {
        let request = QueryRequest {
            namespace: None,
            vector: embedding.to_vec(),
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
            anyhow::bail!("Pinecone vector query failed: {}", error_text);
        }

        let query_response: QueryResponse = response.json().await?;
        let books = query_response
            .matches
            .into_iter()
            .filter_map(|m| m.metadata.and_then(|md| serde_json::from_value(md).ok()))
            .collect();

        Ok(books)
    }
}
