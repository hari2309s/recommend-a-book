use crate::error::Result;
use crate::{
    error::ApiError, ml::huggingface_embedder::HuggingFaceEmbedder, models::Book,
    services::pinecone::Pinecone,
};
use regex::Regex;
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, Mutex, RwLock},
    time::{Duration, Instant},
};
use tracing::{debug, error, info, warn};

static AUTHOR_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)(?:books?\s+)?(?:written\s+)?by\s+([a-zA-Z\s.'-]+)").unwrap(),
        Regex::new(r"(?i)(?:works?\s+)?(?:of|from)\s+([a-zA-Z\s.'-]+)").unwrap(),
        Regex::new(r"(?i)([a-zA-Z\s.'-]+)'s\s+books?").unwrap(),
        Regex::new(r"(?i)author:?\s*([a-zA-Z\s.'-]+)").unwrap(),
    ]
});

static GENRE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(
            r"(?i)(?:genre:?\s*)?(?:books?\s+in\s+)?([a-zA-Z\s&-]+?)\s+(?:books?|novels?|genre)",
        )
        .unwrap(),
        Regex::new(
            r"(?i)(?:recommend\s+)?([a-zA-Z\s&-]+?)\s+(?:books?|novels?|fiction|non-fiction)",
        )
        .unwrap(),
    ]
});

static SIMILAR_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)(?:books?\s+)?(?:similar\s+to|like)\s+(.+)").unwrap(),
        Regex::new(r"(?i)(?:more\s+books?\s+like)\s+(.+)").unwrap(),
    ]
});

static COMMON_GENRES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "fiction",
        "non-fiction",
        "mystery",
        "romance",
        "fantasy",
        "sci-fi",
        "science fiction",
        "biography",
        "history",
        "self-help",
        "business",
        "philosophy",
        "poetry",
        "drama",
        "thriller",
        "horror",
        "young adult",
        "children",
    ]
    .into()
});

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
    sentence_encoder: HuggingFaceEmbedder,
    pinecone: Pinecone,
    // Use thread-safe cache with read-write lock for better performance
    result_cache: std::sync::Arc<RwLock<HashMap<String, CacheEntry>>>,
    query_intent_cache: std::sync::Arc<Mutex<HashMap<String, (QueryIntent, Instant)>>>,
}

impl RecommendationService {
    pub fn new(sentence_encoder: HuggingFaceEmbedder, pinecone: Pinecone) -> Self {
        Self {
            sentence_encoder,
            pinecone,
            result_cache: std::sync::Arc::new(RwLock::new(HashMap::new())),
            query_intent_cache: std::sync::Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_recommendations(&self, query: &str, top_k: usize) -> Result<Vec<Book>> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return Err(ApiError::InvalidInput("Query cannot be empty".into()));
        }

        // Check cache for existing results
        let cache_key = format!("{}:{}", trimmed_query, top_k);

        // Try to read from cache first (read lock is faster than write lock)
        if let Ok(cache) = self.result_cache.read() {
            if let Some(entry) = cache.get(&cache_key) {
                // Return cached results if they're still valid
                if entry.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECONDS) {
                    debug!("Cache hit for query: {}", trimmed_query);
                    return Ok(entry.results.clone());
                }
            }
        }

        // Parse query intent with caching
        let intent = self.get_cached_intent(trimmed_query);
        debug!(?intent, "Detected query intent");

        // Get search strategy
        let strategy = self.get_search_strategy(&intent);
        debug!(?strategy, "Using search strategy");

        // Perform hybrid search with better error handling
        let raw_results = match self.perform_hybrid_search(&intent, &strategy, top_k).await {
            Ok(results) => results,
            Err(e) => {
                error!("Search error: {}. Trying fallback strategy", e);
                // Attempt fallback if the main search fails
                self.perform_fallback_search(trimmed_query, top_k).await?
            }
        };

        debug!(result_count = raw_results.len(), "Got raw search results");

        // Rank and process results
        let ranked_results = self.rank_results(raw_results, &intent, top_k);
        info!(
            final_count = ranked_results.len(),
            "Returning ranked results"
        );

        // Update cache with new results
        if let Ok(mut cache) = self.result_cache.write() {
            cache.insert(
                cache_key,
                CacheEntry {
                    results: ranked_results.clone(),
                    timestamp: Instant::now(),
                },
            );

            // Cleanup old cache entries periodically
            if cache.len() > 100 {
                self.cleanup_cache(&mut cache);
            }
        }

        Ok(ranked_results)
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

    // Get cached intent or compute new one
    fn get_cached_intent(&self, query: &str) -> QueryIntent {
        let now = Instant::now();
        let cache_ttl = Duration::from_secs(CACHE_TTL_SECONDS);

        // Try to get from cache first
        if let Ok(mut cache) = self.query_intent_cache.lock() {
            if let Some((intent, timestamp)) = cache.get(query) {
                if timestamp.elapsed() < cache_ttl {
                    return intent.clone();
                }
            }

            // Not in cache or expired, compute new intent
            let intent = self.parse_query_intent(query);

            // Update cache
            cache.insert(query.to_string(), (intent.clone(), now));

            // Cleanup old entries if cache is too large
            if cache.len() > 1000 {
                let expired_keys: Vec<String> = cache
                    .iter()
                    .filter(|(_, (_, ts))| ts.elapsed() > cache_ttl)
                    .map(|(k, _)| k.clone())
                    .collect();

                for key in expired_keys {
                    cache.remove(&key);
                }
            }

            intent
        } else {
            // If we can't lock the cache, just compute the intent
            self.parse_query_intent(query)
        }
    }

    fn parse_query_intent(&self, query: &str) -> QueryIntent {
        // Try to match author patterns
        for pattern in AUTHOR_PATTERNS.iter() {
            if let Some(cap) = pattern.captures(query) {
                let author = cap[1]
                    .trim()
                    .replace(|c: char| !c.is_alphanumeric() && c != ' ', "");
                return QueryIntent::Author {
                    name: author,
                    original_query: query.to_string(),
                };
            }
        }

        // Try to match genre patterns
        for pattern in GENRE_PATTERNS.iter() {
            if let Some(cap) = pattern.captures(query) {
                let potential_genre = cap[1].trim().to_lowercase();
                if COMMON_GENRES.iter().any(|genre| {
                    potential_genre.contains(*genre) || genre.contains(&potential_genre)
                }) {
                    return QueryIntent::Genre {
                        genre: potential_genre,
                        original_query: query.to_string(),
                    };
                }
            }
        }

        // Try to match similar-to patterns
        for pattern in SIMILAR_PATTERNS.iter() {
            if pattern.captures(query).is_some() {
                return QueryIntent::SimilarTo {
                    original_query: query.to_string(),
                };
            }
        }

        // Default to general search
        QueryIntent::General {
            query: query.to_string(),
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
            QueryIntent::SimilarTo { .. } | QueryIntent::General { .. } => SearchStrategy {
                metadata_filter: None,
                semantic_weight: 1.0,
                hybrid_search: false,
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
                for result in &mut results {
                    result.rating *= strategy.semantic_weight;
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
                // For general search, sort directly by rating for better performance
                results.sort_by(|a, b| {
                    b.rating
                        .partial_cmp(&a.rating)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
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

        // Only limit to exactly top_k at the very end
        unique_results.into_iter().take(top_k).collect()
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
