//! ModelCache - Fetches and caches models.dev API data
//!
//! Cache strategy: Indefinite cache, only refetch when:
//! - Cache file is missing
//! - Cache file is corrupted (invalid JSON)
//! - User explicitly requests refresh
//!
//! Cache location is derived from the global data directory:
//! {data_dir}/cache/models.json

use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, info};

use super::types::ModelsDevResponse;
use crate::error::ProviderError;

/// URL for models.dev API
const MODELS_DEV_URL: &str = "https://models.dev/api.json";

/// Get the cache directory for model data
///
/// Derives from the global data directory: {data_dir}/cache
pub fn get_cache_dir() -> Result<PathBuf, String> {
    codelet_common::get_data_dir().map(|data_dir| data_dir.join("cache"))
}

/// Cache for models.dev API data
pub struct ModelCache {
    cache_path: PathBuf,
}

impl ModelCache {
    /// Create a new ModelCache with default cache path
    ///
    /// Uses get_cache_dir() which derives from the global data directory
    pub fn new() -> Self {
        let cache_dir = get_cache_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            cache_path: cache_dir.join("models.json"),
        }
    }

    /// Create a new ModelCache with a custom cache path (for testing)
    pub fn new_with_path(cache_path: PathBuf) -> Self {
        Self { cache_path }
    }

    /// Get models data. Uses cache if valid, fetches only if needed.
    ///
    /// Strategy:
    /// 1. Try to read and parse cache file
    /// 2. If cache exists and parses, return it (indefinite cache)
    /// 3. If cache missing or corrupted, fetch from API (must succeed or error)
    pub async fn get(&self) -> Result<ModelsDevResponse, ProviderError> {
        // Try to read and parse cache first
        match self.read_cache().await {
            Ok(data) => {
                debug!("Loaded models from cache: {}", self.cache_path.display());
                Ok(data)
            }
            Err(e) => {
                info!("Cache miss or invalid ({}), fetching from API", e);
                self.fetch_from_api().await
            }
        }
    }

    /// Force refresh from API (user-initiated via --refresh flag)
    pub async fn refresh(&self) -> Result<ModelsDevResponse, ProviderError> {
        info!("Force refreshing models cache");
        self.fetch_and_cache().await
    }

    /// Fetch from API - must succeed or error
    async fn fetch_from_api(&self) -> Result<ModelsDevResponse, ProviderError> {
        self.fetch_and_cache().await.map_err(|e| {
            ProviderError::api(
                "models.dev",
                format!("Failed to fetch models (cache miss/invalid, API unreachable): {e}"),
            )
        })
    }

    /// Fetch from models.dev API and cache the result
    async fn fetch_and_cache(&self) -> Result<ModelsDevResponse, ProviderError> {
        let client = reqwest::Client::new();
        let response = client
            .get(MODELS_DEV_URL)
            .header("User-Agent", "codelet/0.1")
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| ProviderError::api("models.dev", format!("Network error: {e}")))?;

        if !response.status().is_success() {
            return Err(ProviderError::api(
                "models.dev",
                format!("HTTP error: {}", response.status()),
            ));
        }

        let data = response.text().await.map_err(|e| {
            ProviderError::api("models.dev", format!("Failed to read response: {e}"))
        })?;

        // Validate JSON before saving
        let parsed: ModelsDevResponse = serde_json::from_str(&data)
            .map_err(|e| ProviderError::api("models.dev", format!("Invalid JSON: {e}")))?;

        // Ensure cache directory exists
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                ProviderError::api("models.dev", format!("Failed to create cache dir: {e}"))
            })?;
        }

        // Write cache
        fs::write(&self.cache_path, &data)
            .await
            .map_err(|e| ProviderError::api("models.dev", format!("Failed to write cache: {e}")))?;

        info!("Models cache updated: {}", self.cache_path.display());
        Ok(parsed)
    }

    /// Read and parse cache file
    async fn read_cache(&self) -> Result<ModelsDevResponse, CacheError> {
        let data = fs::read_to_string(&self.cache_path)
            .await
            .map_err(|e| CacheError::NotFound(e.to_string()))?;

        serde_json::from_str(&data).map_err(|e| CacheError::ParseError(e.to_string()))
    }

    /// Get the cache file path
    pub fn cache_path(&self) -> &PathBuf {
        &self.cache_path
    }
}

impl Default for ModelCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal cache errors
#[derive(Debug)]
enum CacheError {
    NotFound(String),
    ParseError(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::NotFound(msg) => write!(f, "cache not found: {msg}"),
            CacheError::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_cache_path() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cache_path = temp_dir.path().join("models.json");
        (temp_dir, cache_path)
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let (_temp_dir, cache_path) = test_cache_path();

        // Write valid cache
        let cache_content = r#"{
            "anthropic": {
                "id": "anthropic",
                "name": "Anthropic",
                "env": ["ANTHROPIC_API_KEY"],
                "models": {
                    "claude-sonnet-4": {
                        "id": "claude-sonnet-4-20250514",
                        "name": "Claude Sonnet 4",
                        "reasoning": true,
                        "tool_call": true,
                        "limit": {"context": 200000, "output": 16000}
                    }
                }
            }
        }"#;
        std::fs::write(&cache_path, cache_content).expect("Failed to write cache");

        let cache = ModelCache::new_with_path(cache_path);
        let result = cache.get().await;

        assert!(result.is_ok(), "Should load from cache");
        let models = result.unwrap();
        assert!(models.providers.contains_key("anthropic"));
    }

    #[test]
    fn test_new_with_path() {
        let path = PathBuf::from("/tmp/test/models.json");
        let cache = ModelCache::new_with_path(path.clone());
        assert_eq!(cache.cache_path(), &path);
    }
}
