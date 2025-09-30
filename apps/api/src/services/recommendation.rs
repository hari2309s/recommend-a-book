use crate::error::Result;
use crate::services::templates::EnhancedQuery;
use crate::services::ExplanationGenerator;
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
    explanation_generator: ExplanationGenerator,
}

impl RecommendationService {
    pub fn new(sentence_encoder: HuggingFaceEmbedder, pinecone: Pinecone) -> Self {
        Self {
            sentence_encoder: Arc::new(sentence_encoder),
            pinecone,
            result_cache: std::sync::Arc::new(RwLock::new(HashMap::new())),
            prewarmed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            query_enhancer: QueryEnhancer::new(),
            explanation_generator: ExplanationGenerator::new(),
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

    pub async fn get_recommendations(&self, query: &str, top_k: usize) -> Result<Vec<Book>> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return Err(ApiError::InvalidInput("Query cannot be empty".into()));
        }

        // Check cache for existing results
        let cache_key = format!("{}:{}", trimmed_query, top_k);
        info!("Generated cache key: {}", cache_key);

        // Try to read from cache first
        if let Ok(cache) = self.result_cache.read() {
            info!("Current cache keys: {:?}", cache.keys().collect::<Vec<_>>());
            if let Some(entry) = cache.get(&cache_key) {
                if entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS) {
                    info!("CACHE HIT for query: {}", trimmed_query);
                    return Ok(entry.results.clone());
                }
            }
        }

        info!("CACHE MISS for query: {}", trimmed_query);

        // *** NEW: Use template-based query enhancement ***
        let enhanced_query = self.query_enhancer.enhance(trimmed_query);
        info!("Enhanced query pattern: {:?}", enhanced_query.pattern);
        info!("Extracted terms: {:?}", enhanced_query.extracted_terms);

        // Convert EnhancedQuery to your existing QueryIntent for backward compatibility
        let intent = self.convert_enhanced_to_intent(&enhanced_query);
        info!(?intent, "Converted to legacy intent format");

        // Get search strategy (your existing code)
        let strategy = self.get_search_strategy(&intent);
        info!("Performing hybrid search with strategy: {:?}", strategy);

        // Increase search scope
        let expanded_k = top_k * 3;

        // Perform hybrid search (your existing code)
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

        // Rank and process results (your existing code)
        let mut ranked_results = self.rank_results(raw_results, &intent, top_k);
        info!(
            "Returning {} ranked results for query '{}'",
            ranked_results.len(),
            trimmed_query
        );

        // *** NEW: Generate explanations for all results ***
        let explanations = self.explanation_generator.generate_batch_explanations(
            trimmed_query,
            &ranked_results,
            &enhanced_query,
        );

        // Attach explanations to books
        for (book, explanation) in ranked_results.iter_mut().zip(explanations.iter()) {
            book.explanation = Some(explanation.clone());
        }

        info!(
            "Generated {} explanations for recommendations",
            explanations.len()
        );

        // Update cache with new results (your existing code)
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

        Ok(ranked_results)
    }

    #[allow(dead_code)]
    pub fn get_cache_stats(&self) -> (Option<usize>, Option<usize>) {
        let query_stats = self.query_enhancer.cache_stats().map(|s| s.valid_entries);
        let explanation_stats = self
            .explanation_generator
            .cache_stats()
            .map(|s| s.valid_entries);
        (query_stats, explanation_stats)
    }

    fn convert_enhanced_to_intent(&self, enhanced: &EnhancedQuery) -> QueryIntent {
        use crate::services::templates::QueryPattern;

        match enhanced.pattern {
            QueryPattern::Author => {
                if let Some(author) = &enhanced.filters.author {
                    QueryIntent::Author {
                        name: author.clone(),
                        original_query: enhanced.original_query.clone(),
                    }
                } else {
                    QueryIntent::General {
                        query: enhanced.original_query.clone(),
                    }
                }
            }
            QueryPattern::Genre => {
                if let Some(genre) = enhanced.filters.genres.first() {
                    QueryIntent::Genre {
                        genre: genre.clone(),
                        original_query: enhanced.original_query.clone(),
                    }
                } else {
                    QueryIntent::General {
                        query: enhanced.original_query.clone(),
                    }
                }
            }
            QueryPattern::SimilarTo => QueryIntent::SimilarTo {
                original_query: enhanced.original_query.clone(),
            },
            _ => QueryIntent::General {
                query: enhanced.original_query.clone(),
            },
        }
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

    fn rank_results(
        &self,
        mut results: Vec<Book>,
        intent: &QueryIntent,
        top_k: usize,
    ) -> Vec<Book> {
        // Early return if no results or only one result
        if results.len() <= 1 {
            return results;
        }

        // Calculate the max result count to avoid unnecessary sorting
        // Allow for 2-3x more results than requested to account for duplicates and filtering
        let max_needed = (top_k * 3).min(results.len());

        // Use specialized sorting based on intent for better performance
        match intent {
            QueryIntent::Author { name, .. } => {
                let name_lower = name.to_lowercase();

                // Create a vector with (index, match score, rating) for each book
                let mut indexed_books: Vec<(usize, i32, f32)> = results
                    .iter()
                    .enumerate()
                    .map(|(idx, book)| {
                        let author = book.author.as_deref().unwrap_or("").to_lowercase();
                        let exact_match = author.contains(&name_lower) as i32;
                        (idx, exact_match, book.rating)
                    })
                    .collect();

                // Sort the indices
                indexed_books.sort_by(|a, b| {
                    let (_, a_exact, a_rating) = *a;
                    let (_, b_exact, b_rating) = *b;

                    b_exact.cmp(&a_exact).then_with(|| {
                        b_rating
                            .partial_cmp(&a_rating)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                });

                // Create a new sorted result vector
                let mut sorted_results = Vec::with_capacity(results.len());
                for (idx, _, _) in indexed_books {
                    sorted_results.push(results[idx].clone());
                }

                // Replace the original results with the sorted ones
                results = sorted_results;
            }
            QueryIntent::Genre { genre, .. } => {
                let genre_lower = genre.to_lowercase();

                // Create a vector with (index, match score, rating) for each book
                let mut indexed_books: Vec<(usize, i32, f32)> = results
                    .iter()
                    .enumerate()
                    .map(|(idx, book)| {
                        let categories = book.categories.join(", ").to_lowercase();
                        let has_match = categories.contains(&genre_lower) as i32;
                        (idx, has_match, book.rating)
                    })
                    .collect();

                // Sort the indices
                indexed_books.sort_by(|a, b| {
                    let (_, a_match, a_rating) = *a;
                    let (_, b_match, b_rating) = *b;

                    b_match.cmp(&a_match).then_with(|| {
                        b_rating
                            .partial_cmp(&a_rating)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                });

                // Create a new sorted result vector
                let mut sorted_results = Vec::with_capacity(results.len());
                for (idx, _, _) in indexed_books {
                    sorted_results.push(results[idx].clone());
                }

                // Replace the original results with the sorted ones
                results = sorted_results;
            }
            _ => {
                info!("Using GENERAL search ranking logic");

                // For general search, use more sophisticated ranking that balances
                // semantic relevance with book quality
                let total_results = results.len();
                let mut scored_results = results.iter().enumerate()
                    .map(|(idx, book)| {
                        // Position score: 3.0 (best) to 0.01 (worst)
                        let position_score = 3.0 * (1.0 - (idx as f32 / total_results as f32));

                        // Rating score: Scale to 0-1 range and then to 0.85-0.95
                        // This gives ratings influence but doesn't overpower position
                        let rating_score = 0.85 + (book.rating / 5.0) * 0.10;

                        // Compute final score
                        let final_score = if idx < 50 {
                            // For top results, position has more weight
                            position_score + rating_score
                        } else {
                            // For later results, rating has more weight to help good books rise
                            position_score * 0.7 + rating_score * 1.3
                        };

                        if tracing::enabled!(tracing::Level::DEBUG) {
                            debug!("Book scoring: {:?} - Position: {}/{} (score: {:.2}), Rating: {:.2} (scaled: {:.2}), Final score: {:.2}",
                                 book.title, idx + 1, total_results, position_score, book.rating, rating_score, final_score);
                        }

                        (book.clone(), final_score)
                    })
                    .collect::<Vec<_>>();

                // Sort by final score
                scored_results
                    .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                // Extract just the books in new order
                results = scored_results.into_iter().map(|(book, _)| book).collect();

                // Just log a summary at info level
                info!("Completed scoring of {} books", results.len());

                // Add detailed information only at debug level
                if tracing::enabled!(tracing::Level::DEBUG) {
                    debug!(
                        "After custom scoring, top 5 results: {:?}",
                        results
                            .iter()
                            .take(5)
                            .map(|b| b.title.clone())
                            .collect::<Vec<_>>()
                    );
                }
            }
        }

        // Remove duplicates but don't limit too early
        let mut seen = HashSet::with_capacity(max_needed);
        let mut unique_results = Vec::with_capacity(max_needed);

        for book in results {
            // Keep collecting until we have significantly more than requested
            // This ensures we don't limit too early and end up with fewer than desired
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

        // Final ranking list
        let final_results = unique_results
            .into_iter()
            .take(top_k)
            .collect::<Vec<Book>>();

        info!(
            "FINAL RANKING: Top {} results ready for response. First book: {:?}",
            final_results.len(),
            final_results.first().map(|b| b.title.clone())
        );

        // Return limited results
        final_results
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
