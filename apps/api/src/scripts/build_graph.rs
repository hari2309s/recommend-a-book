use anyhow::{Context, Result};
use log::{error, info, warn};
use recommend_a_book_api::{
    config::Config,
    ml::huggingface_embedder::HuggingFaceEmbedder,
    services::{
        neo4j::{BookRelationship, Neo4jClient, RelationType},
        pinecone::Pinecone,
    },
};
use std::collections::HashMap;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Calculate cosine similarity between two embeddings
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

async fn build_book_graph() -> Result<()> {
    info!("Starting graph building process...");

    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;

    // Initialize Neo4j client
    let neo4j_uri =
        env::var("APP_NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let neo4j_user = env::var("APP_NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let neo4j_password = env::var("APP_NEO4J_PASSWORD")
        .context("Missing APP_NEO4J_PASSWORD environment variable")?;

    info!("Connecting to Neo4j at {}", neo4j_uri);
    let neo4j = Neo4jClient::new(&neo4j_uri, &neo4j_user, &neo4j_password)
        .await
        .context("Failed to initialize Neo4j client")?;

    // Initialize Pinecone to fetch books
    info!("Initializing Pinecone client...");
    let pinecone = Pinecone::new(
        &config.pinecone_api_key,
        &config.pinecone_environment,
        &config.pinecone_index,
    )
    .await
    .context("Failed to initialize Pinecone client")?;

    // Initialize HuggingFace embedder
    info!("Initializing HuggingFace embedder...");
    let embedder = HuggingFaceEmbedder::new()
        .await
        .context("Failed to initialize HuggingFace embedder")?;

    // Fetch all books from Pinecone (using a broad query)
    info!("Fetching books from Pinecone...");
    let all_books = pinecone.query_vector(&vec![0.0; 512], 10000).await?;
    info!("Retrieved {} books from Pinecone", all_books.len());

    // Clear existing graph if requested
    if env::var("CLEAR_GRAPH").unwrap_or_default() == "true" {
        info!("Clearing existing graph...");
        neo4j.clear_graph().await?;
    }

    // Add all books as nodes
    info!("Adding books to Neo4j graph...");
    let batch_size = 100;
    for (i, chunk) in all_books.chunks(batch_size).enumerate() {
        info!(
            "Processing batch {}/{}",
            i + 1,
            all_books.len().div_ceil(batch_size)
        );
        neo4j.add_books_batch(chunk).await?;
    }

    // Build relationships
    info!("Building relationships between books...");

    // Group books by author
    let mut books_by_author: HashMap<String, Vec<&recommend_a_book_api::models::Book>> =
        HashMap::new();
    for book in &all_books {
        if let Some(author) = &book.author {
            books_by_author
                .entry(author.to_lowercase())
                .or_insert_with(Vec::new)
                .push(book);
        }
    }

    // Create SAME_AUTHOR relationships
    info!("Creating SAME_AUTHOR relationships...");
    let mut same_author_rels = Vec::new();
    for books in books_by_author.values() {
        if books.len() > 1 {
            for i in 0..books.len() {
                for j in (i + 1)..books.len() {
                    if let (Some(id1), Some(id2)) = (&books[i].id, &books[j].id) {
                        same_author_rels.push(BookRelationship {
                            from_id: id1.clone(),
                            to_id: id2.clone(),
                            relation_type: RelationType::SameAuthor,
                            weight: 1.0,
                            metadata: None,
                        });

                        // Bidirectional
                        same_author_rels.push(BookRelationship {
                            from_id: id2.clone(),
                            to_id: id1.clone(),
                            relation_type: RelationType::SameAuthor,
                            weight: 1.0,
                            metadata: None,
                        });
                    }
                }
            }
        }
    }
    info!(
        "Created {} SAME_AUTHOR relationships",
        same_author_rels.len()
    );

    // Create relationships in batches
    for (i, chunk) in same_author_rels.chunks(100).enumerate() {
        info!(
            "Adding SAME_AUTHOR batch {}/{}",
            i + 1,
            same_author_rels.len().div_ceil(100)
        );
        neo4j.create_relationships_batch(chunk).await?;
    }

    // Group books by genre
    let mut books_by_genre: HashMap<String, Vec<&recommend_a_book_api::models::Book>> =
        HashMap::new();
    for book in &all_books {
        for category in &book.categories {
            books_by_genre
                .entry(category.to_lowercase())
                .or_insert_with(Vec::new)
                .push(book);
        }
    }

    // Create SAME_GENRE relationships
    info!("Creating SAME_GENRE relationships...");
    let mut same_genre_rels = Vec::new();
    for books in books_by_genre.values() {
        if books.len() > 1 && books.len() < 500 {
            // Limit to avoid too many relationships
            for i in 0..books.len().min(20) {
                for j in (i + 1)..books.len().min(20) {
                    if let (Some(id1), Some(id2)) = (&books[i].id, &books[j].id) {
                        same_genre_rels.push(BookRelationship {
                            from_id: id1.clone(),
                            to_id: id2.clone(),
                            relation_type: RelationType::SameGenre,
                            weight: 0.8,
                            metadata: None,
                        });
                    }
                }
            }
        }
    }
    info!("Created {} SAME_GENRE relationships", same_genre_rels.len());

    for (i, chunk) in same_genre_rels.chunks(100).enumerate() {
        info!(
            "Adding SAME_GENRE batch {}/{}",
            i + 1,
            same_genre_rels.len().div_ceil(100)
        );
        neo4j.create_relationships_batch(chunk).await?;
    }

    // Create SIMILAR_TO relationships using embeddings
    info!("Creating SIMILAR_TO relationships using semantic similarity...");
    let similarity_threshold = 0.85; // High threshold for semantic similarity

    // Sample books for similarity (processing all would be too expensive)
    let sample_size = 500.min(all_books.len());
    info!(
        "Processing {} books for similarity relationships",
        sample_size
    );

    let mut book_embeddings: HashMap<String, Vec<f32>> = HashMap::new();

    for (idx, book) in all_books.iter().take(sample_size).enumerate() {
        if idx % 50 == 0 {
            info!("Processing embeddings: {}/{}", idx, sample_size);
        }

        if let Some(book_id) = &book.id {
            // Create text for embedding
            let text = format!(
                "{} {} {}",
                book.title.as_deref().unwrap_or(""),
                book.author.as_deref().unwrap_or(""),
                book.categories.join(" ")
            );

            match embedder.encode(&text).await {
                Ok(embedding) => {
                    book_embeddings.insert(book_id.clone(), embedding);
                }
                Err(e) => {
                    warn!("Failed to generate embedding for book {}: {}", book_id, e);
                }
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    info!("Generated embeddings for {} books", book_embeddings.len());

    // Calculate similarities
    let mut similar_rels = Vec::new();
    let book_ids: Vec<String> = book_embeddings.keys().cloned().collect();

    for i in 0..book_ids.len() {
        if i % 50 == 0 {
            info!("Calculating similarities: {}/{}", i, book_ids.len());
        }

        let id1 = &book_ids[i];
        let emb1 = &book_embeddings[id1];

        for j in (i + 1)..book_ids.len() {
            let id2 = &book_ids[j];
            let emb2 = &book_embeddings[id2];

            let similarity = cosine_similarity(emb1, emb2);

            if similarity >= similarity_threshold {
                similar_rels.push(BookRelationship {
                    from_id: id1.clone(),
                    to_id: id2.clone(),
                    relation_type: RelationType::SimilarTo,
                    weight: similarity,
                    metadata: Some(serde_json::json!({
                        "similarity_score": similarity
                    })),
                });

                // Bidirectional
                similar_rels.push(BookRelationship {
                    from_id: id2.clone(),
                    to_id: id1.clone(),
                    relation_type: RelationType::SimilarTo,
                    weight: similarity,
                    metadata: Some(serde_json::json!({
                        "similarity_score": similarity
                    })),
                });
            }
        }
    }

    info!("Created {} SIMILAR_TO relationships", similar_rels.len());

    for (i, chunk) in similar_rels.chunks(100).enumerate() {
        info!(
            "Adding SIMILAR_TO batch {}/{}",
            i + 1,
            similar_rels.len().div_ceil(100)
        );
        neo4j.create_relationships_batch(chunk).await?;
    }

    // Get final statistics
    let stats = neo4j.get_graph_stats().await?;
    info!("Graph building complete!");
    info!("Final statistics:");
    info!("  Total books: {}", stats.total_books);
    info!("  Total relationships: {}", stats.total_relationships);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "build_graph=info,recommend_a_book_api=info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_level(true),
        )
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Check for required environment variables
    let required_vars = [
        "APP_HUGGINGFACE_API_KEY",
        "APP_PINECONE_API_KEY",
        "APP_PINECONE_ENV",
        "APP_PINECONE_INDEX_NAME",
        "APP_NEO4J_PASSWORD",
    ];

    for var in &required_vars {
        if env::var(var).is_err() {
            error!("Missing required environment variable: {}", var);
            std::process::exit(1);
        }
    }

    info!("Book Graph Builder");
    info!("==================");

    match build_book_graph().await {
        Ok(_) => {
            info!("✅ Graph building completed successfully!");
            std::process::exit(0);
        }
        Err(e) => {
            error!("❌ Graph building failed: {}", e);
            std::process::exit(1);
        }
    }
}
