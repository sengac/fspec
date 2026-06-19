//! NAPI bindings for model functions
//!
//! Exposes model listing, info, and refresh to TypeScript via NAPI-RS.
//! Uses the shared REGISTRY_CACHE from the parent module.

use super::{get_registry, invalidate_registry_cache, is_current_model};
use codelet_providers::models::ModelCache;
use napi::bindgen_prelude::*;

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
    /// RPC-338: `Some(profile)` marks a local-server profile section
    /// (mirrors `ProviderInfo.profile_name`). `None` for cloud / custom.
    pub profile_name: Option<String>,
    /// RPC-338: `true` when the local-server profile is unreachable
    /// (mirrors `ProviderInfo.is_unreachable`).
    pub is_unreachable: bool,
}

// ============================================================================
// Cache Directory (Read-Only - derived from global data directory)
// ============================================================================

/// Get the current cache directory for model data
///
/// Returns {data_dir}/cache where data_dir is set via persistenceSetDataDirectory().
#[napi]
pub fn models_get_cache_directory() -> Result<String> {
    codelet_providers::models::get_cache_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| Error::from_reason(format!("Failed to get cache directory: {}", e)))
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
    let registry = get_registry().await.map_err(Error::from_reason)?;

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
                profile_name: None,
                is_unreachable: false,
            }
        })
        .collect())
}

/// List models for a specific provider (async)
///
/// Applies the same is_current_model() filter and newest-first sort as
/// models_list_all() so deprecated and stale models are excluded regardless
/// of which listing API is called (MODEL-003 Rules [0] and [1]).
///
/// # Arguments
/// * `provider_id` - Provider ID (e.g., "anthropic", "openai", "google")
#[napi]
pub async fn models_list_for_provider(provider_id: String) -> Result<Vec<NapiModelInfo>> {
    let registry = get_registry().await.map_err(Error::from_reason)?;

    let models = registry.list_models(&provider_id).map_err(|e| {
        Error::from_reason(format!(
            "Failed to list models for provider '{}': {}",
            provider_id, e
        ))
    })?;

    // Apply the same current-model filter and newest-first sort as models_list_all()
    let mut filtered: Vec<_> = models.into_iter().filter(|m| is_current_model(m)).collect();

    // Sort by release date descending (newest first)
    filtered.sort_by(|a, b| {
        let date_a = a.release_date.as_deref().unwrap_or("1970-01-01");
        let date_b = b.release_date.as_deref().unwrap_or("1970-01-01");
        date_b.cmp(date_a)
    });

    Ok(filtered.iter().map(|m| to_napi_model_info(m)).collect())
}

/// Get information for a specific model (async)
///
/// # Arguments
/// * `provider_id` - Provider ID (e.g., "anthropic")
/// * `model_id` - Model ID (e.g., "claude-sonnet-4")
#[napi]
pub async fn models_get_info(provider_id: String, model_id: String) -> Result<NapiModelInfo> {
    let registry = get_registry().await.map_err(Error::from_reason)?;

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
/// Also invalidates the in-memory registry cache so subsequent
/// calls to models_list_all() will pick up the new data.
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

    let provider_count = response.providers.len() as u32;

    // Persist refreshed data to disk and invalidate the in-memory registry
    // so get_registry() rebuilds from the fresh disk cache.
    invalidate_registry_cache().await;

    Ok(provider_count)
}

// ============================================================================
// Local Model Listing (PROV-006, PROV-040)
// ============================================================================

/// List models from a local OpenAI-compatible server (async)
///
/// PROV-006: Makes HTTP GET request to {base_url}/v1/models endpoint.
/// Used by TUI when OPENAI_BASE_URL is set.
///
/// PROV-040: Added optional api_key parameter for servers requiring authentication
/// (e.g., Fireworks AI, Together AI). Local servers (vLLM, Ollama) typically don't
/// require auth for the /models endpoint.
///
/// # Arguments
/// * `base_url` - The base URL of the local server (e.g., "http://localhost:8888")
/// * `api_key` - Optional API key for servers requiring authentication
///
/// # Returns
/// Array of model ID strings
///
/// # Example
/// ```typescript
/// // Local server (no auth required)
/// const models = await modelsListLocalOpenai("http://localhost:8888");
/// // Returns: ["Qwen/Qwen3-80B", "mistral-7b"]
///
/// // Fireworks AI (auth required)
/// const models = await modelsListLocalOpenai("https://api.fireworks.ai/inference", "fw_...");
/// // Returns: ["accounts/fireworks/models/llama-v3p1-8b-instruct", ...]
/// ```
#[napi]
pub async fn models_list_local_openai(
    base_url: String,
    api_key: Option<String>,
) -> Result<Vec<String>> {
    use codelet_providers::OpenAIProvider;

    OpenAIProvider::list_local_models_with_auth(&base_url, api_key.as_deref())
        .await
        .map_err(|e| Error::from_reason(format!("Failed to list local models: {}", e)))
}

// ============================================================================
// RPC-338: NapiProviderModels profile/unreachable surface-sync fields.
// Feature: spec/features/model-selector-profile-sections.feature
// ============================================================================
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod rpc338_tests {
    use super::NapiProviderModels;

    /// Scenario: The napi binding mirrors the new provider fields
    #[test]
    fn napi_provider_models_mirrors_profile_and_reachability_fields() {
        // @step Given the NapiProviderModels napi object binding
        let binding = NapiProviderModels {
            provider_id: "openai".to_string(),
            provider_name: "openai: my-profile".to_string(),
            models: Vec::new(),
            profile_name: Some("my-profile".to_string()),
            is_unreachable: false,
        };

        // @step Then it exposes a profile_name field of type Option<String>
        let profile_name: Option<String> = binding.profile_name.clone();
        assert_eq!(profile_name.as_deref(), Some("my-profile"));

        // @step And it exposes an is_unreachable field of type bool
        let is_unreachable: bool = binding.is_unreachable;
        assert!(!is_unreachable);
    }
}
