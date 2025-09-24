//! Generic caching utilities for the application.
//!
//! This module provides thread-safe caching mechanisms with configurable TTL (time-to-live)
//! for various components of the application.

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

/// Cache entry that stores a value and its insertion timestamp
#[derive(Debug, Clone)]
pub struct CacheEntry<V: Clone> {
    /// The cached value
    pub value: V,
    /// When the entry was inserted or last updated
    pub timestamp: Instant,
}

/// Generic thread-safe cache with TTL support
#[derive(Debug)]
pub struct Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Internal storage with read-write lock for thread safety
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    /// TTL in seconds for cache entries
    ttl: u64,
}

#[allow(dead_code)]
impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Create a new cache with the specified TTL in seconds
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl: ttl_seconds,
        }
    }

    /// Get a value from the cache if it exists and is not expired
    pub fn get(&self, key: &K) -> Option<V> {
        if let Ok(cache) = self.entries.read() {
            if let Some(entry) = cache.get(key) {
                if entry.timestamp.elapsed() < Duration::from_secs(self.ttl) {
                    return Some(entry.value.clone());
                }
            }
        }
        None
    }

    /// Set a value in the cache
    pub fn set(&self, key: K, value: V) -> Result<(), String> {
        match self.entries.write() {
            Ok(mut cache) => {
                cache.insert(
                    key,
                    CacheEntry {
                        value,
                        timestamp: Instant::now(),
                    },
                );
                Ok(())
            }
            Err(e) => Err(format!("Failed to acquire write lock: {}", e)),
        }
    }

    /// Remove a specific entry from the cache
    pub fn invalidate(&self, key: &K) -> Result<(), String> {
        match self.entries.write() {
            Ok(mut cache) => {
                cache.remove(key);
                Ok(())
            }
            Err(e) => Err(format!("Failed to acquire write lock: {}", e)),
        }
    }

    /// Remove all expired entries from the cache
    pub fn cleanup(&self) -> Result<usize, String> {
        match self.entries.write() {
            Ok(mut cache) => {
                let before_count = cache.len();
                cache.retain(|_, entry| entry.timestamp.elapsed() < Duration::from_secs(self.ttl));
                Ok(before_count - cache.len())
            }
            Err(e) => Err(format!("Failed to acquire write lock: {}", e)),
        }
    }

    /// Get all keys currently in the cache (including expired ones)
    pub fn keys(&self) -> Result<Vec<K>, String> {
        match self.entries.read() {
            Ok(cache) => Ok(cache.keys().cloned().collect()),
            Err(e) => Err(format!("Failed to acquire read lock: {}", e)),
        }
    }

    /// Get the number of entries in the cache (including expired ones)
    pub fn len(&self) -> Result<usize, String> {
        match self.entries.read() {
            Ok(cache) => Ok(cache.len()),
            Err(e) => Err(format!("Failed to acquire read lock: {}", e)),
        }
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> Result<bool, String> {
        self.len().map(|len| len == 0)
    }

    /// Clear all entries from the cache
    pub fn clear(&self) -> Result<(), String> {
        match self.entries.write() {
            Ok(mut cache) => {
                cache.clear();
                Ok(())
            }
            Err(e) => Err(format!("Failed to acquire write lock: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_cache_basic_operations() {
        let cache = Cache::<String, i32>::new(10); // 10 second TTL

        // Set and get
        cache.set("key1".to_string(), 42).unwrap();
        assert_eq!(cache.get(&"key1".to_string()), Some(42));

        // Key that doesn't exist
        assert_eq!(cache.get(&"non_existent".to_string()), None);

        // Invalidate
        cache.invalidate(&"key1".to_string()).unwrap();
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = Cache::<String, i32>::new(1); // 1 second TTL

        cache.set("short_lived".to_string(), 100).unwrap();
        assert_eq!(cache.get(&"short_lived".to_string()), Some(100));

        // Wait for expiration
        thread::sleep(Duration::from_secs(2));
        assert_eq!(cache.get(&"short_lived".to_string()), None);
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = Cache::<String, i32>::new(1); // 1 second TTL

        cache.set("key1".to_string(), 1).unwrap();
        cache.set("key2".to_string(), 2).unwrap();

        // Wait for expiration
        thread::sleep(Duration::from_secs(2));

        // Items should still be in the cache until cleanup
        assert_eq!(cache.len().unwrap(), 2);

        // After cleanup, they should be gone
        let removed = cache.cleanup().unwrap();
        assert_eq!(removed, 2);
        assert_eq!(cache.len().unwrap(), 0);
    }
}
