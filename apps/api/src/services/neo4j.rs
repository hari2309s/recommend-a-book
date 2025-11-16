use crate::error::{ApiError, Result};
use crate::models::Book;
use neo4rs::{Graph, Query};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};
use utoipa::ToSchema;

/// Relationship types between books
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum RelationType {
    SimilarTo,
    SameAuthor,
    SameGenre,
    SameTheme,
    ReadNext,
    PartOfSeries,
}

#[allow(dead_code)]
impl RelationType {
    fn as_str(&self) -> &str {
        match self {
            RelationType::SimilarTo => "SIMILAR_TO",
            RelationType::SameAuthor => "SAME_AUTHOR",
            RelationType::SameGenre => "SAME_GENRE",
            RelationType::SameTheme => "SAME_THEME",
            RelationType::ReadNext => "READ_NEXT",
            RelationType::PartOfSeries => "PART_OF_SERIES",
        }
    }
}

/// Graph node representing a book
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BookNode {
    #[schema(example = "book-123")]
    pub id: String,
    #[schema(example = "The Hobbit")]
    pub title: String,
    #[schema(example = "J.R.R. Tolkien")]
    pub author: Option<String>,
    #[schema(example = json!(["Fantasy", "Adventure"]))]
    pub categories: Vec<String>,
    #[schema(example = 4.5)]
    pub rating: f32,
    #[schema(example = 1937)]
    pub year: Option<i32>,
    pub description: Option<String>,
}

impl From<&Book> for BookNode {
    fn from(book: &Book) -> Self {
        Self {
            id: book.id.clone().unwrap_or_default(),
            title: book.title.clone().unwrap_or_default(),
            author: book.author.clone(),
            categories: book.categories.clone(),
            rating: book.rating,
            year: book.year,
            description: book.description.clone(),
        }
    }
}

/// Graph relationship between books
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BookRelationship {
    pub from_id: String,
    pub to_id: String,
    pub relation_type: RelationType,
    pub weight: f32,
    pub metadata: Option<serde_json::Value>,
}

/// Response structure for graph queries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GraphResponse {
    pub nodes: Vec<BookNode>,
    pub relationships: Vec<GraphRelationshipResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GraphRelationshipResponse {
    pub from_id: String,
    pub to_id: String,
    pub relation_type: String,
    pub weight: f32,
}

/// Neo4j graph database client for book relationships
#[derive(Clone)]
pub struct Neo4jClient {
    graph: Arc<Graph>,
}

#[allow(dead_code)]
impl Neo4jClient {
    /// Create a new Neo4j client
    pub async fn new(uri: &str, username: &str, password: &str) -> Result<Self> {
        info!("Connecting to Neo4j at {}", uri);

        let graph = Graph::new(uri, username, password).await.map_err(|e| {
            ApiError::ExternalServiceError(format!("Failed to connect to Neo4j: {}", e))
        })?;

        info!("Successfully connected to Neo4j");

        let client = Self {
            graph: Arc::new(graph),
        };

        // Create constraints and indexes
        client.setup_constraints().await?;

        Ok(client)
    }

    /// Setup Neo4j constraints and indexes
    async fn setup_constraints(&self) -> Result<()> {
        info!("Setting up Neo4j constraints and indexes");

        // Create constraint for unique book IDs
        let constraint_query = Query::new(
            "CREATE CONSTRAINT book_id_unique IF NOT EXISTS FOR (b:Book) REQUIRE b.id IS UNIQUE"
                .to_string(),
        );

        self.graph
            .run(constraint_query)
            .await
            .map_err(|e| {
                warn!("Failed to create constraint (may already exist): {}", e);
                ApiError::ExternalServiceError(format!("Failed to create Neo4j constraint: {}", e))
            })
            .ok();

        // Create indexes for better query performance
        let index_queries = vec![
            "CREATE INDEX book_title_idx IF NOT EXISTS FOR (b:Book) ON (b.title)",
            "CREATE INDEX book_author_idx IF NOT EXISTS FOR (b:Book) ON (b.author)",
            "CREATE INDEX book_rating_idx IF NOT EXISTS FOR (b:Book) ON (b.rating)",
        ];

        for query_str in index_queries {
            let query = Query::new(query_str.to_string());
            self.graph
                .run(query)
                .await
                .map_err(|e| {
                    warn!("Failed to create index (may already exist): {}", e);
                    ApiError::ExternalServiceError(format!("Failed to create Neo4j index: {}", e))
                })
                .ok();
        }

        info!("Neo4j setup complete");
        Ok(())
    }

    /// Add a book node to the graph
    pub async fn add_book(&self, book: &Book) -> Result<()> {
        let node = BookNode::from(book);

        debug!("Adding book node: {}", node.title);

        let query = Query::new(
            "MERGE (b:Book {id: $id})
             SET b.title = $title,
                 b.author = $author,
                 b.categories = $categories,
                 b.rating = $rating,
                 b.year = $year,
                 b.description = $description"
                .to_string(),
        )
        .param("id", node.id)
        .param("title", node.title)
        .param("author", node.author.unwrap_or_default())
        .param("categories", node.categories)
        .param("rating", node.rating as f64)
        .param("year", node.year.unwrap_or(0) as i64)
        .param("description", node.description.unwrap_or_default());

        self.graph.run(query).await.map_err(|e| {
            ApiError::ExternalServiceError(format!("Failed to add book to Neo4j: {}", e))
        })?;

        Ok(())
    }

    /// Add multiple books in batch
    pub async fn add_books_batch(&self, books: &[Book]) -> Result<()> {
        info!("Adding batch of {} books to Neo4j", books.len());

        for book in books {
            if let Err(e) = self.add_book(book).await {
                warn!(
                    "Failed to add book '{}': {}",
                    book.title.as_deref().unwrap_or("unknown"),
                    e
                );
            }
        }

        info!("Batch insert complete");
        Ok(())
    }

    /// Create a relationship between two books
    pub async fn create_relationship(&self, relationship: &BookRelationship) -> Result<()> {
        debug!(
            "Creating relationship: {} -> {} ({})",
            relationship.from_id,
            relationship.to_id,
            relationship.relation_type.as_str()
        );

        let query = Query::new(format!(
            "MATCH (a:Book {{id: $from_id}}), (b:Book {{id: $to_id}})
                 MERGE (a)-[r:{}]->(b)
                 SET r.weight = $weight",
            relationship.relation_type.as_str()
        ))
        .param("from_id", relationship.from_id.clone())
        .param("to_id", relationship.to_id.clone())
        .param("weight", relationship.weight as f64);

        self.graph.run(query).await.map_err(|e| {
            ApiError::ExternalServiceError(format!("Failed to create relationship: {}", e))
        })?;

        Ok(())
    }

    /// Create relationships in batch
    pub async fn create_relationships_batch(
        &self,
        relationships: &[BookRelationship],
    ) -> Result<()> {
        info!("Creating batch of {} relationships", relationships.len());

        for rel in relationships {
            if let Err(e) = self.create_relationship(rel).await {
                warn!("Failed to create relationship: {}", e);
            }
        }

        Ok(())
    }

    /// Get books similar to a given book ID
    pub async fn get_similar_books(&self, book_id: &str, limit: usize) -> Result<Vec<BookNode>> {
        debug!("Finding similar books for: {}", book_id);

        let query = Query::new(
            "MATCH (b:Book {id: $book_id})-[r:SIMILAR_TO]->(similar:Book)
             RETURN similar.id as id, similar.title as title, similar.author as author,
                    similar.categories as categories, similar.rating as rating,
                    similar.year as year, similar.description as description,
                    r.weight as weight
             ORDER BY r.weight DESC
             LIMIT $limit"
                .to_string(),
        )
        .param("book_id", book_id.to_string())
        .param("limit", limit as i64);

        let mut result = self.graph.execute(query).await.map_err(|e| {
            ApiError::ExternalServiceError(format!("Failed to query similar books: {}", e))
        })?;

        let mut books = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            let book = BookNode {
                id: row.get::<String>("id").unwrap_or_default(),
                title: row.get::<String>("title").unwrap_or_default(),
                author: row.get::<String>("author").ok(),
                categories: row.get::<Vec<String>>("categories").unwrap_or_default(),
                rating: row.get::<f64>("rating").unwrap_or(0.0) as f32,
                year: row.get::<i64>("year").ok().map(|y| y as i32),
                description: row.get::<String>("description").ok(),
            };
            books.push(book);
        }

        debug!("Found {} similar books", books.len());
        Ok(books)
    }

    /// Get books by the same author
    pub async fn get_books_by_same_author(
        &self,
        book_id: &str,
        limit: usize,
    ) -> Result<Vec<BookNode>> {
        debug!("Finding books by same author for: {}", book_id);

        let query = Query::new(
            "MATCH (b:Book {id: $book_id})-[r:SAME_AUTHOR]->(other:Book)
             RETURN other.id as id, other.title as title, other.author as author,
                    other.categories as categories, other.rating as rating,
                    other.year as year, other.description as description
             ORDER BY other.rating DESC
             LIMIT $limit"
                .to_string(),
        )
        .param("book_id", book_id.to_string())
        .param("limit", limit as i64);

        let mut result = self.graph.execute(query).await.map_err(|e| {
            ApiError::ExternalServiceError(format!("Failed to query books by author: {}", e))
        })?;

        let mut books = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            let book = BookNode {
                id: row.get::<String>("id").unwrap_or_default(),
                title: row.get::<String>("title").unwrap_or_default(),
                author: row.get::<String>("author").ok(),
                categories: row.get::<Vec<String>>("categories").unwrap_or_default(),
                rating: row.get::<f64>("rating").unwrap_or(0.0) as f32,
                year: row.get::<i64>("year").ok().map(|y| y as i32),
                description: row.get::<String>("description").ok(),
            };
            books.push(book);
        }

        Ok(books)
    }

    /// Get the full graph neighborhood for a book
    pub async fn get_book_graph(&self, book_id: &str, depth: usize) -> Result<GraphResponse> {
        info!(
            "Getting graph neighborhood for book: {} (depth: {})",
            book_id, depth
        );

        let query = Query::new(format!(
            "MATCH path = (b:Book {{id: $book_id}})-[*1..{}]-(related:Book)
                 WITH b, related, relationships(path) as rels
                 RETURN DISTINCT
                        b.id as source_id, b.title as source_title, b.author as source_author,
                        b.categories as source_categories, b.rating as source_rating,
                        related.id as target_id, related.title as target_title,
                        related.author as target_author, related.categories as target_categories,
                        related.rating as target_rating, related.year as target_year,
                        related.description as target_description,
                        [r in rels | type(r)] as rel_types,
                        [r in rels | r.weight] as weights
                 LIMIT 100",
            depth
        ))
        .param("book_id", book_id.to_string());

        let mut result = self.graph.execute(query).await.map_err(|e| {
            ApiError::ExternalServiceError(format!("Failed to query book graph: {}", e))
        })?;

        let mut nodes_map = std::collections::HashMap::new();
        let mut relationships = Vec::new();

        // Add the source book first
        if let Some(source_book) = self.get_book_by_id(book_id).await? {
            nodes_map.insert(source_book.id.clone(), source_book);
        }

        while let Ok(Some(row)) = result.next().await {
            // Add target node
            let target_node = BookNode {
                id: row.get::<String>("target_id").unwrap_or_default(),
                title: row.get::<String>("target_title").unwrap_or_default(),
                author: row.get::<String>("target_author").ok(),
                categories: row
                    .get::<Vec<String>>("target_categories")
                    .unwrap_or_default(),
                rating: row.get::<f64>("target_rating").unwrap_or(0.0) as f32,
                year: row.get::<i64>("target_year").ok().map(|y| y as i32),
                description: row.get::<String>("target_description").ok(),
            };

            let source_id = row.get::<String>("source_id").unwrap_or_default();
            let target_id = target_node.id.clone();

            nodes_map.insert(target_id.clone(), target_node);

            // Add relationships
            if let Ok(rel_types) = row.get::<Vec<String>>("rel_types") {
                if let Ok(weights) = row.get::<Vec<f64>>("weights") {
                    for (rel_type, weight) in rel_types.iter().zip(weights.iter()) {
                        relationships.push(GraphRelationshipResponse {
                            from_id: source_id.clone(),
                            to_id: target_id.clone(),
                            relation_type: rel_type.clone(),
                            weight: *weight as f32,
                        });
                    }
                }
            }
        }

        let nodes: Vec<BookNode> = nodes_map.into_values().collect();

        info!(
            "Graph query returned {} nodes and {} relationships",
            nodes.len(),
            relationships.len()
        );

        Ok(GraphResponse {
            nodes,
            relationships,
        })
    }

    /// Get a book by ID
    pub async fn get_book_by_id(&self, book_id: &str) -> Result<Option<BookNode>> {
        let query = Query::new(
            "MATCH (b:Book {id: $book_id})
             RETURN b.id as id, b.title as title, b.author as author,
                    b.categories as categories, b.rating as rating,
                    b.year as year, b.description as description"
                .to_string(),
        )
        .param("book_id", book_id.to_string());

        let mut result =
            self.graph.execute(query).await.map_err(|e| {
                ApiError::ExternalServiceError(format!("Failed to get book: {}", e))
            })?;

        if let Ok(Some(row)) = result.next().await {
            Ok(Some(BookNode {
                id: row.get::<String>("id").unwrap_or_default(),
                title: row.get::<String>("title").unwrap_or_default(),
                author: row.get::<String>("author").ok(),
                categories: row.get::<Vec<String>>("categories").unwrap_or_default(),
                rating: row.get::<f64>("rating").unwrap_or(0.0) as f32,
                year: row.get::<i64>("year").ok().map(|y| y as i32),
                description: row.get::<String>("description").ok(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Search books by title pattern
    pub async fn search_books(&self, title_pattern: &str, limit: usize) -> Result<Vec<BookNode>> {
        let query = Query::new(
            "MATCH (b:Book)
             WHERE toLower(b.title) CONTAINS toLower($pattern)
             RETURN b.id as id, b.title as title, b.author as author,
                    b.categories as categories, b.rating as rating,
                    b.year as year, b.description as description
             ORDER BY b.rating DESC
             LIMIT $limit"
                .to_string(),
        )
        .param("pattern", title_pattern.to_string())
        .param("limit", limit as i64);

        let mut result = self.graph.execute(query).await.map_err(|e| {
            ApiError::ExternalServiceError(format!("Failed to search books: {}", e))
        })?;

        let mut books = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            let book = BookNode {
                id: row.get::<String>("id").unwrap_or_default(),
                title: row.get::<String>("title").unwrap_or_default(),
                author: row.get::<String>("author").ok(),
                categories: row.get::<Vec<String>>("categories").unwrap_or_default(),
                rating: row.get::<f64>("rating").unwrap_or(0.0) as f32,
                year: row.get::<i64>("year").ok().map(|y| y as i32),
                description: row.get::<String>("description").ok(),
            };
            books.push(book);
        }

        Ok(books)
    }

    /// Get statistics about the graph
    pub async fn get_graph_stats(&self) -> Result<GraphStats> {
        // Count total books
        let count_query = Query::new("MATCH (b:Book) RETURN count(b) as count".to_string());
        let mut result =
            self.graph.execute(count_query).await.map_err(|e| {
                ApiError::ExternalServiceError(format!("Failed to count books: {}", e))
            })?;

        let book_count = if let Ok(Some(row)) = result.next().await {
            row.get::<i64>("count").unwrap_or(0) as usize
        } else {
            0
        };

        // Count relationships
        let rel_query = Query::new("MATCH ()-[r]->() RETURN count(r) as count".to_string());
        let mut result = self.graph.execute(rel_query).await.map_err(|e| {
            ApiError::ExternalServiceError(format!("Failed to count relationships: {}", e))
        })?;

        let relationship_count = if let Ok(Some(row)) = result.next().await {
            row.get::<i64>("count").unwrap_or(0) as usize
        } else {
            0
        };

        Ok(GraphStats {
            total_books: book_count,
            total_relationships: relationship_count,
        })
    }

    /// Clear all data from the graph
    pub async fn clear_graph(&self) -> Result<()> {
        warn!("Clearing all data from Neo4j graph");

        let query = Query::new("MATCH (n) DETACH DELETE n".to_string());
        self.graph
            .run(query)
            .await
            .map_err(|e| ApiError::ExternalServiceError(format!("Failed to clear graph: {}", e)))?;

        info!("Graph cleared successfully");
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GraphStats {
    #[schema(example = 1000)]
    pub total_books: usize,
    #[schema(example = 5000)]
    pub total_relationships: usize,
}
