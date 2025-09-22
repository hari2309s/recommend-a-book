# Recommended Changes to Reduce Excessive Logging

## Overview

The current recommendation service has extremely verbose logging that's flooding the console with information about each book being scored. This makes it difficult to see important information and impacts performance.

## Changes Needed

### 1. In `perform_hybrid_search` method:

```rust
// Change this:
info!("Successfully encoded query '{}'. Embedding stats: length={}, avg={:.4}, min={:.4}, max={:.4}, sum={:.4}",
     query_text, embedding.len(),
     embedding.iter().sum::<f32>() / embedding.len() as f32,
     embedding.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
     embedding.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)),
     embedding.iter().sum::<f32>());
     
// To this:
info!("Successfully encoded query '{}'", query_text);
debug!("Embedding stats: length={}, avg={:.4}, min={:.4}, max={:.4}, sum={:.4}",
     embedding.len(),
     embedding.iter().sum::<f32>() / embedding.len() as f32,
     embedding.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
     embedding.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)),
     embedding.iter().sum::<f32>());
```

```rust
// Change this:
info!("Performing vector search with embedding for '{}', semantic_weight={}",
    query_text, strategy.semantic_weight);
    
// To this:
debug!("Performing vector search with embedding for '{}', semantic_weight={}",
    query_text, strategy.semantic_weight);
```

```rust
// Change this:
info!("Applying hybrid search weights: semantic_weight={}", strategy.semantic_weight);

// To this:
debug!("Applying hybrid search weights: semantic_weight={}", strategy.semantic_weight);
```

### 2. In `rank_results` method:

```rust
// Change this:
info!("Book scoring: {:?} - Position: {}/{} (score: {:.2}), Rating: {:.2} (scaled: {:.2}), Final score: {:.2}",
    book.title, idx + 1, total_results, position_score, book.rating, rating_score, final_score);
    
// To this:
if tracing::enabled!(tracing::Level::DEBUG) {
    debug!("Book scoring: {:?} - Position: {}/{} (score: {:.2}), Rating: {:.2} (scaled: {:.2}), Final score: {:.2}",
        book.title, idx + 1, total_results, position_score, book.rating, rating_score, final_score);
}
```

```rust
// Change this:
info!("After custom scoring, top 5 results: {:?}",
    results
        .iter()
        .take(5)
        .map(|b| b.title.clone())
        .collect::<Vec<_>>());
        
// To this:
info!("Completed scoring of {} books", results.len());
if tracing::enabled!(tracing::Level::DEBUG) {
    debug!("After custom scoring, top 5 results: {:?}",
        results
            .iter()
            .take(5)
            .map(|b| b.title.clone())
            .collect::<Vec<_>>());
}
```

```rust
// Change this:
info!("FINAL RANKING: Top 5 results after ranking and deduplication: {:?}",
    final_results
        .iter()
        .take(5)
        .map(|b| b.title.clone())
        .collect::<Vec<_>>());
        
// To this:
info!("FINAL RANKING: Top {} results ready for response. First book: {:?}",
    final_results.len(),
    final_results.first().map(|b| b.title.clone()));
```

### 3. In `get_recommendations` method:

```rust
// Change this:
info!("Vector search returned {} results, first book: {:?}",
    results.len(),
    results.first().and_then(|b| b.title.clone()));
    
// To this:
debug!("Vector search returned {} results, first book: {:?}",
    results.len(),
    results.first().map(|r| r.title.clone()));
```

```rust
// Change this:
info!("PRE-RANKING: Top 5 raw results: {:?}",
    raw_results
        .iter()
        .take(5)
        .map(|b| b.title.clone())
        .collect::<Vec<_>>());

// To this:
debug!("PRE-RANKING: Top 5 raw results: {:?}",
    results
        .iter()
        .take(5)
        .map(|b| b.title.clone())
        .collect::<Vec<_>>());
```

```rust
// Change this:
for (i, book) in raw_results.iter().take(5).enumerate() {
    info!("PRE-RANKING #{}: Title: {:?}, Rating: {:.2}",
        i + 1,
        book.title,
        book.rating);
}

// To this:
if tracing::enabled!(tracing::Level::DEBUG) {
    for (i, book) in results.iter().take(5).enumerate() {
        debug!("PRE-RANKING #{}: Title: {:?}, Rating: {:.2}",
            i + 1,
            book.title,
            book.rating);
    }
}
```

## Benefits

These changes will:

1. Significantly reduce log volume by only logging individual book scoring at DEBUG level
2. Keep important summary information at INFO level
3. Only perform detailed logging when DEBUG level is enabled
4. Make the logs much more readable for normal operation
5. Still allow for detailed debugging when needed by setting log level to DEBUG