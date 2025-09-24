//! Main recommendation service implementation.
//!
//! This service integrates intent detection, search strategies, and result ranking
//! to provide book recommendations based on user queries.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

use log::{debug, info, warn};
// Removed unused import: uuid::Uuid

use crate::{
    error::{ApiError, Result},
    ml::huggingface_embedder::HuggingFaceEmbedder,
    models::{Book, BookRecommendation},
    services::pinecone::Pinecone,
    // Removed unused import: RetryConfig
};

use super::{
    cache::{RecommendationCache, RecommendationCacheKey},
    intent::{self, IntentCache, QueryIntent},
    ranking,
    search::{self, SearchStrategy},
};

// Constants for the recommendation service
const INTENT_CACHE_TTL_SECONDS: u64 = 3600; // 1 hour
const DEFAULT_TOP_K: usize = 10;
const DIVERSITY_FACTOR: f32 = 0.2; // 20% penalty for duplicate authors
const PREWARM_QUERIES: &[&str] = &[
    "fantasy books",
    "science fiction recommendations",
    "best mystery novels",
    "books similar to Harry Potter",
    "books by Stephen King",
    "historical fiction",
    "non-fiction books about science",
];

/// Main recommendation service responsible for processing queries and returning book recommendations
pub struct RecommendationService {
    /// Sentence encoder for converting text to vector embeddings
    sentence_encoder: Arc<HuggingFaceEmbedder>,

    /// Pinecone vector database client for semantic search
    pinecone: Pinecone,

    /// Cache for recommendation results to improve performance
    result_cache: Arc<RecommendationCache>,

    /// Cache for parsed query intents to avoid repeated parsing
    intent_cache: Arc<IntentCache>,

    /// Flag to track whether the service has been prewarmed
    prewarmed: Arc<AtomicBool>,
}

impl RecommendationService {
    /// Create a new recommendation service instance
    pub fn new(sentence_encoder: HuggingFaceEmbedder, pinecone: Pinecone) -> Self {
        Self {
            sentence_encoder: Arc::new(sentence_encoder),
            pinecone,
            result_cache: Arc::new(RecommendationCache::new()),
            intent_cache: Arc::new(IntentCache::new(INTENT_CACHE_TTL_SECONDS)),
            prewarmed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the service has been prewarmed
    pub fn is_prewarmed(&self) -> bool {
        self.prewarmed.load(Ordering::SeqCst)
    }

    /// Warms up the recommendation service to mitigate cold start issues
    ///
    /// This is important for ML models and database connections that may have
    /// slow initialization times on first request.
    pub async fn prewarm(&self) -> Result<()> {
        // Skip if already prewarmed
        if self.prewarmed.load(Ordering::SeqCst) {
            info!("Recommendation service already prewarmed, skipping");
            return Ok(());
        }

        let start = Instant::now();
        info!("Prewarming recommendation service...");

        // Prewarm the sentence encoder
        info!("Prewarming sentence encoder...");
        self.sentence_encoder.prewarm().await.map_err(|e| {
            ApiError::model_inference_error(format!("Failed to prewarm sentence encoder: {}", e))
                .with_context("recommendation_service")
                .with_operation("prewarm")
        })?;

        // Prewarm the vector database with some common queries
        info!("Prewarming vector database with common queries...");
        let mut success_count = 0;

        for &query in PREWARM_QUERIES {
            match self.get_recommendations(query, DEFAULT_TOP_K).await {
                Ok(_) => {
                    debug!("Successfully prewarmed with query: '{}'", query);
                    success_count += 1;
                }
                Err(e) => {
                    warn!("Failed to prewarm with query '{}': {}", query, e);
                }
            }
        }

        let elapsed = start.elapsed();
        info!(
            "Recommendation service prewarmed in {:.2?} ({}/{} queries successful)",
            elapsed,
            success_count,
            PREWARM_QUERIES.len()
        );

        // Mark as prewarmed
        self.prewarmed.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Get book recommendations based on a text query
    ///
    /// This is the main entry point for the recommendation service that:
    /// 1. Parses the query intent (author, genre, similarity)
    /// 2. Retrieves or computes recommendations based on the intent
    /// 3. Ranks and returns the top results
    pub async fn get_recommendations(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<BookRecommendation>> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return Err(ApiError::invalid_input("Query cannot be empty")
                .with_context("recommendation_service")
                .with_operation("get_recommendations"));
        }

        // Check cache for existing results
        let cache_key = RecommendationCacheKey::new(trimmed_query, top_k);

        if let Some(cached_results) = self.result_cache.get(&cache_key) {
            info!("CACHE HIT for query: {}", trimmed_query);

            // Convert cached books to recommendations
            return Ok(cached_results
                .into_iter()
                .enumerate()
                .map(|(i, book)| BookRecommendation {
                    id: format!("rec_{}", i),
                    book,
                    similarity_score: 1.0 - (i as f32 * 0.05).min(0.5), // Descending scores
                })
                .collect());
        }

        info!("CACHE MISS for query: {}", trimmed_query);

        // Parse query intent with caching
        let intent = self.get_cached_intent(trimmed_query)?;
        info!("Detected intent: {:?}", intent);

        // Get search strategy based on intent
        let strategy = search::get_search_strategy(&intent);
        info!("Using search strategy: {:?}", strategy);

        // Increase search scope to get more candidates for ranking
        let expanded_k = top_k * 3;

        // Perform search with better error handling
        let raw_results = self.perform_search(&intent, &strategy, expanded_k).await?;

        if raw_results.is_empty() {
            info!(
                "No results found for query: '{}', trying fallback search",
                trimmed_query
            );

            // Try fallback search if primary search returns no results
            let fallback_results = self
                .perform_fallback_search(trimmed_query, expanded_k)
                .await?;

            if fallback_results.is_empty() {
                info!("Fallback search also returned no results");
                return Ok(Vec::new());
            }

            // Rank fallback results
            let recommendations =
                ranking::rank_results(fallback_results, &intent, &strategy, top_k)?;

            // Cache the raw books for future queries
            let books = recommendations.iter().map(|rec| rec.book.clone()).collect();
            if let Err(e) = self.result_cache.set(cache_key, books) {
                warn!("Failed to cache results: {}", e);
            }

            return Ok(recommendations);
        }

        // Rank and return top results
        let mut recommendations = ranking::rank_results(raw_results, &intent, &strategy, top_k)?;

        // Apply diversity boosting to avoid too many books by the same author
        ranking::apply_diversity_boost(&mut recommendations, DIVERSITY_FACTOR);

        // Cache the raw books for future queries
        let books = recommendations.iter().map(|rec| rec.book.clone()).collect();
        if let Err(e) = self.result_cache.set(cache_key, books) {
            warn!("Failed to cache results: {}", e);
        }

        Ok(recommendations)
    }

    /// Get a cached query intent or parse a new one
    fn get_cached_intent(&self, query: &str) -> Result<QueryIntent> {
        // Try to get from cache first
        if let Some(intent) = self.intent_cache.get(query) {
            return Ok(intent);
        }

        // Parse new intent
        let intent = intent::parse_query_intent_optimized(query);

        // Store in cache
        if let Err(e) = self.intent_cache.set(query.to_string(), intent.clone()) {
            warn!("Failed to cache query intent: {}", e);
        }

        Ok(intent)
    }

    /// Perform a search based on the query intent and strategy
    async fn perform_search(
        &self,
        intent: &QueryIntent,
        strategy: &SearchStrategy,
        top_k: usize,
    ) -> Result<Vec<(Book, f32)>> {
        // Get the query text based on intent type
        let query_text = match intent {
            QueryIntent::Author { original_query, .. } => original_query,
            QueryIntent::Genre { original_query, .. } => original_query,
            QueryIntent::SimilarTo { original_query } => original_query,
            QueryIntent::General { query } => query,
        };

        // Encode the query text to a vector embedding
        info!("Encoding query: '{}'", query_text);
        let query_embedding = self
            .sentence_encoder
            .encode(query_text)
            .await
            .map_err(|e| {
                ApiError::model_inference_error(format!("Failed to encode query: {}", e))
                    .with_context("recommendation_service")
                    .with_operation("perform_search")
            })?;

        // Prepare filter if specified in strategy
        let _filter = strategy.metadata_filter.as_ref().map(|filter| {
            serde_json::json!({
                filter.field.clone(): {
                    if filter.exact_match { "$eq" } else { "$contains" }: filter.value.clone()
                }
            })
        });

        // Perform search using our search module
        let results =
            search::perform_hybrid_search(&self.pinecone, &query_embedding, strategy, top_k)
                .await
                .map_err(|e| {
                    ApiError::external_service_error(format!("Search failed: {}", e))
                        .with_context("recommendation_service")
                        .with_operation("perform_search")
                })?;

        info!("Search returned {} results", results.len());
        Ok(results)
    }

    /// Perform a fallback search when the primary search returns no results
    async fn perform_fallback_search(&self, query: &str, top_k: usize) -> Result<Vec<(Book, f32)>> {
        // For fallback, we use a direct text search or broader filters
        info!("Performing fallback search for query: '{}'", query);

        // Use the search module's fallback search
        let results = search::perform_fallback_search(&self.pinecone, query, top_k)
            .await
            .map_err(|e| {
                ApiError::external_service_error(format!("Fallback search failed: {}", e))
                    .with_context("recommendation_service")
                    .with_operation("fallback_search")
            })?;

        info!("Fallback search returned {} results", results.len());

        Ok(results)
    }

    /// Clean up expired entries in caches
    #[allow(dead_code)]
    pub fn cleanup_caches(&self) -> Result<()> {
        // Clean up result cache
        if let Err(e) = self.result_cache.cleanup() {
            warn!("Failed to clean up result cache: {}", e);
        }

        // Clean up intent cache
        if let Err(e) = self.intent_cache.cleanup() {
            warn!("Failed to clean up intent cache: {}", e);
        }

        Ok(())
    }
}
