use crate::services::templates::{EnhancedQuery, QueryPattern};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Cache entry for enhanced queries
struct CacheEntry {
    enhanced_query: EnhancedQuery,
    timestamp: Instant,
}

/// Service for enhancing user queries using template-based approach
#[derive(Clone)]
pub struct QueryEnhancer {
    /// Cache for enhanced queries to avoid repeated processing
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Cache TTL in seconds
    cache_ttl: Duration,
}

#[allow(dead_code)]
impl QueryEnhancer {
    /// Create a new QueryEnhancer with default settings
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(3600), // 1 hour cache
        }
    }

    /// Create a new QueryEnhancer with custom cache TTL
    pub fn with_ttl(cache_ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(cache_ttl_seconds),
        }
    }

    /// Enhance a user query using template-based pattern matching
    ///
    /// This method:
    /// 1. Checks cache first for instant results
    /// 2. Parses query to identify intent (author, genre, mood, etc.)
    /// 3. Extracts key terms and expands with synonyms
    /// 4. Provides search hints for optimal retrieval
    /// 5. Caches result for future use
    pub fn enhance(&self, query: &str) -> EnhancedQuery {
        let query_trimmed = query.trim();

        // Check cache first
        if let Ok(cache) = self.cache.read() {
            if let Some(entry) = cache.get(query_trimmed) {
                if entry.timestamp.elapsed() < self.cache_ttl {
                    debug!("Cache HIT for query enhancement: '{}'", query_trimmed);
                    return entry.enhanced_query.clone();
                }
            }
        }

        info!(
            "Cache MISS for query enhancement: '{}' - processing with templates",
            query_trimmed
        );

        // Process query using templates
        let enhanced_query = EnhancedQuery::from_query(query_trimmed);

        // Log enhancement results
        self.log_enhancement(&enhanced_query);

        // Update cache
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(
                query_trimmed.to_string(),
                CacheEntry {
                    enhanced_query: enhanced_query.clone(),
                    timestamp: Instant::now(),
                },
            );

            // Cleanup old entries if cache is too large
            if cache.len() > 1000 {
                self.cleanup_cache(&mut cache);
            }
        }

        enhanced_query
    }

    /// Log enhancement results for debugging
    fn log_enhancement(&self, enhanced: &EnhancedQuery) {
        info!("Query Enhancement Results:");
        info!("  Pattern: {:?}", enhanced.pattern);
        info!("  Extracted terms: {:?}", enhanced.extracted_terms);

        if !enhanced.expanded_terms.is_empty() {
            debug!("  Expanded terms: {:?}", enhanced.expanded_terms);
        }

        if enhanced.filters.author.is_some() {
            info!("  Author filter: {:?}", enhanced.filters.author);
        }

        if !enhanced.filters.genres.is_empty() {
            info!("  Genre filters: {:?}", enhanced.filters.genres);
        }

        if !enhanced.filters.themes.is_empty() {
            info!("  Theme filters: {:?}", enhanced.filters.themes);
        }

        info!(
            "  Search hints: semantic={:.2}, metadata={:.2}, rating_boost={:.2}",
            enhanced.search_hints.semantic_weight,
            enhanced.search_hints.metadata_weight,
            enhanced.search_hints.rating_boost
        );
    }

    /// Clean up expired cache entries
    fn cleanup_cache(&self, cache: &mut HashMap<String, CacheEntry>) {
        let expired_keys: Vec<String> = cache
            .iter()
            .filter(|(_, entry)| entry.timestamp.elapsed() > self.cache_ttl)
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired_keys {
            cache.remove(&key);
        }

        info!(
            "Cleaned up query enhancement cache, remaining entries: {}",
            cache.len()
        );
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> Option<CacheStats> {
        if let Ok(cache) = self.cache.read() {
            let _now = Instant::now();
            let valid_entries = cache
                .values()
                .filter(|entry| entry.timestamp.elapsed() < self.cache_ttl)
                .count();

            Some(CacheStats {
                total_entries: cache.len(),
                valid_entries,
                expired_entries: cache.len() - valid_entries,
            })
        } else {
            None
        }
    }

    /// Clear the cache (useful for testing or manual cache management)
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
            info!("Query enhancement cache cleared");
        }
    }

    /// Get pattern type for a query without full enhancement (lightweight)
    pub fn detect_pattern(&self, query: &str) -> QueryPattern {
        let enhanced = self.enhance(query);
        enhanced.pattern
    }
}

impl Default for QueryEnhancer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the query enhancement cache
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_author_query_enhancement() {
        let enhancer = QueryEnhancer::new();
        let enhanced = enhancer.enhance("books by J.K. Rowling");

        assert_eq!(enhanced.pattern, QueryPattern::Author);
        assert!(enhanced.filters.author.is_some());
        assert!(enhanced.search_hints.metadata_weight > enhanced.search_hints.semantic_weight);
    }

    #[test]
    fn test_genre_query_enhancement() {
        let enhancer = QueryEnhancer::new();
        let enhanced = enhancer.enhance("science fiction books");

        assert_eq!(enhanced.pattern, QueryPattern::Genre);
        assert!(!enhanced.filters.genres.is_empty());
        assert!(enhanced.expanded_terms.len() > 1);
    }

    #[test]
    fn test_caching() {
        let enhancer = QueryEnhancer::with_ttl(1);
        let query = "fantasy books";

        // First call - should process
        let result1 = enhancer.enhance(query);

        // Second call - should use cache
        let result2 = enhancer.enhance(query);

        assert_eq!(result1.original_query, result2.original_query);
        assert_eq!(result1.pattern, result2.pattern);
    }

    #[test]
    fn test_theme_extraction() {
        let enhancer = QueryEnhancer::new();
        let enhanced = enhancer.enhance("books about dragons and magic");

        assert!(!enhanced.filters.themes.is_empty());
        assert!(
            enhanced.filters.themes.contains(&"dragon".to_string())
                || enhanced.filters.themes.contains(&"magic".to_string())
        );
    }
}
