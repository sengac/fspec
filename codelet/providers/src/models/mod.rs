//! Model selection and registry for dynamic model selection
//!
//! This module provides:
//! - `ModelCache` - Fetches and caches models.dev API data
//! - `ModelRegistry` - Model lookup, filtering, and validation
//! - Type definitions for models.dev data structures
//!
//! Cache directory is derived from the global data directory:
//! {data_dir}/cache/models.json

mod cache;
mod registry;
mod types;

pub use cache::{get_cache_dir, ModelCache};
pub use registry::ModelRegistry;
pub use types::{
    Capability, CostInfo, InterleavedConfig, LimitInfo, Modalities, Modality, ModelInfo,
    ModelStatus, ModelsDevResponse, ProviderInfo,
};
