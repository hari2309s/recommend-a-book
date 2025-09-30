use crate::models::Book;
use crate::services::templates::{generate_explanation, EnhancedQuery};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Cache entry for explanations
struct ExplanationCacheEntry {
    explanation: String,
    timestamp: Instant,
}

/// Service for generating explanations for book recommendations
#[derive(Clone)]
pub struct ExplanationGenerator {
    /// Cache for explanations to avoid repeated generation
    cache: Arc<RwLock<HashMap<String, ExplanationCacheEntry>>>,
    /// Cache TTL in seconds
    cache_ttl: Duration,
}

#[allow(dead_code)]
impl ExplanationGenerator {
    /// Create a new ExplanationGenerator with default settings
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(86400), // 24 hours cache
        }
    }

    /// Create a new ExplanationGenerator with custom cache TTL
    pub fn with_ttl(cache_ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(cache_ttl_seconds),
        }
    }

    /// Generate explanation for a single book recommendation
    ///
    /// This method uses template-based generation with caching for performance.
    /// Explanations are contextual based on the query and book metadata.
    pub fn generate_explanation(
        &self,
        query: &str,
        book: &Book,
        enhanced_query: &EnhancedQuery,
    ) -> String {
        // Create cache key from query + book ID
        let cache_key = self.create_cache_key(query, book);

        // Check cache first
        if let Ok(cache) = self.cache.read() {
            if let Some(entry) = cache.get(&cache_key) {
                if entry.timestamp.elapsed() < self.cache_ttl {
                    debug!("Cache HIT for explanation: book={:?}", book.title);
                    return entry.explanation.clone();
                }
            }
        }

        debug!(
            "Cache MISS for explanation: book={:?} - generating",
            book.title
        );

        // Generate explanation using templates
        let explanation = generate_explanation(query, book, &enhanced_query.pattern);

        // Update cache
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(
                cache_key,
                ExplanationCacheEntry {
                    explanation: explanation.clone(),
                    timestamp: Instant::now(),
                },
            );

            // Cleanup old entries if cache is too large
            if cache.len() > 5000 {
                self.cleanup_cache(&mut cache);
            }
        }

        explanation
    }

    /// Generate explanations for multiple books efficiently
    ///
    /// This processes all books and uses caching to avoid repeated work.
    pub fn generate_batch_explanations(
        &self,
        query: &str,
        books: &[Book],
        enhanced_query: &EnhancedQuery,
    ) -> Vec<String> {
        info!("Generating explanations for {} books", books.len());

        books
            .iter()
            .map(|book| self.generate_explanation(query, book, enhanced_query))
            .collect()
    }

    /// Generate explanations only for top N results
    ///
    /// This is more efficient than generating for all results when you only
    /// need explanations for the top recommendations.
    pub fn generate_top_explanations(
        &self,
        query: &str,
        books: &[Book],
        enhanced_query: &EnhancedQuery,
        top_n: usize,
    ) -> Vec<String> {
        let limit = top_n.min(books.len());
        info!(
            "Generating explanations for top {} of {} books",
            limit,
            books.len()
        );

        books
            .iter()
            .take(limit)
            .map(|book| self.generate_explanation(query, book, enhanced_query))
            .collect()
    }

    /// Create a cache key for a query-book pair
    fn create_cache_key(&self, query: &str, book: &Book) -> String {
        // Use book ID if available, otherwise use title+author
        let book_identifier = if let Some(id) = &book.id {
            id.clone()
        } else {
            format!(
                "{}:{}",
                book.title.as_deref().unwrap_or("unknown"),
                book.author.as_deref().unwrap_or("unknown")
            )
        };

        // Create a simple cache key
        // We use a hash of the query to keep keys shorter
        let query_hash = self.simple_hash(query);
        format!("{}:{}", query_hash, book_identifier)
    }

    /// Simple hash function for query strings
    fn simple_hash(&self, s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Clean up expired cache entries
    fn cleanup_cache(&self, cache: &mut HashMap<String, ExplanationCacheEntry>) {
        let expired_keys: Vec<String> = cache
            .iter()
            .filter(|(_, entry)| entry.timestamp.elapsed() > self.cache_ttl)
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired_keys {
            cache.remove(&key);
        }

        info!(
            "Cleaned up explanation cache, remaining entries: {}",
            cache.len()
        );
    }

    /// Get cache statistics for monitoring
    pub fn cache_stats(&self) -> Option<CacheStats> {
        if let Ok(cache) = self.cache.read() {
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
            info!("Explanation cache cleared");
        }
    }
}

impl Default for ExplanationGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the explanation cache
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

    fn create_test_book(title: &str, author: &str, rating: f32, genres: Vec<String>) -> Book {
        Book {
            id: Some(format!("book_{}", title)),
            title: Some(title.to_string()),
            author: Some(author.to_string()),
            description: Some("A great book".to_string()),
            categories: genres,
            thumbnail: None,
            rating,
            year: Some(2020),
            isbn: None,
            page_count: Some(300),
            ratings_count: Some(5000),
            language: Some("English".to_string()),
            publisher: None,
            explanation: None,
        }
    }

    #[test]
    fn test_explanation_generation() {
        let generator = ExplanationGenerator::new();
        let book = create_test_book(
            "The Hobbit",
            "J.R.R. Tolkien",
            4.5,
            vec!["Fantasy".to_string()],
        );
        let enhanced_query = EnhancedQuery::from_query("fantasy books");

        let explanation = generator.generate_explanation("fantasy books", &book, &enhanced_query);

        assert!(!explanation.is_empty());
        assert!(explanation.len() > 10); // Should be a meaningful explanation
    }

    #[test]
    fn test_batch_explanations() {
        let generator = ExplanationGenerator::new();
        let books = vec![
            create_test_book("Book 1", "Author 1", 4.0, vec!["Fantasy".to_string()]),
            create_test_book("Book 2", "Author 2", 4.2, vec!["Sci-Fi".to_string()]),
            create_test_book("Book 3", "Author 3", 4.5, vec!["Mystery".to_string()]),
        ];
        let enhanced_query = EnhancedQuery::from_query("good books");

        let explanations =
            generator.generate_batch_explanations("good books", &books, &enhanced_query);

        assert_eq!(explanations.len(), 3);
        assert!(explanations.iter().all(|e| !e.is_empty()));
    }

    #[test]
    fn test_caching() {
        let generator = ExplanationGenerator::with_ttl(1);
        let book = create_test_book("Test Book", "Test Author", 4.0, vec!["Test".to_string()]);
        let enhanced_query = EnhancedQuery::from_query("test");

        // First call - should generate
        let explanation1 = generator.generate_explanation("test", &book, &enhanced_query);

        // Second call - should use cache
        let explanation2 = generator.generate_explanation("test", &book, &enhanced_query);

        assert_eq!(explanation1, explanation2);
    }

    #[test]
    fn test_top_n_explanations() {
        let generator = ExplanationGenerator::new();
        let books = vec![
            create_test_book("Book 1", "Author 1", 4.0, vec!["Fantasy".to_string()]),
            create_test_book("Book 2", "Author 2", 4.2, vec!["Sci-Fi".to_string()]),
            create_test_book("Book 3", "Author 3", 4.5, vec!["Mystery".to_string()]),
        ];
        let enhanced_query = EnhancedQuery::from_query("good books");

        let explanations =
            generator.generate_top_explanations("good books", &books, &enhanced_query, 2);

        assert_eq!(explanations.len(), 2); // Should only generate for top 2
    }
}
