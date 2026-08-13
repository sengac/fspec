//! Context Management bounded context
//!
//! Token tracking, compaction, prompt caching.
//!
//! Re-exports core types from the agent::compaction module to maintain
//! the bounded context architecture while avoiding code duplication.

pub use codelet_core::compaction::TokenTracker;
