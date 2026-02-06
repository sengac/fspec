//! NAPI bindings for model cache and selection functions
//!
//! MODEL-001: Exposes models.dev model listing to TypeScript.
//!
//! This enables fspec to:
//! - List available models from models.dev
//! - Get model information for display
//!
//! NOTE: Cache directory is derived from the global data directory
//! set via persistenceSetDataDirectory(). No separate cache directory
//! configuration is needed.

use chrono::Datelike;
use codelet_providers::models::{get_cache_dir, ModelCache, ModelRegistry};
use napi::bindgen_prelude::*;
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
            let cache = ModelCache::new()
                .map_err(|e| Error::from_reason(format!("Failed to initialize model cache: {}", e)))?;
            let registry = ModelRegistry::new(&cache)
                .await
                .map_err(|e| Error::from_reason(format!("Failed to load model registry: {}", e)))?;
            Ok(Arc::new(registry))
        })
        .await
        .cloned()
}

// ============================================================================
// Cache Directory (Read-Only - derived from global data directory)
// ============================================================================

/// Get the current cache directory for model data
///
/// Returns {data_dir}/cache where data_dir is set via persistenceSetDataDirectory().
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

/// Check if a model should be shown in the UI (filters out deprecated/old models)
fn is_current_model(model: &codelet_providers::models::ModelInfo) -> bool {
    use codelet_providers::models::ModelStatus;

    // Filter out deprecated models
    if model.status == Some(ModelStatus::Deprecated) {
        return false;
    }

    // Filter out invalid aliases (models without dated versions)
    // For Anthropic models, only show dated versions like "claude-opus-4-5-20251101"
    // NOT aliases like "claude-opus-4-5" which are not valid API model IDs
    // EXCEPTION: If the model has a release_date, it's a real model (e.g., claude-opus-4-6)
    if model.id.starts_with("claude-") {
        // Check if it ends with a date pattern (8 digits: YYYYMMDD)
        let has_date_suffix = model.id.chars().rev().take(8).all(|c| c.is_ascii_digit())
            && model.id.chars().rev().nth(8) == Some('-');

        // Only filter out if no date suffix AND no release_date
        // Models with release_date are real models, not just aliases
        if !has_date_suffix && model.release_date.is_none() {
            // This is an alias like "claude-opus-4-5" or "claude-sonnet-4-5"
            // Skip it because it's not a valid Anthropic API model ID
            return false;
        }
    }

    // Filter out models older than 18 months
    if let Some(ref release_date) = model.release_date {
        // Parse release date (format: "YYYY-MM-DD")
        if let Ok(date) = chrono::NaiveDate::parse_from_str(release_date, "%Y-%m-%d") {
            let today = chrono::Utc::now().date_naive();
            let age_months =
                (today.year() - date.year()) * 12 + (today.month() as i32 - date.month() as i32);
            if age_months > 18 {
                return false;
            }
        }
    }

    true
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
///
/// Filters out:
/// - Deprecated models (status = "deprecated")
/// - Models older than 18 months
///
/// Sorts models by release date (newest first).
#[napi]
pub async fn models_list_all() -> Result<Vec<NapiProviderModels>> {
    let registry = get_registry().await?;

    Ok(registry
        .list_providers()
        .iter()
        .map(|provider_info| {
            // Filter to current models and sort by release date (newest first)
            let mut models: Vec<_> = provider_info
                .models
                .values()
                .filter(|m| is_current_model(m))
                .collect();

            // Sort by release date descending (newest first)
            models.sort_by(|a, b| {
                let date_a = a.release_date.as_deref().unwrap_or("1970-01-01");
                let date_b = b.release_date.as_deref().unwrap_or("1970-01-01");
                date_b.cmp(date_a)
            });

            NapiProviderModels {
                provider_id: provider_info.id.clone(),
                provider_name: provider_info.name.clone(),
                models: models.into_iter().map(to_napi_model_info).collect(),
            }
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
    let cache = ModelCache::new()
        .map_err(|e| Error::from_reason(format!("Failed to initialize model cache: {}", e)))?;
    let response = cache.refresh().await.map_err(|e| {
        Error::from_reason(format!(
            "Failed to refresh model cache from models.dev: {}",
            e
        ))
    })?;

    Ok(response.providers.len() as u32)
}
