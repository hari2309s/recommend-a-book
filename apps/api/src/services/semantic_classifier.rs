use crate::error::Result;
use tracing::{debug, info};

/// Semantic classifier using HuggingFace zero-shot classification
#[derive(Clone)]
pub struct SemanticClassifier {}

impl SemanticClassifier {
    /// Create a new semantic classifier
    pub fn new() -> Result<Self> {
        info!("Initialized SemanticClassifier (keyword-based)");
        Ok(Self {})
    }

    /// Extract author name from query using pattern matching
    pub fn extract_author(&self, query: &str) -> Option<String> {
        use regex::Regex;

        let author_patterns = vec![
            Regex::new(r"(?i)(?:books?\s+)?(?:written\s+)?by\s+([a-zA-Z\s.'-]+?)(?:\s+books?|\s+novels?|\s*$)").unwrap(),
            Regex::new(r"(?i)(?:works?\s+)?(?:of|from)\s+([a-zA-Z\s.'-]+?)(?:\s+books?|\s+novels?|\s*$)").unwrap(),
            Regex::new(r"(?i)([a-zA-Z\s.'-]+?)'s\s+(?:books?|novels?|works?|writings?)").unwrap(),
            Regex::new(r"(?i)author:?\s*([a-zA-Z\s.'-]+?)(?:\s|$)").unwrap(),
        ];

        for pattern in author_patterns {
            if let Some(captures) = pattern.captures(query) {
                if let Some(author_match) = captures.get(1) {
                    let author = author_match.as_str().trim().to_string();
                    if !author.is_empty() && author.len() > 2 {
                        debug!("Extracted author: {}", author);
                        return Some(author);
                    }
                }
            }
        }

        None
    }

    /// Extract temporal information from query
    pub fn extract_temporal_info(&self, query: &str) -> Option<TemporalFilter> {
        let query_lower = query.to_lowercase();

        if query_lower.contains("recent")
            || query_lower.contains("new")
            || query_lower.contains("modern")
            || query_lower.contains("contemporary")
        {
            return Some(TemporalFilter {
                min_year: Some(2015),
                max_year: None,
                recency_boost: 1.3,
            });
        }

        if query_lower.contains("classic") || query_lower.contains("old") {
            return Some(TemporalFilter {
                min_year: None,
                max_year: Some(2000),
                recency_boost: 0.8,
            });
        }

        None
    }

    /// Check if query is asking for similar books
    pub fn is_similar_query(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        query_lower.contains("similar to")
            || query_lower.contains("like")
            || query_lower.contains("reminds me of")
            || query_lower.contains("in the style of")
    }
}

/// Temporal filter information extracted from query
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TemporalFilter {
    pub min_year: Option<i32>,
    pub max_year: Option<i32>,
    pub recency_boost: f32,
}

/// Enhanced query information from semantic classification
#[derive(Debug, Clone)]
pub struct SemanticQueryInfo {
    pub original_query: String,
    pub themes: Vec<(String, f32)>,
    pub author: Option<String>,
    pub temporal_filter: Option<TemporalFilter>,
    pub is_similar_query: bool,
    pub semantic_tags: Vec<String>,
}

impl SemanticClassifier {
    /// Analyze query and extract all relevant information
    pub async fn analyze_query(&self, query: &str) -> Result<SemanticQueryInfo> {
        // Extract keywords for display (no ML needed)
        let keywords = self.extract_keywords(query);

        // Extract author if mentioned
        let author = self.extract_author(query);

        // Extract temporal information
        let temporal_filter = self.extract_temporal_info(query);

        // Check if it's a similarity query
        let is_similar_query = self.is_similar_query(query);

        // Create semantic tags from keywords
        let semantic_tags = keywords.clone();

        Ok(SemanticQueryInfo {
            original_query: query.to_string(),
            themes: keywords.into_iter().map(|k| (k, 0.8)).collect(), // Uniform confidence
            author,
            temporal_filter,
            is_similar_query,
            semantic_tags,
        })
    }

    /// Extract meaningful keywords from the query (simple approach)
    fn extract_keywords(&self, query: &str) -> Vec<String> {
        let stop_words: std::collections::HashSet<&str> = [
            // Articles & prepositions
            "the",
            "a",
            "an",
            "and",
            "or",
            "but",
            "in",
            "on",
            "at",
            "to",
            "for",
            "of",
            "with",
            "by",
            "from",
            "about",
            "as",
            "into",
            "through",
            "during",
            // Common search terms
            "books",
            "book",
            "novel",
            "novels",
            "story",
            "stories",
            "read",
            "reading",
            "recommend",
            "suggestion",
            "find",
            "looking",
            "want",
            "need",
            "please",
            "give",
            "show",
            "tell",
            "help",
            "any",
            "some",
            "good",
            "best",
            "great",
            // Question words
            "what",
            "where",
            "when",
            "who",
            "which",
            "how",
            "why",
            // Pronouns
            "i",
            "me",
            "my",
            "you",
            "your",
            "it",
            "its",
            "that",
            "this",
            "these",
            "those",
        ]
        .iter()
        .cloned()
        .collect();

        query
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
            .filter(|word| !word.is_empty() && word.len() > 3 && !stop_words.contains(word))
            .take(5) // Limit to top 5 keywords
            .map(|s| s.to_string())
            .collect()
    }
}
