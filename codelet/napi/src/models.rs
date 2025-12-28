//! NAPI bindings for model cache and selection functions
//!
//! MODEL-001: Exposes models.dev cache configuration and model selection to TypeScript.
//!
//! This enables fspec to:
//! - Configure the cache directory to ~/.fspec/cache
//! - List available models from models.dev
//! - Get model information for display
//!
//! IMPORTANT: Call modelsSetCacheDirectory() BEFORE any other model operations
//! if you need a custom cache location. The directory is captured at first use.

use codelet_providers::models::{get_cache_dir, set_cache_directory, ModelCache, ModelRegistry};
use napi::bindgen_prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

// ============================================================================
// Cached Registry (DRY - avoid repeated JSON parsing)
// ============================================================================

/// Cached model registry - initialized once, reused across all NAPI calls
static REGISTRY_CACHE: OnceCell<Arc<ModelRegistry>> = OnceCell::const_new();

/// Get or initialize the cached model registry
async fn get_registry() -> Result<Arc<ModelRegistry>> {
    REGISTRY_CACHE
        .get_or_try_init(|| async {
            let cache = ModelCache::new();
            let registry = ModelRegistry::new(&cache)
                .await
                .map_err(|e| Error::from_reason(format!("Failed to load model registry: {}", e)))?;
            Ok(Arc::new(registry))
        })
        .await
        .cloned()
}


// ============================================================================
// Cache Directory Configuration
// ============================================================================

/// Set the cache directory for model data (e.g., ~/.fspec/cache)
///
/// IMPORTANT: This MUST be called BEFORE any other model operations.
/// The directory setting is captured when the registry is first loaded.
/// Calling this after other model functions will have no effect until
/// modelsRefreshCache() is called.
///
/// # Arguments
/// * `dir` - The directory path for cache data (models.json will be stored here)
#[napi]
pub fn models_set_cache_directory(dir: String) -> Result<()> {
    let path = PathBuf::from(&dir);
    set_cache_directory(path).map_err(|e| {
        Error::from_reason(format!("Failed to set cache directory '{}': {}", dir, e))
    })
}

/// Get the current cache directory for model data
///
/// Returns the custom directory if set via modelsSetCacheDirectory(),
/// otherwise returns ~/.fspec/cache as the default.
#[napi]
pub fn models_get_cache_directory() -> Result<String> {
    get_cache_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| Error::from_reason(format!("Failed to get cache directory: {}", e)))
}

// ============================================================================
// Model Information Types
// ============================================================================

/// Model information from models.dev
#[napi(object)]
pub struct NapiModelInfo {
    /// The API model ID (e.g., "claude-sonnet-4-20250514")
    pub id: String,
    /// Display name (e.g., "Claude Sonnet 4")
    pub name: String,
    /// Model family (e.g., "claude-sonnet")
    pub family: Option<String>,
    /// Whether model supports reasoning/thinking
    pub reasoning: bool,
    /// Whether model supports tool calls
    pub tool_call: bool,
    /// Whether model supports file/image attachments
    pub attachment: bool,
    /// Whether model supports temperature parameter
    pub temperature: bool,
    /// Context window size in tokens
    pub context_window: u32,
    /// Maximum output tokens
    pub max_output: u32,
    /// Whether model has vision capability (image input)
    pub has_vision: bool,
}

/// Helper to convert ModelInfo to NapiModelInfo (DRY - single conversion point)
fn to_napi_model_info(model: &codelet_providers::models::ModelInfo) -> NapiModelInfo {
    use codelet_providers::models::Modality;

    let has_vision = model
        .modalities
        .as_ref()
        .map(|m| m.input.contains(&Modality::Image))
        .unwrap_or(false);

    NapiModelInfo {
        id: model.id.clone(),
        name: model.name.clone(),
        family: model.family.clone(),
        reasoning: model.reasoning,
        tool_call: model.tool_call,
        attachment: model.attachment,
        temperature: model.temperature,
        context_window: model.limit.context,
        max_output: model.limit.output,
        has_vision,
    }
}

/// Provider with its available models
#[napi(object)]
pub struct NapiProviderModels {
    /// Provider ID (e.g., "anthropic", "openai", "google")
    pub provider_id: String,
    /// Provider display name (e.g., "Anthropic", "OpenAI", "Google")
    pub provider_name: String,
    /// List of models available from this provider
    pub models: Vec<NapiModelInfo>,
}

// ============================================================================
// Model Listing Functions
// ============================================================================

/// List all available models from models.dev (async)
///
/// Returns models grouped by provider. Uses cached registry for efficiency.
/// First call loads from disk/API, subsequent calls use cached data.
#[napi]
pub async fn models_list_all() -> Result<Vec<NapiProviderModels>> {
    let registry = get_registry().await?;

    Ok(registry
        .list_providers()
        .iter()
        .map(|provider_info| NapiProviderModels {
            provider_id: provider_info.id.clone(),
            provider_name: provider_info.name.clone(),
            models: provider_info
                .models
                .values()
                .map(to_napi_model_info)
                .collect(),
        })
        .collect())
}

/// List models for a specific provider (async)
///
/// # Arguments
/// * `provider_id` - Provider ID (e.g., "anthropic", "openai", "google")
#[napi]
pub async fn models_list_for_provider(provider_id: String) -> Result<Vec<NapiModelInfo>> {
    let registry = get_registry().await?;

    let models = registry.list_models(&provider_id).map_err(|e| {
        Error::from_reason(format!(
            "Failed to list models for provider '{}': {}",
            provider_id, e
        ))
    })?;

    Ok(models.iter().map(|m| to_napi_model_info(m)).collect())
}

/// Get information for a specific model (async)
///
/// # Arguments
/// * `provider_id` - Provider ID (e.g., "anthropic")
/// * `model_id` - Model ID (e.g., "claude-sonnet-4")
#[napi]
pub async fn models_get_info(provider_id: String, model_id: String) -> Result<NapiModelInfo> {
    let registry = get_registry().await?;

    let model = registry.get_model(&provider_id, &model_id).map_err(|e| {
        Error::from_reason(format!(
            "Model '{}/{}' not found: {}",
            provider_id, model_id, e
        ))
    })?;

    Ok(to_napi_model_info(model))
}

/// Refresh the model cache from models.dev API (async)
///
/// Forces a fresh fetch from the API, ignoring cached data.
/// NOTE: This does NOT invalidate the in-memory registry cache.
/// For a full refresh, restart the process after calling this.
///
/// Returns the number of providers loaded.
#[napi]
pub async fn models_refresh_cache() -> Result<u32> {
    let cache = ModelCache::new();
    let response = cache.refresh().await.map_err(|e| {
        Error::from_reason(format!(
            "Failed to refresh model cache from models.dev: {}",
            e
        ))
    })?;

    Ok(response.providers.len() as u32)
}
