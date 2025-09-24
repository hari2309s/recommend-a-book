//! Intent detection for user queries.
//!
//! This module handles parsing and categorizing user queries into specific intents,
//! such as searching for books by a particular author, genre, or similar to a book.

use lazy_static::lazy_static;
use log::{debug, info};
use regex::Regex;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::error::ApiError;

/// Regular expression patterns for detecting author-related queries
pub static AUTHOR_PATTERNS: &[&str] = &[
    r"(?i)^books?\s+by\s+(.+)$",
    r"(?i)^(?:show|find|get)\s+(?:me\s+)?books?\s+by\s+(.+)$",
    r"(?i)^author[:\s]+(.+)$",
    r"(?i)^(.+)'s books$",
    r"(?i)^what\s+(?:books?|else)?\s+(?:did|has)\s+(.+)\s+writ(?:e|ten)$",
];

/// Regular expression patterns for detecting genre-related queries
pub static GENRE_PATTERNS: &[&str] = &[
    r"(?i)^(?:books?\s+in|books?\s+about|books?\s+on)\s+(.+)$",
    r"(?i)^(.+)\s+books?$",
    r"(?i)^genre[:\s]+(.+)$",
    r"(?i)^books?\s+(?:about|on|in)\s+(.+)$",
    r"(?i)^(?:show|find|get)\s+(?:me\s+)?(.+)\s+books?$",
    r"(?i)^(?:show|find|get)\s+(?:me\s+)?books?\s+(?:about|on|in)\s+(.+)$",
    r"(?i)^i\s+want\s+to\s+read\s+(?:about|on)?\s+(.+)$",
    r"(?i)^i'm\s+looking\s+for\s+(?:a\s+book\s+)?(?:about|on)?\s+(.+)$",
    r"(?i)^recommend\s+(?:me\s+)?(?:a\s+book\s+)?(?:about|on)?\s+(.+)$",
];

/// Regular expression patterns for detecting similarity-based queries
pub static SIMILAR_PATTERNS: &[&str] = &[
    r"(?i)^(?:books?|something)?\s+(?:similar|like)\s+(?:to)?\s+(.+)$",
    r"(?i)^(?:if|when)\s+(?:i|you)\s+(?:like[d]?|enjoy[ed]?|loved)\s+(.+)$",
    r"(?i)^i\s+(?:like[d]?|enjoy[ed]?|loved)\s+(.+)$",
];

/// Common book genres for recognizing genre-related queries
pub static COMMON_GENRES: &[&str] = &[
    "fantasy",
    "sci-fi",
    "science fiction",
    "mystery",
    "thriller",
    "romance",
    "historical fiction",
    "history",
    "non-fiction",
    "biography",
    "autobiography",
    "memoir",
    "self-help",
    "business",
    "finance",
    "philosophy",
    "psychology",
    "science",
    "travel",
    "adventure",
    "horror",
    "poetry",
    "drama",
    "comedy",
    "young adult",
    "children",
    "graphic novel",
    "comics",
    "classic",
    "literary fiction",
    "crime",
    "detective",
    "dystopian",
    "urban fantasy",
    "paranormal",
    "supernatural",
    "western",
    "action",
    "magical realism",
    "mythology",
    "folklore",
    "fairy tale",
    "short story",
    "essay",
    "satire",
    "political",
    "religious",
    "spiritual",
    "christian",
];

/// The detected intent of a user query
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryIntent {
    /// Intent to find books by a specific author
    Author {
        /// The author's name
        name: String,
        /// The original query text
        original_query: String,
    },
    /// Intent to find books in a specific genre
    Genre {
        /// The genre
        genre: String,
        /// The original query text
        original_query: String,
    },
    /// Intent to find books similar to something
    SimilarTo {
        /// The original query text
        original_query: String,
    },
    /// General search query with no specific categorization
    General {
        /// The query text
        query: String,
    },
}

/// Cache for storing parsed query intents to avoid repeated parsing of the same query
pub struct IntentCache {
    /// Cached intents with timestamps for TTL management
    entries: Arc<Mutex<HashMap<String, (QueryIntent, Instant)>>>,
    /// TTL in seconds for cache entries
    ttl: u64,
}

impl IntentCache {
    /// Create a new intent cache with the specified TTL
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            ttl: ttl_seconds,
        }
    }

    /// Get a cached intent for a query if it exists and is not expired
    pub fn get(&self, query: &str) -> Option<QueryIntent> {
        if let Ok(cache) = self.entries.lock() {
            if let Some((intent, timestamp)) = cache.get(query) {
                if timestamp.elapsed() < Duration::from_secs(self.ttl) {
                    debug!("Intent cache hit for query: {}", query);
                    return Some(intent.clone());
                }
            }
        }
        None
    }

    /// Set a query intent in the cache
    pub fn set(&self, query: String, intent: QueryIntent) -> Result<(), ApiError> {
        match self.entries.lock() {
            Ok(mut cache) => {
                cache.insert(query, (intent, Instant::now()));
                Ok(())
            }
            Err(e) => Err(
                ApiError::internal_error(format!("Failed to acquire cache lock: {}", e))
                    .with_context("intent_cache")
                    .with_operation("set"),
            ),
        }
    }

    /// Clean up expired entries in the cache
    pub fn cleanup(&self) -> Result<(), ApiError> {
        match self.entries.lock() {
            Ok(mut cache) => {
                let before_count = cache.len();
                cache.retain(|_, (_, timestamp)| {
                    timestamp.elapsed() < Duration::from_secs(self.ttl)
                });
                let removed = before_count - cache.len();

                if removed > 0 {
                    debug!("Removed {} expired entries from intent cache", removed);
                }

                Ok(())
            }
            Err(e) => Err(
                ApiError::internal_error(format!("Failed to acquire cache lock: {}", e))
                    .with_context("intent_cache")
                    .with_operation("cleanup"),
            ),
        }
    }
}

/// Parse a user query to determine the intent
#[allow(dead_code)]
pub fn parse_query_intent(query: &str) -> QueryIntent {
    let trimmed = query.trim();

    if trimmed.is_empty() {
        return QueryIntent::General {
            query: trimmed.to_string(),
        };
    }

    // Try to match author patterns
    for pattern in AUTHOR_PATTERNS {
        if let Some(captures) = Regex::new(pattern).unwrap().captures(trimmed) {
            if let Some(author_match) = captures.get(1) {
                let author = author_match.as_str().trim();
                info!("Detected author intent: '{}'", author);
                return QueryIntent::Author {
                    name: author.to_string(),
                    original_query: trimmed.to_string(),
                };
            }
        }
    }

    // Try to match genre patterns and check against common genres
    for pattern in GENRE_PATTERNS {
        if let Some(captures) = Regex::new(pattern).unwrap().captures(trimmed) {
            if let Some(genre_match) = captures.get(1) {
                let genre = genre_match.as_str().trim();

                // Check if the extracted text matches a known genre
                if COMMON_GENRES
                    .iter()
                    .any(|&g| genre.to_lowercase().contains(&g.to_lowercase()))
                {
                    info!("Detected genre intent: '{}'", genre);
                    return QueryIntent::Genre {
                        genre: genre.to_string(),
                        original_query: trimmed.to_string(),
                    };
                }
            }
        }
    }

    // Try to match similarity patterns
    for pattern in SIMILAR_PATTERNS {
        if let Some(captures) = Regex::new(pattern).unwrap().captures(trimmed) {
            if let Some(_) = captures.get(1) {
                info!("Detected similarity intent for: '{}'", trimmed);
                return QueryIntent::SimilarTo {
                    original_query: trimmed.to_string(),
                };
            }
        }
    }

    // Default to general search if no specific intent is detected
    info!(
        "No specific intent detected, using general search for: '{}'",
        trimmed
    );
    QueryIntent::General {
        query: trimmed.to_string(),
    }
}

/// Optimized query intent detection using pre-compiled regular expressions
pub fn parse_query_intent_optimized(query: &str) -> QueryIntent {
    lazy_static! {
        static ref AUTHOR_REGEX: Vec<Regex> = AUTHOR_PATTERNS
            .iter()
            .map(|p| Regex::new(p).unwrap())
            .collect();
        static ref GENRE_REGEX: Vec<Regex> = GENRE_PATTERNS
            .iter()
            .map(|p| Regex::new(p).unwrap())
            .collect();
        static ref SIMILAR_REGEX: Vec<Regex> = SIMILAR_PATTERNS
            .iter()
            .map(|p| Regex::new(p).unwrap())
            .collect();
    }

    let trimmed = query.trim();

    if trimmed.is_empty() {
        return QueryIntent::General {
            query: trimmed.to_string(),
        };
    }

    // Try to match author patterns
    for regex in AUTHOR_REGEX.iter() {
        if let Some(captures) = regex.captures(trimmed) {
            if let Some(author_match) = captures.get(1) {
                let author = author_match.as_str().trim();
                info!("Detected author intent: '{}'", author);
                return QueryIntent::Author {
                    name: author.to_string(),
                    original_query: trimmed.to_string(),
                };
            }
        }
    }

    // Try to match genre patterns and check against common genres
    for regex in GENRE_REGEX.iter() {
        if let Some(captures) = regex.captures(trimmed) {
            if let Some(genre_match) = captures.get(1) {
                let genre = genre_match.as_str().trim();

                // Check if the extracted text matches a known genre
                if COMMON_GENRES
                    .iter()
                    .any(|&g| genre.to_lowercase().contains(&g.to_lowercase()))
                {
                    info!("Detected genre intent: '{}'", genre);
                    return QueryIntent::Genre {
                        genre: genre.to_string(),
                        original_query: trimmed.to_string(),
                    };
                }
            }
        }
    }

    // Try to match similarity patterns
    for regex in SIMILAR_REGEX.iter() {
        if let Some(captures) = regex.captures(trimmed) {
            if let Some(_) = captures.get(1) {
                info!("Detected similarity intent for: '{}'", trimmed);
                return QueryIntent::SimilarTo {
                    original_query: trimmed.to_string(),
                };
            }
        }
    }

    // Default to general search if no specific intent is detected
    info!(
        "No specific intent detected, using general search for: '{}'",
        trimmed
    );
    QueryIntent::General {
        query: trimmed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_author_intent_detection() {
        assert!(matches!(
            parse_query_intent_optimized("books by J.K. Rowling"),
            QueryIntent::Author { name, .. } if name == "J.K. Rowling"
        ));

        assert!(matches!(
            parse_query_intent_optimized("show me books by Stephen King"),
            QueryIntent::Author { name, .. } if name == "Stephen King"
        ));
    }

    #[test]
    fn test_genre_intent_detection() {
        assert!(matches!(
            parse_query_intent_optimized("fantasy books"),
            QueryIntent::Genre { genre, .. } if genre == "fantasy"
        ));

        assert!(matches!(
            parse_query_intent_optimized("books about science fiction"),
            QueryIntent::Genre { genre, .. } if genre == "science fiction"
        ));
    }

    #[test]
    fn test_similar_intent_detection() {
        assert!(matches!(
            parse_query_intent_optimized("books similar to Harry Potter"),
            QueryIntent::SimilarTo { .. }
        ));

        assert!(matches!(
            parse_query_intent_optimized("if you liked The Hobbit"),
            QueryIntent::SimilarTo { .. }
        ));
    }

    #[test]
    fn test_general_intent_detection() {
        assert!(matches!(
            parse_query_intent_optimized("how to program in rust"),
            QueryIntent::General { query } if query == "how to program in rust"
        ));
    }
}
