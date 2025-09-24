//! Caching implementation for the recommendation service.
//!
//! This module provides specialized caching for recommendation results,
//! improving performance by avoiding redundant expensive operations.

use log::{debug, info};

use crate::error::ApiError;
use crate::models::Book;
use crate::services::utils::Cache;

/// Time-to-live for cached recommendation results in seconds
pub const RECOMMENDATION_CACHE_TTL: u64 = 1800; // 30 minutes

/// Cache key generation format for recommendation queries
#[allow(dead_code)]
#[derive(Clone)]
pub struct RecommendationCacheKey {
    pub query: String,
    pub top_k: usize,
}

#[allow(dead_code)]
impl RecommendationCacheKey {
    /// Create a new cache key for a recommendation request
    pub fn new(query: &str, top_k: usize) -> Self {
        Self {
            query: query.trim().to_lowercase(),
            top_k,
        }
    }

    /// Convert the key components to a string for use in the cache
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.query, self.top_k)
    }
}

/// Recommendation cache for storing and retrieving recommendation results
pub struct RecommendationCache {
    /// The underlying generic cache implementation
    cache: Cache<String, Vec<Book>>,
}

#[allow(dead_code)]
impl RecommendationCache {
    /// Create a new recommendation cache with the default TTL
    pub fn new() -> Self {
        Self {
            cache: Cache::new(RECOMMENDATION_CACHE_TTL),
        }
    }

    /// Create a new recommendation cache with a custom TTL
    pub fn with_ttl(ttl_seconds: u64) -> Self {
        Self {
            cache: Cache::new(ttl_seconds),
        }
    }

    /// Get cached recommendation results for a query
    pub fn get(&self, key: &RecommendationCacheKey) -> Option<Vec<Book>> {
        let cache_key = key.to_string();

        match self.cache.get(&cache_key) {
            Some(results) => {
                info!("Cache hit for query: {}", key.query);
                Some(results)
            }
            None => {
                debug!("Cache miss for query: {}", key.query);
                None
            }
        }
    }

    /// Store recommendation results in the cache
    pub fn set(&self, key: RecommendationCacheKey, results: Vec<Book>) -> Result<(), ApiError> {
        let cache_key = key.to_string();

        self.cache.set(cache_key, results).map_err(|e| {
            ApiError::cache_error(format!("Failed to store in recommendation cache: {}", e))
                .with_context("recommendation_cache")
                .with_operation("set")
        })
    }

    /// Invalidate a specific cache entry
    pub fn invalidate(&self, key: &RecommendationCacheKey) -> Result<(), ApiError> {
        let cache_key = key.to_string();

        self.cache.invalidate(&cache_key).map_err(|e| {
            ApiError::cache_error(format!("Failed to invalidate cache entry: {}", e))
                .with_context("recommendation_cache")
                .with_operation("invalidate")
        })
    }

    /// Clean up expired cache entries
    pub fn cleanup(&self) -> Result<usize, ApiError> {
        self.cache.cleanup().map_err(|e| {
            ApiError::cache_error(format!("Failed to clean up recommendation cache: {}", e))
                .with_context("recommendation_cache")
                .with_operation("cleanup")
        })
    }

    /// Get all cache keys (for debugging/monitoring)
    pub fn keys(&self) -> Result<Vec<String>, ApiError> {
        self.cache.keys().map_err(|e| {
            ApiError::cache_error(format!("Failed to get recommendation cache keys: {}", e))
                .with_context("recommendation_cache")
                .with_operation("keys")
        })
    }

    /// Get the number of entries in the cache
    pub fn len(&self) -> Result<usize, ApiError> {
        self.cache.len().map_err(|e| {
            ApiError::cache_error(format!("Failed to get recommendation cache size: {}", e))
                .with_context("recommendation_cache")
                .with_operation("len")
        })
    }

    /// Clear all cache entries
    pub fn clear(&self) -> Result<(), ApiError> {
        self.cache.clear().map_err(|e| {
            ApiError::cache_error(format!("Failed to clear recommendation cache: {}", e))
                .with_context("recommendation_cache")
                .with_operation("clear")
        })
    }
}

impl Default for RecommendationCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn test_cache_key_generation() {
        let key1 = RecommendationCacheKey::new("Fantasy Books", 10);
        let key2 = RecommendationCacheKey::new("fantasy books", 10);
        let key3 = RecommendationCacheKey::new("Fantasy Books", 5);

        // Case-insensitive keys
        assert_eq!(key1.to_string(), key2.to_string());

        // Different top_k should result in different keys
        assert_ne!(key1.to_string(), key3.to_string());
    }

    #[test]
    fn test_recommendation_cache_operations() {
        let cache = RecommendationCache::with_ttl(1);
        let key = RecommendationCacheKey::new("test query", 5);

        // Create some test books
        let books = vec![Book {
            id: Some(Uuid::new_v4().to_string()),
            title: Some("Test Book 1".to_string()),
            author: Some("Test Author".to_string()),
            description: Some("Test description".to_string()),
            categories: vec!["Fiction".to_string()],
            thumbnail: None,
            rating: 4.5,
            year: Some(2020),
            isbn: None,
            page_count: Some(200),
            ratings_count: Some(100),
            language: Some("English".to_string()),
            publisher: Some("Test Publisher".to_string()),
        }];

        // Set and retrieve from cache
        cache.set(key.clone(), books.clone()).unwrap();
        let retrieved = cache.get(&key).unwrap();

        assert_eq!(retrieved.len(), books.len());
        assert_eq!(retrieved[0].title, books[0].title);

        // Test expiration
        thread::sleep(Duration::from_secs(2));
        assert!(cache.get(&key).is_none());
    }
}
