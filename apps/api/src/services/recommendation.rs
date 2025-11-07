use crate::error::Result;
use crate::services::semantic_classifier::{SemanticClassifier, SemanticQueryInfo};
use crate::services::QueryEnhancer;
use crate::{
    error::ApiError, ml::huggingface_embedder::HuggingFaceEmbedder, models::Book,
    services::pinecone::Pinecone,
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum QueryIntent {
    Author {
        name: String,
        original_query: String,
    },
    Genre {
        genre: String,
        original_query: String,
    },
    SimilarTo {
        original_query: String,
    },
    General {
        query: String,
    },
}

#[derive(Debug)]
struct SearchStrategy {
    metadata_filter: Option<MetadataFilter>,
    semantic_weight: f32,
    hybrid_search: bool,
}

#[derive(Debug, Serialize)]
struct MetadataFilter {
    field: String,
    value: String,
    exact_match: bool,
}

// Cache entry for query results to avoid repeated computation
struct CacheEntry {
    results: Vec<Book>,
    timestamp: Instant,
}

// Cache duration in seconds
const CACHE_TTL_SECONDS: u64 = 300; // 5 minutes

#[derive(Clone)]
pub struct RecommendationService {
    sentence_encoder: Arc<HuggingFaceEmbedder>,
    pinecone: Pinecone,
    // Use thread-safe cache with read-write lock for better performance
    result_cache: std::sync::Arc<RwLock<HashMap<String, CacheEntry>>>,
    prewarmed: Arc<std::sync::atomic::AtomicBool>,
    query_enhancer: QueryEnhancer,
    semantic_classifier: SemanticClassifier,
}

impl RecommendationService {
    pub fn new(sentence_encoder: HuggingFaceEmbedder, pinecone: Pinecone) -> Self {
        let semantic_classifier = SemanticClassifier::new().unwrap_or_else(|e| {
            warn!(
                "Failed to initialize semantic classifier: {}. Using fallback.",
                e
            );
            SemanticClassifier::new().unwrap()
        });
        Self {
            sentence_encoder: Arc::new(sentence_encoder),
            pinecone,
            result_cache: std::sync::Arc::new(RwLock::new(HashMap::new())),
            prewarmed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            query_enhancer: QueryEnhancer::new(),
            semantic_classifier,
        }
    }

    /// Warms up the recommendation service to mitigate cold start issues
    ///
    /// This method:
    /// 1. Initializes the ML embedder
    /// 2. Establishes connection to Pinecone with a test query
    /// 3. Executes a sample query to prepare the entire pipeline
    ///
    /// Returns true if this was the first warm-up operation
    pub async fn prewarm(&self) -> Result<bool> {
        // Check if already prewarmed to avoid duplicate work
        if self.prewarmed.load(std::sync::atomic::Ordering::Relaxed) {
            debug!("RecommendationService already prewarmed, skipping");
            return Ok(false);
        }

        info!("Warming up RecommendationService...");

        // Step 1: Initialize the sentence encoder
        let _encoder_prewarmed = self.sentence_encoder.prewarm().await?;

        // Step 2: Initialize Pinecone connection with a simple metadata query
        let pinecone_test = self
            .pinecone
            .query_metadata("title", "test", false, 1)
            .await;
        if let Err(e) = &pinecone_test {
            warn!(
                "Pinecone initialization returned an error during warm-up: {}",
                e
            );
            // Continue anyway - this might be a temporary issue
        }

        // Step 3: Prime the recommendation pipeline with a common query
        // This helps initialize internal caches and prepares everything
        let test_queries = ["fantasy books", "science fiction", "mystery novels"];

        // Simple selection without fastrand dependency
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let test_query = test_queries[now as usize % test_queries.len()];
        debug!("Running test query for prewarm: '{}'", test_query);

        // Use internal implementation to avoid query intent caching
        let _embedding = self.sentence_encoder.encode(test_query).await?;

        // Create a dummy query intent
        let intent = QueryIntent::General {
            query: test_query.to_string(),
        };

        // Create a dummy search strategy for prewarming
        let strategy = SearchStrategy {
            metadata_filter: None,
            semantic_weight: 1.0,
            hybrid_search: true,
        };

        // Use a small limit for the test query
        let _ = self.perform_hybrid_search(&intent, &strategy, 3).await;

        // Mark as initialized
        self.prewarmed
            .store(true, std::sync::atomic::Ordering::Release);
        info!("RecommendationService successfully warmed up");

        Ok(true)
    }

    pub async fn get_recommendations(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<(Vec<Book>, Vec<String>)> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return Err(ApiError::InvalidInput("Query cannot be empty".into()));
        }

        // Validate query
        if trimmed_query.len() < 3 {
            return Err(ApiError::InvalidInput(
                "Query too short (minimum 3 characters)".into(),
            ));
        }

        if trimmed_query.len() > 200 {
            return Err(ApiError::InvalidInput(
                "Query too long (maximum 200 characters)".into(),
            ));
        }

        // Check cache for existing results
        let cache_key = format!("{}:{}", trimmed_query, top_k);
        info!("Generated cache key: {}", cache_key);

        // Try to read from cache first
        if let Ok(cache) = self.result_cache.read() {
            if let Some(entry) = cache.get(&cache_key) {
                if entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS) {
                    info!("CACHE HIT for query: {}", trimmed_query);
                    // For cached results, extract keywords
                    let query_info = self
                        .semantic_classifier
                        .analyze_query(trimmed_query)
                        .await
                        .unwrap_or_else(|e| {
                            warn!("Failed to analyze query for cached result: {}", e);
                            SemanticQueryInfo {
                                original_query: trimmed_query.to_string(),
                                themes: vec![],
                                author: None,
                                temporal_filter: None,
                                is_similar_query: false,
                                semantic_tags: vec![],
                            }
                        });
                    return Ok((entry.results.clone(), query_info.semantic_tags));
                }
            }
        }

        info!("CACHE MISS for query: {}", trimmed_query);

        // Extract keywords and metadata (no ML classification needed)
        let query_info = self
            .semantic_classifier
            .analyze_query(trimmed_query)
            .await
            .unwrap_or_else(|e| {
                warn!("Keyword extraction failed, using fallback: {}", e);
                SemanticQueryInfo {
                    original_query: trimmed_query.to_string(),
                    themes: vec![],
                    author: None,
                    temporal_filter: None,
                    is_similar_query: false,
                    semantic_tags: vec![],
                }
            });

        info!("Keyword extraction results:");
        info!("  - Keywords: {:?}", query_info.themes);
        info!("  - Author: {:?}", query_info.author);
        info!("  - Temporal filter: {:?}", query_info.temporal_filter);
        info!("  - Is similar query: {}", query_info.is_similar_query);
        info!("  - Display tags: {:?}", query_info.semantic_tags);

        // Convert to intent format
        let intent = self.semantic_info_to_intent(&query_info);
        info!(?intent, "Converted to intent format");

        // Get search strategy
        let strategy = self.get_search_strategy(&intent);
        info!("Performing hybrid search with strategy: {:?}", strategy);

        // Increase search scope
        let expanded_k = top_k * 3;

        // Perform hybrid search
        let raw_results = match self
            .perform_hybrid_search(&intent, &strategy, expanded_k)
            .await
        {
            Ok(results) => {
                debug!(
                    "Vector search returned {} results, first book: {:?}",
                    results.len(),
                    results.first().map(|r| r.title.clone())
                );
                results
            }
            Err(e) => {
                error!("Search error: {}. Trying fallback strategy", e);
                self.perform_fallback_search(trimmed_query, expanded_k)
                    .await?
            }
        };

        // Rank and process results with keywords
        let ranked_results =
            self.rank_results_with_semantic_info(raw_results, &intent, &query_info, top_k);
        info!(
            "Returning {} ranked results for query '{}'",
            ranked_results.len(),
            trimmed_query
        );

        // Update cache with new results
        if let Ok(mut cache) = self.result_cache.write() {
            info!(
                "Updating cache for key '{}' with {} results",
                cache_key,
                ranked_results.len()
            );

            cache.insert(
                cache_key,
                CacheEntry {
                    results: ranked_results.clone(),
                    timestamp: Instant::now(),
                },
            );

            if cache.len() > 100 {
                self.cleanup_cache(&mut cache);
            }

            info!("Current cache size: {} entries", cache.len());
        }

        Ok((ranked_results, query_info.semantic_tags))
    }

    /// Convert semantic query info to legacy QueryIntent format
    fn semantic_info_to_intent(&self, info: &SemanticQueryInfo) -> QueryIntent {
        // If author is detected, prioritize that - metadata search is best for authors
        if let Some(author) = &info.author {
            return QueryIntent::Author {
                name: author.clone(),
                original_query: info.original_query.clone(),
            };
        }

        // If similar query, use SimilarTo intent - semantic search is best
        if info.is_similar_query {
            return QueryIntent::SimilarTo {
                original_query: info.original_query.clone(),
            };
        }

        // For all other queries, use General intent
        // The semantic themes will be used in ranking and relevance indicators
        // This gives the best balance between semantic search and metadata filtering
        QueryIntent::General {
            query: info.original_query.clone(),
        }
    }

    /// Rank results with semantic information
    fn rank_results_with_semantic_info(
        &self,
        mut results: Vec<Book>,
        intent: &QueryIntent,
        query_info: &SemanticQueryInfo,
        top_k: usize,
    ) -> Vec<Book> {
        // Early return if no results or only one result
        if results.len() <= 1 {
            return results;
        }

        let max_needed = (top_k * 3).min(results.len());

        // Use existing ranking logic but with semantic information
        match intent {
            QueryIntent::Author { name, .. } => {
                let name_lower = name.to_lowercase();
                let mut indexed_books: Vec<(usize, i32, f32)> = results
                    .iter()
                    .enumerate()
                    .map(|(idx, book)| {
                        let author = book.author.as_deref().unwrap_or("").to_lowercase();
                        let exact_match = author.contains(&name_lower) as i32;
                        (idx, exact_match, book.rating)
                    })
                    .collect();

                indexed_books.sort_by(|a, b| {
                    let (_, a_exact, a_rating) = *a;
                    let (_, b_exact, b_rating) = *b;
                    b_exact.cmp(&a_exact).then_with(|| {
                        b_rating
                            .partial_cmp(&a_rating)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                });

                let mut sorted_results = Vec::with_capacity(results.len());
                for (idx, _, _) in indexed_books {
                    sorted_results.push(results[idx].clone());
                }
                results = sorted_results;
            }
            QueryIntent::Genre { genre, .. } => {
                let genre_lower = genre.to_lowercase();
                let mut indexed_books: Vec<(usize, i32, f32)> = results
                    .iter()
                    .enumerate()
                    .map(|(idx, book)| {
                        let categories = book.categories.join(", ").to_lowercase();
                        let has_match = categories.contains(&genre_lower) as i32;
                        (idx, has_match, book.rating)
                    })
                    .collect();

                indexed_books.sort_by(|a, b| {
                    let (_, a_match, a_rating) = *a;
                    let (_, b_match, b_rating) = *b;
                    b_match.cmp(&a_match).then_with(|| {
                        b_rating
                            .partial_cmp(&a_rating)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                });

                let mut sorted_results = Vec::with_capacity(results.len());
                for (idx, _, _) in indexed_books {
                    sorted_results.push(results[idx].clone());
                }
                results = sorted_results;
            }
            _ => {
                info!("Using GENERAL search ranking logic with keyword boost");

                let total_results = results.len();
                let mut scored_results = results
                    .iter()
                    .enumerate()
                    .map(|(idx, book)| {
                        let position_score = 3.0 * (1.0 - (idx as f32 / total_results as f32));
                        let rating_score = 0.85 + (book.rating / 5.0) * 0.10;

                        // Add keyword boost - check if query keywords appear in book metadata
                        let mut keyword_boost: f32 = 0.0;
                        for (keyword, _) in &query_info.themes {
                            let keyword_lower = keyword.to_lowercase();

                            // Check categories
                            let category_match = book.categories.iter().any(|cat| {
                                cat.to_lowercase().contains(&keyword_lower)
                            });

                            // Check title
                            let title_match = book.title.as_ref().map_or(false, |title| {
                                title.to_lowercase().contains(&keyword_lower)
                            });

                            // Check description
                            let desc_match = book.description.as_ref().map_or(false, |desc| {
                                desc.to_lowercase().contains(&keyword_lower)
                            });

                            if title_match {
                                keyword_boost += 1.0; // Title match is strongest
                            } else if category_match {
                                keyword_boost += 0.8; // Category match is strong
                            } else if desc_match {
                                keyword_boost += 0.5; // Description match is moderate
                            }
                        }

                        // Cap keyword boost at 2.0
                        keyword_boost = keyword_boost.min(2.0);

                        let final_score = if idx < 50 {
                            position_score + rating_score + keyword_boost
                        } else {
                            position_score * 0.7 + rating_score * 1.3 + keyword_boost
                        };

                        if tracing::enabled!(tracing::Level::DEBUG) {
                            debug!(
                                "Book scoring: {:?} - Position: {}/{} (score: {:.2}), Rating: {:.2}, Keyword boost: {:.2}, Final: {:.2}",
                                book.title, idx + 1, total_results, position_score, book.rating, keyword_boost, final_score
                            );
                        }

                        (book.clone(), final_score)
                    })
                    .collect::<Vec<_>>();

                scored_results
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                results = scored_results.into_iter().map(|(book, _)| book).collect();

                info!(
                    "Completed scoring of {} books with keyword boosting",
                    results.len()
                );
            }
        }

        // Remove duplicates
        let mut seen = HashSet::with_capacity(max_needed);
        let mut unique_results = Vec::with_capacity(max_needed);

        for book in results {
            if unique_results.len() >= top_k * 3 {
                break;
            }

            let key = format!(
                "{}-{}",
                book.title.as_deref().unwrap_or("Unknown"),
                book.author.as_deref().unwrap_or("Unknown")
            );

            if seen.insert(key) {
                unique_results.push(book);
            }
        }

        // Final ranking with metadata
        let final_results = unique_results
            .iter()
            .enumerate()
            .take(top_k)
            .map(|(index, book)| {
                let mut book_clone = book.clone();
                let position_factor = 1.0 - (index as f32 / top_k as f32);
                let rating_factor = book_clone.rating / 5.0;
                book_clone.confidence_score =
                    (position_factor * 0.7 + rating_factor * 0.3).min(1.0);

                book_clone.relevance_indicators =
                    self.generate_relevance_indicators_semantic(&book_clone, query_info);

                book_clone
            })
            .collect::<Vec<Book>>();

        info!(
            "FINAL RANKING: Top {} results ready. First book: {:?}",
            final_results.len(),
            final_results.first().map(|b| b.title.clone())
        );

        final_results
    }

    /// Generate relevance indicators using semantic information
    fn generate_relevance_indicators_semantic(
        &self,
        book: &Book,
        query_info: &SemanticQueryInfo,
    ) -> Vec<String> {
        let mut indicators = Vec::new();

        // Check for keyword matches in description and title
        for (keyword, _) in &query_info.themes {
            let keyword_lower = keyword.to_lowercase();

            // Check if keyword appears in title
            if book
                .title
                .as_ref()
                .map_or(false, |title| title.to_lowercase().contains(&keyword_lower))
            {
                indicators.push(keyword.clone());
                continue;
            }

            // Check if keyword appears in categories
            if book
                .categories
                .iter()
                .any(|cat| cat.to_lowercase().contains(&keyword_lower))
            {
                indicators.push(keyword.clone());
                continue;
            }

            // Check if keyword appears in description
            if book
                .description
                .as_ref()
                .map_or(false, |desc| desc.to_lowercase().contains(&keyword_lower))
            {
                indicators.push(keyword.clone());
            }
        }

        // Add author match if applicable
        if let Some(author) = &query_info.author {
            if book.author.as_ref().map_or(false, |book_author| {
                book_author.to_lowercase().contains(&author.to_lowercase())
            }) {
                indicators.push(format!("Author: {}", author));
            }
        }

        // Add categories if not enough indicators
        if indicators.len() < 2 {
            for category in &book.categories {
                if !indicators.contains(category) {
                    indicators.push(category.clone());
                }
                if indicators.len() >= 3 {
                    break;
                }
            }
        }

        indicators.into_iter().take(3).collect()
    }

    #[allow(dead_code)]
    pub fn get_cache_stats(&self) -> Option<usize> {
        self.query_enhancer.cache_stats().map(|s| s.valid_entries)
    }

    // Helper method to clean up expired cache entries
    fn cleanup_cache(&self, cache: &mut HashMap<String, CacheEntry>) {
        let expired_keys: Vec<String> = cache
            .iter()
            .filter(|(_, entry)| entry.timestamp.elapsed() > Duration::from_secs(CACHE_TTL_SECONDS))
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired_keys {
            cache.remove(&key);
        }
    }

    fn get_search_strategy(&self, intent: &QueryIntent) -> SearchStrategy {
        match intent {
            QueryIntent::Author { name, .. } => SearchStrategy {
                metadata_filter: Some(MetadataFilter {
                    field: "author".into(),
                    value: name.clone(),
                    exact_match: false,
                }),
                semantic_weight: 0.3,
                hybrid_search: true,
            },
            QueryIntent::Genre { genre, .. } => SearchStrategy {
                metadata_filter: Some(MetadataFilter {
                    field: "categories".into(),
                    value: genre.clone(),
                    exact_match: false,
                }),
                semantic_weight: 0.7,
                hybrid_search: true,
            },
            QueryIntent::SimilarTo { .. } => SearchStrategy {
                metadata_filter: None,
                semantic_weight: 0.8,
                hybrid_search: true,
            },
            QueryIntent::General { .. } => SearchStrategy {
                metadata_filter: None,
                semantic_weight: 0.6, // Lower weight gives more importance to ratings
                hybrid_search: true,  // Enable hybrid search for better results
            },
        }
    }

    async fn perform_hybrid_search(
        &self,
        intent: &QueryIntent,
        strategy: &SearchStrategy,
        top_k: usize,
    ) -> Result<Vec<Book>> {
        info!("Performing hybrid search with strategy: {:?}", strategy);
        let mut results = Vec::new();

        // Try metadata filtering if applicable
        if let Some(filter) = &strategy.metadata_filter {
            // Try exact match first
            if filter.exact_match {
                let exact_matches = self
                    .pinecone
                    .query_metadata(&filter.field, &filter.value, true, top_k * 3)
                    .await?;
                results.extend(exact_matches);
            }

            // If we need more results, try partial matching
            if results.len() < top_k {
                let partial_matches = self
                    .pinecone
                    .query_metadata(&filter.field, &filter.value, false, top_k * 3)
                    .await?;

                // Add only new results
                let existing_ids: HashSet<_> = results.iter().map(|r| r.id.clone()).collect();
                results.extend(
                    partial_matches
                        .into_iter()
                        .filter(|book| !existing_ids.contains(&book.id)),
                );
            }
        }

        // Perform semantic search if needed
        if results.len() < top_k || strategy.hybrid_search {
            let query_text = match intent {
                QueryIntent::Author { original_query, .. }
                | QueryIntent::Genre { original_query, .. }
                | QueryIntent::SimilarTo { original_query } => original_query,
                QueryIntent::General { query } => query,
            };

            // Try to get embeddings with fallback strategy
            let (semantic_results, using_fallback) =
                match self.sentence_encoder.encode(query_text).await {
                    Ok(embedding) => {
                        // Successfully got embedding, proceed with vector search
                        info!("Successfully encoded query '{}'", query_text);
                        debug!(
                        "Embedding stats: length={}, avg={:.4}, min={:.4}, max={:.4}, sum={:.4}",
                        embedding.len(),
                        embedding.iter().sum::<f32>() / embedding.len() as f32,
                        embedding.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
                        embedding.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)),
                        embedding.iter().sum::<f32>()
                    );
                        debug!(
                            "Performing vector search with embedding for '{}', semantic_weight={}",
                            query_text, strategy.semantic_weight
                        );
                        let results = self.pinecone.query_vector(&embedding, top_k * 3).await?;
                        (results, false) // Not using fallback
                    }
                    Err(e) => {
                        // Check if the error is a timeout
                        if e.to_string().contains("timed out") || e.to_string().contains("timeout")
                        {
                            // Log the timeout but continue with fallback strategy
                            warn!(
                                "HuggingFace API timed out, using fallback search strategy: {}",
                                e
                            );

                            // Use fallback search strategy when embeddings are unavailable
                            let fallback_results =
                                self.perform_fallback_search(query_text, top_k).await?;
                            (fallback_results, true) // Using fallback
                        } else {
                            // For non-timeout errors, propagate them
                            return Err(e);
                        }
                    }
                };

            if strategy.hybrid_search && !using_fallback {
                // Weight semantic results (only if not using fallback)
                debug!(
                    "Applying hybrid search weights: semantic_weight={}",
                    strategy.semantic_weight
                );
                for result in &mut results {
                    // Apply semantic weight as a scaling factor for ratings
                    // Higher semantic_weight means more trust in semantic search results
                    let original_rating = result.rating;
                    result.rating *= strategy.semantic_weight;
                    debug!(
                        "Adjusted rating for '{:?}': {} -> {}",
                        result.title, original_rating, result.rating
                    );
                }
            }

            // Add new semantic results
            let existing_ids: HashSet<_> = results.iter().map(|r| r.id.clone()).collect();
            results.extend(
                semantic_results
                    .into_iter()
                    .filter(|book| !existing_ids.contains(&book.id)),
            );
        }

        Ok(results)
    }

    /// Fallback search when HuggingFace embedding service is unavailable
    /// Uses metadata search based on query terms or falls back to popular books
    async fn perform_fallback_search(&self, query_text: &str, top_k: usize) -> Result<Vec<Book>> {
        info!("Using fallback search strategy for query: {}", query_text);

        // Extract meaningful terms from the query - optimized for better term extraction
        let terms: Vec<&str> = query_text
            .split_whitespace()
            .filter(|word| {
                // Use a more sophisticated filter for meaningful words
                let word_lower = word.to_lowercase();
                word.len() > 3
                    && !["this", "that", "with", "from", "have", "like"]
                        .contains(&word_lower.as_str())
            })
            .take(5) // Take more terms for better coverage
            .collect();

        // Use sequential search instead for simplicity
        let mut fallback_results = Vec::new();
        let mut seen_ids = HashSet::new();

        // If we have terms, search with each term
        if !terms.is_empty() {
            for term in &terms {
                // Search in title field
                if let Ok(title_matches) = self
                    .pinecone
                    .query_metadata("title", term, false, top_k * 3)
                    .await
                {
                    // Add unique books to results
                    for book in title_matches {
                        if let Some(id) = &book.id {
                            if seen_ids.insert(id.clone()) {
                                fallback_results.push(book);
                            }
                        } else {
                            fallback_results.push(book);
                        }

                        // If we have enough results, stop processing
                        if fallback_results.len() >= top_k * 2 {
                            break;
                        }
                    }
                }

                // If we already have enough results, don't keep searching
                if fallback_results.len() >= top_k * 2 {
                    break;
                }

                // Search in description field
                if let Ok(desc_matches) = self
                    .pinecone
                    .query_metadata("description", term, false, top_k * 3)
                    .await
                {
                    // Add unique books to results
                    for book in desc_matches {
                        if let Some(id) = &book.id {
                            if seen_ids.insert(id.clone()) {
                                fallback_results.push(book);
                            }
                        } else {
                            fallback_results.push(book);
                        }

                        // If we have enough results, stop processing
                        if fallback_results.len() >= top_k * 2 {
                            break;
                        }
                    }
                }

                // If we already have enough results, don't keep searching
                if fallback_results.len() >= top_k * 2 {
                    break;
                }
            }
        }

        // If we still don't have enough results, try more aggressive fallback strategies
        if fallback_results.len() < top_k {
            warn!("Parallel term search yielded insufficient results ({}), using additional fallbacks",
                  fallback_results.len());

            // Try high rating books first
            if let Ok(popular_books) = self
                .pinecone
                .query_metadata("rating", "4.5", false, top_k * 3)
                .await
            {
                // Use our existing seen_ids HashSet to filter duplicates
                for book in popular_books {
                    if let Some(id) = &book.id {
                        if seen_ids.insert(id.clone()) {
                            fallback_results.push(book);
                        }
                    } else {
                        fallback_results.push(book);
                    }
                }
            }

            // If we still need more, try recent/trending books
            if fallback_results.len() < top_k {
                if let Ok(recent_books) = self
                    .pinecone
                    .query_metadata("year", "2020", false, top_k * 3)
                    .await
                {
                    // Use our existing seen_ids HashSet to filter duplicates
                    for book in recent_books {
                        if let Some(id) = &book.id {
                            if seen_ids.insert(id.clone()) {
                                fallback_results.push(book);
                            }
                        } else {
                            fallback_results.push(book);
                        }
                    }
                }
            }
        }

        info!("Fallback strategy found {} results", fallback_results.len());

        // Ensure each fallback result has a recognizable ID for analytics
        for book in &mut fallback_results {
            if let Some(id) = &mut book.id {
                if !id.starts_with("fallback-") {
                    *id = format!("fallback-{}", id);
                }
            }
        }

        Ok(fallback_results)
    }
}
