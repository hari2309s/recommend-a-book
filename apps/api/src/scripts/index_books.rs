use anyhow::{Context, Result};
use csv::ReaderBuilder;
use log::{error, info, warn};
use recommend_a_book_api::{
    config::Config, ml::huggingface_embedder::HuggingFaceEmbedder, models::Book,
    services::pinecone::Pinecone,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
    fs::File,
    path::PathBuf,
};
use tokio::time::{sleep, Duration};
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Deserialize)]
struct BookCsvRecord {
    #[serde(alias = "Title", alias = "title")]
    title: Option<String>,
    #[serde(
        alias = "Authors",
        alias = "Author",
        alias = "authors",
        alias = "author"
    )]
    authors: Option<String>,
    #[serde(alias = "Description", alias = "description")]
    description: Option<String>,
    #[serde(alias = "Categories", alias = "categories")]
    categories: Option<String>,
    #[serde(alias = "isbn13", alias = "ISBN13", alias = "ISBN", alias = "isbn")]
    isbn: Option<String>,
    #[serde(alias = "published_year", alias = "publishedYear", alias = "year")]
    published_year: Option<String>,
    #[serde(alias = "ratings_count", alias = "ratingsCount")]
    ratings_count: Option<String>,
    #[serde(alias = "average_rating", alias = "rating")]
    rating: Option<String>,
    #[serde(alias = "image_url", alias = "thumbnail", alias = "imageLinks")]
    thumbnail: Option<String>,
    #[serde(alias = "page_count", alias = "pageCount")]
    page_count: Option<String>,
    #[serde(alias = "language")]
    language: Option<String>,
    #[serde(alias = "publisher")]
    publisher: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct PineconeVector {
    id: String,
    values: Vec<f32>,
    metadata: serde_json::Value,
}

#[derive(Debug, Serialize, Clone)]
struct UpsertRequest {
    vectors: Vec<PineconeVector>,
    namespace: Option<String>,
}

/// Enhanced text preprocessing for better semantic understanding
fn preprocess_text(text: &str) -> String {
    text.trim()
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Clean and normalize author names for better matching
fn normalize_author(author: &str) -> String {
    author
        .trim()
        .split(&[',', ';', '|', '&'][..])
        .map(|name| name.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Extract and clean categories
fn normalize_categories(categories: &str) -> Vec<String> {
    categories
        .trim()
        .to_lowercase()
        .split(&['&', '|', ';', ','][..])
        .map(|cat| {
            cat.trim()
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|cat| !cat.is_empty() && cat.len() > 1)
        .collect()
}

/// Create a comprehensive searchable text representation
fn create_searchable_text(book: &Book) -> String {
    let mut parts = Vec::new();

    // Add title with emphasis
    if let Some(title) = &book.title {
        parts.push(format!("Title: {}", title));
        parts.push(title.clone()); // Add title again for emphasis
    }

    // Add author information
    if let Some(author) = &book.author {
        parts.push(format!("Author: {}", author));
        parts.push(format!("Written by {}", author));
    }

    // Add categories/genres
    if !book.categories.is_empty() {
        let categories_str = book.categories.join(", ");
        parts.push(format!("Genre: {}", categories_str));
        parts.push(format!("Categories: {}", categories_str));
    }

    // Add description if available
    if let Some(description) = &book.description {
        if !description.trim().is_empty() {
            let cleaned_desc = preprocess_text(description);
            if cleaned_desc.len() > 50 {
                // Only add substantial descriptions
                parts.push(format!("Description: {}", cleaned_desc));
            }
        }
    }

    // Add publisher and year for context
    if let Some(publisher) = &book.publisher {
        parts.push(format!("Publisher: {}", publisher));
    }

    if let Some(year) = book.year {
        parts.push(format!("Published: {}", year));
    }

    let result = parts.join(". ");
    debug!(
        "Created searchable text for '{}': {} chars",
        book.title.as_deref().unwrap_or("Unknown"),
        result.len()
    );
    result
}

/// Convert CSV record to Book model
fn csv_record_to_book(record: BookCsvRecord, row_index: usize) -> Option<Book> {
    // Require at least title
    let title = record.title?.trim().to_string();
    if title.is_empty() {
        return None;
    }

    // Clean and validate author
    let author = record.authors.as_ref().map(|a| normalize_author(a));
    if author.as_ref().is_none_or(|a| a.is_empty()) {
        warn!("Row {}: Book '{}' has no valid author", row_index, title);
    }

    // Process categories
    let categories = record
        .categories
        .as_ref()
        .map(|c| normalize_categories(c))
        .unwrap_or_else(|| vec!["General".to_string()]);

    // Generate a unique ID
    let id = record
        .isbn
        .clone()
        .filter(|isbn| !isbn.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "book-{}-{}",
                title
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(20)
                    .collect::<String>(),
                author
                    .as_deref()
                    .unwrap_or("unknown")
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .take(10)
                    .collect::<String>()
            )
        });

    Some(Book {
        id: Some(id),
        title: Some(title),
        author,
        description: record.description.filter(|d| !d.trim().is_empty()),
        categories,
        thumbnail: record.thumbnail.filter(|t| !t.trim().is_empty()),
        rating: record.rating.and_then(|r| r.parse().ok()).unwrap_or(0.0),
        year: record.published_year.and_then(|y| y.parse().ok()),
        isbn: record.isbn.filter(|i| !i.trim().is_empty()),
        page_count: record.page_count.and_then(|p| p.parse().ok()).or(Some(0)),
        ratings_count: record.ratings_count.and_then(|r| r.parse().ok()),
        language: record
            .language
            .filter(|l| !l.trim().is_empty())
            .or(Some("unknown".to_string())),
        publisher: record
            .publisher
            .filter(|p| !p.trim().is_empty())
            .or(Some("unknown".to_string())),
    })
}

/// Retry operation with exponential backoff
async fn retry_with_backoff<F, T, E>(
    operation: F,
    max_retries: u32,
    base_delay_ms: u64,
    operation_name: &str,
) -> Result<T>
where
    F: Fn()
        -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send>>,
    E: std::fmt::Display + Send,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(anyhow::anyhow!(
                        "{} failed after {} attempts: {}",
                        operation_name,
                        max_retries,
                        e
                    ));
                }
                let delay = base_delay_ms * 2u64.pow(attempt - 1);
                warn!(
                    "{} attempt {} failed, retrying in {}ms: {}",
                    operation_name, attempt, delay, e
                );
                sleep(Duration::from_millis(delay)).await;
            }
        }
    }
}

/// Upsert vectors to Pinecone with proper error handling
async fn upsert_vectors_to_pinecone(
    _pinecone: &Pinecone,
    vectors: Vec<PineconeVector>,
    batch_index: usize,
) -> Result<()> {
    let upsert_request = UpsertRequest {
        vectors: vectors.clone(),
        namespace: None,
    };

    retry_with_backoff(
        || {
            let request = upsert_request.clone();
            Box::pin(async move {
                // Create a simple HTTP client request since we need more control
                let client = reqwest::Client::new();
                let response = client
                    .post(("https://books-index-0rnb22r.svc.aped-4627-b74a.pinecone.io/vectors/upsert").to_owned())
                    .header("Api-Key", std::env::var("APP_PINECONE_API_KEY").unwrap())
                    .header("Content-Type", "application/json")
                    .json(&request)
                    .send()
                    .await?;

                let status = response.status();
                if status.is_success() {
                    Ok(())
                } else {
                    let error_text = response.text().await.unwrap_or_default();
                    Err(anyhow::anyhow!(
                        "Pinecone API error: {} - {}",
                        status,
                        error_text
                    ))
                }
            })
        },
        3,
        1000,
        &format!("Pinecone upsert for batch {}", batch_index),
    )
    .await
}

async fn index_books_from_csv(csv_path: PathBuf) -> Result<()> {
    info!("Starting book indexing process...");
    info!("CSV file: {}", csv_path.display());

    // Initialize services
    info!("Initializing HuggingFace embedder...");
    let embedder = HuggingFaceEmbedder::new()
        .await
        .context("Failed to initialize HuggingFace embedder")?;

    let (model_name, embedding_size) = embedder.model_info();
    info!(
        "Using model: {} ({}D embeddings)",
        model_name, embedding_size
    );

    info!("Initializing Pinecone client...");
    let config = Config::load().context("Failed to load configuration")?;
    let pinecone = Pinecone::new(
        &config.pinecone_api_key,
        &config.pinecone_environment,
        &config.pinecone_index,
    )
    .await
    .context("Failed to initialize Pinecone client")?;

    // Read and parse CSV
    info!("Reading CSV file...");
    let file = File::open(&csv_path)
        .with_context(|| format!("Failed to open CSV file: {}", csv_path.display()))?;

    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(file);

    let mut books = Vec::new();
    let mut processed_count = 0;
    let mut skipped_count = 0;

    // Process CSV records
    for (row_index, result) in reader.deserialize().enumerate() {
        let record: BookCsvRecord = match result {
            Ok(record) => record,
            Err(e) => {
                error!("Error parsing CSV row {}: {}", row_index + 1, e);
                skipped_count += 1;
                continue;
            }
        };

        if let Some(book) = csv_record_to_book(record, row_index + 1) {
            books.push(book);
            processed_count += 1;
        } else {
            skipped_count += 1;
        }

        if processed_count % 1000 == 0 {
            info!("Processed {} books...", processed_count);
        }
    }

    info!("CSV parsing complete:");
    info!("  ✅ Valid books: {}", books.len());
    info!("  ❌ Skipped rows: {}", skipped_count);

    if books.is_empty() {
        return Err(anyhow::anyhow!("No valid books found in CSV file"));
    }

    // Deduplicate books
    let mut seen_titles: HashMap<String, usize> = HashMap::new();
    let mut unique_books = Vec::new();
    let mut duplicate_count = 0;

    for book in books {
        let key = format!(
            "{}|{}",
            book.title.as_deref().unwrap_or("").to_lowercase(),
            book.author.as_deref().unwrap_or("").to_lowercase()
        );

        if seen_titles.contains_key(&key) {
            duplicate_count += 1;
        } else {
            seen_titles.entry(key).or_insert(unique_books.len());
            unique_books.push(book);
        }
    }

    info!("Deduplication complete:");
    info!("  ✅ Unique books: {}", unique_books.len());
    info!("  🔄 Duplicates removed: {}", duplicate_count);

    // Process books in batches
    let batch_size = 25; // Smaller batches for better reliability
    let total_batches = unique_books.len().div_ceil(batch_size);
    let mut successfully_indexed = 0;

    for (batch_index, batch) in unique_books.chunks(batch_size).enumerate() {
        info!(
            "Processing batch {}/{} ({} books)...",
            batch_index + 1,
            total_batches,
            batch.len()
        );

        // Create searchable texts
        let texts: Vec<String> = batch.iter().map(create_searchable_text).collect();

        // Generate embeddings
        let embeddings = match embedder.encode_batch(&texts).await {
            Ok(embeddings) => {
                debug!("Generated embeddings shape: {:?}", embeddings.dim());
                embeddings
            }
            Err(e) => {
                error!(
                    "Failed to generate embeddings for batch {}: {}",
                    batch_index + 1,
                    e
                );
                continue;
            }
        };

        // Create Pinecone vectors
        let mut vectors = Vec::new();
        for (book_idx, book) in batch.iter().enumerate() {
            let embedding_row = embeddings.row(book_idx);
            let embedding_vec: Vec<f32> = embedding_row.to_vec();

            let metadata =
                serde_json::to_value(book).context("Failed to serialize book metadata")?;

            vectors.push(PineconeVector {
                id: book.id.as_ref().unwrap().clone(),
                values: embedding_vec,
                metadata,
            });
        }

        // Upsert to Pinecone
        match upsert_vectors_to_pinecone(&pinecone, vectors, batch_index + 1).await {
            Ok(_) => {
                successfully_indexed += batch.len();
                info!(
                    "✅ Successfully indexed batch {}/{} ({} books)",
                    batch_index + 1,
                    total_batches,
                    batch.len()
                );
            }
            Err(e) => {
                error!("❌ Failed to index batch {}: {}", batch_index + 1, e);
            }
        }

        // Small delay between batches to avoid rate limiting
        sleep(Duration::from_millis(500)).await;
    }

    // Final statistics
    info!("🎉 Indexing process completed!");
    info!("  📚 Total processed: {}", unique_books.len());
    info!("  ✅ Successfully indexed: {}", successfully_indexed);
    info!(
        "  ❌ Failed to index: {}",
        unique_books.len() - successfully_indexed
    );

    // Generate some statistics about the indexed books
    let authors: HashSet<String> = unique_books
        .iter()
        .filter_map(|b| b.author.as_ref())
        .map(|a| a.to_lowercase())
        .collect();

    let categories: HashSet<String> = unique_books
        .iter()
        .flat_map(|b| &b.categories)
        .map(|c| c.to_lowercase())
        .collect();

    let avg_rating = unique_books
        .iter()
        .map(|b| b.rating)
        .filter(|&r| r > 0.0)
        .collect::<Vec<_>>();

    info!("📊 Dataset statistics:");
    info!("  Authors: {}", authors.len());
    info!("  Categories: {}", categories.len());
    if !avg_rating.is_empty() {
        let avg = avg_rating.iter().sum::<f32>() / avg_rating.len() as f32;
        info!("  Average rating: {:.2}", avg);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "index_books=info,recommend_a_book_api=info".into()),
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
    ];

    for var in &required_vars {
        if env::var(var).is_err() {
            error!("Missing required environment variable: {}", var);
            std::process::exit(1);
        }
    }

    // Get CSV file path from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <path_to_csv_file>", args[0]);
        eprintln!("Example: {} ./data/books.csv", args[0]);
        std::process::exit(1);
    }

    let csv_path = PathBuf::from(&args[1]);
    if !csv_path.exists() {
        error!("CSV file does not exist: {}", csv_path.display());
        std::process::exit(1);
    }

    info!("Book Indexing Tool");
    info!("=================");

    match index_books_from_csv(csv_path).await {
        Ok(_) => {
            info!("✅ Indexing completed successfully!");
            std::process::exit(0);
        }
        Err(e) => {
            error!("❌ Indexing failed: {}", e);
            std::process::exit(1);
        }
    }
}
