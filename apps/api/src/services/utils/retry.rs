//! Retry mechanism for handling transient failures in network operations.
//!
//! This module provides utilities to retry operations with configurable
//! backoff strategies and conditions for retrying.

use std::{
    future::Future,
    time::{Duration, Instant},
};

use log::{debug, warn};
use rand::Rng;
use thiserror::Error;

/// Error type for retry operations
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum RetryError<E> {
    /// The maximum number of retry attempts was reached
    #[error("Max retry attempts ({attempts}) reached: {source}")]
    MaxAttemptsReached {
        /// Number of attempts that were made
        attempts: u32,
        /// The source error that caused the retry to fail
        source: E,
    },

    /// The operation was attempted but failed on every retry
    #[error("Operation failed after {attempts} attempts: {source}")]
    OperationFailed {
        /// Number of attempts that were made
        attempts: u32,
        /// The source error that caused the operation to fail
        source: E,
    },
}

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial delay between retries in milliseconds
    pub initial_delay_ms: u64,

    /// Factor by which to increase delay on each retry (for exponential backoff)
    pub backoff_factor: f64,

    /// Maximum delay in milliseconds, regardless of calculated backoff
    pub max_delay_ms: u64,

    /// Whether to add jitter to retry delays to avoid thundering herd problems
    pub jitter: bool,

    /// Maximum total duration to spend on retries
    pub timeout: Option<Duration>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            backoff_factor: 2.0,
            max_delay_ms: 10000, // 10 seconds max delay
            jitter: true,
            timeout: Some(Duration::from_secs(60)), // 1 minute total timeout
        }
    }
}

#[allow(dead_code)]
impl RetryConfig {
    /// Create a new retry configuration with custom values
    pub fn new(
        max_attempts: u32,
        initial_delay_ms: u64,
        backoff_factor: f64,
        max_delay_ms: u64,
        jitter: bool,
        timeout_secs: Option<u64>,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            backoff_factor,
            max_delay_ms,
            jitter,
            timeout: timeout_secs.map(Duration::from_secs),
        }
    }

    /// Calculate the delay for a given retry attempt
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay =
            (self.initial_delay_ms as f64 * self.backoff_factor.powf(attempt as f64)) as u64;
        let capped_delay = base_delay.min(self.max_delay_ms);

        if self.jitter {
            // Add random jitter between 0-25% of the delay
            let jitter_factor = rand::thread_rng().gen_range(0.75..=1.0);
            Duration::from_millis((capped_delay as f64 * jitter_factor) as u64)
        } else {
            Duration::from_millis(capped_delay)
        }
    }
}

/// Retry a future according to the provided configuration
#[allow(dead_code)]
pub async fn retry<F, Fut, T, E>(
    operation: F,
    config: &RetryConfig,
    is_retryable: impl Fn(&E) -> bool,
) -> Result<T, RetryError<E>>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let start_time = Instant::now();
    let mut attempts_made = 0;
    let mut last_error = None;

    loop {
        // If this isn't the first attempt, check limits before proceeding
        if attempts_made > 0 {
            // Check if we've exceeded the timeout
            if let Some(timeout) = config.timeout {
                if start_time.elapsed() >= timeout {
                    if let Some(err) = last_error {
                        return Err(RetryError::OperationFailed {
                            attempts: attempts_made,
                            source: err,
                        });
                    }
                }
            }

            // Check if we've exceeded max attempts
            if attempts_made >= config.max_attempts {
                if let Some(err) = last_error {
                    return Err(RetryError::MaxAttemptsReached {
                        attempts: attempts_made,
                        source: err,
                    });
                }
            }
        }

        // Track the current attempt number (1-indexed for user-facing messages)
        let _current_attempt = attempts_made + 1;

        match operation().await {
            Ok(value) => {
                if attempts_made > 0 {
                    debug!("Operation succeeded after {} retries", attempts_made);
                }
                return Ok(value);
            }
            Err(err) => {
                attempts_made += 1; // Increment attempt counter first

                if attempts_made >= config.max_attempts || !is_retryable(&err) {
                    warn!(
                        "Operation failed and will not be retried: {:?} (attempt {}/{})",
                        err, attempts_made, config.max_attempts
                    );
                    return Err(RetryError::OperationFailed {
                        attempts: attempts_made,
                        source: err,
                    });
                }

                debug!(
                    "Attempt {}/{} failed: {:?}. Retrying...",
                    attempts_made, config.max_attempts, err
                );

                // Store the error in case we need it later
                last_error = Some(err);

                // Calculate and wait for the backoff delay
                let delay = config.calculate_delay(attempts_made - 1);
                tokio::time::sleep(delay).await;
            }
        }
    }
}

/// A convenient trait to make types retryable
#[allow(dead_code)]
pub trait Retryable<T, E> {
    /// Retry this operation with the given configuration and retry predicate
    fn retry_with(
        self,
        config: &RetryConfig,
        is_retryable: impl Fn(&E) -> bool,
    ) -> impl Future<Output = Result<T, RetryError<E>>>;
}

impl<F, Fut, T, E> Retryable<T, E> for F
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    async fn retry_with(
        self,
        config: &RetryConfig,
        is_retryable: impl Fn(&E) -> bool,
    ) -> Result<T, RetryError<E>> {
        retry(self, config, is_retryable).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_retry_succeeds_eventually() {
        let attempts = AtomicU32::new(0);

        let operation = || async {
            let current = attempts.fetch_add(1, Ordering::SeqCst);
            if current < 2 {
                Err(format!("Failed attempt {}", current))
            } else {
                Ok(42)
            }
        };

        let config = RetryConfig {
            max_attempts: 5,
            initial_delay_ms: 1, // Fast for testing
            ..Default::default()
        };

        let result = retry(operation, &config, |_| true).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3); // First success is on the 3rd attempt (index 2)
    }

    #[tokio::test]
    async fn test_retry_respects_max_attempts() {
        let attempts = AtomicU32::new(0);

        let operation = || async {
            let current = attempts.fetch_add(1, Ordering::SeqCst);
            Err(format!("Always fails {}", current))
        };

        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1, // Fast for testing
            ..Default::default()
        };

        let result = retry(operation, &config, |_| true).await;
        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_respects_retryable_predicate() {
        let attempts = AtomicU32::new(0);

        let operation = || async {
            let current = attempts.fetch_add(1, Ordering::SeqCst);
            Err(format!("Error {}", current))
        };

        let config = RetryConfig {
            max_attempts: 10,
            initial_delay_ms: 1,
            ..Default::default()
        };

        // Only retry if the error message contains "0"
        let result = retry(operation, &config, |err| err.contains("0")).await;

        assert!(result.is_err());
        // Should stop after the second attempt (error 1) which is not retryable
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_backoff_calculation() {
        let config = RetryConfig {
            initial_delay_ms: 100,
            backoff_factor: 2.0,
            max_delay_ms: 1000,
            jitter: false,
            ..Default::default()
        };

        assert_eq!(config.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(config.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(config.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(config.calculate_delay(3), Duration::from_millis(800));
        // Should be capped at max_delay_ms
        assert_eq!(config.calculate_delay(4), Duration::from_millis(1000));
    }
}
