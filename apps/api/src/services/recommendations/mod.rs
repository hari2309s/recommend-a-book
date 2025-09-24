//! Recommendation services and related functionality.
//!
//! This module contains components for providing book recommendations,
//! including query intent detection, search strategies, result ranking,
//! and caching.

// Public re-exports for the module
pub mod cache;
pub mod intent;
pub mod ranking;
pub mod search;
pub mod service;

// Re-export the main service for convenience
pub use service::RecommendationService;
