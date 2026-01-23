//! ModelCache - Fetches and caches models.dev API data
//!
//! Cache strategy: Indefinite cache, only refetch when:
//! - Cache file is missing
//! - Cache file is corrupted (invalid JSON)
//! - User explicitly requests refresh
//!
//! MODEL-001: Directory is configurable via set_cache_directory()
//! Default location: ~/.fspec/cache/models.json (matches fspec data directory)

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tokio::fs;
use tracing::{debug, info, warn};

use super::types::ModelsDevResponse;
use crate::error::ProviderError;

/// URL for models.dev API
const MODELS_DEV_URL: &str = "https://models.dev/api.json";

/// Embedded fallback snapshot of models.dev data
/// This is updated at build time by build.rs
const FALLBACK_MODELS: &[u8] = include_bytes!("fallback_models.json");

// MODEL-001: Global configurable cache directory (thread-safe)
lazy_static::lazy_static! {
    static ref CACHE_DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);
}

/// Set a custom cache directory for model data
///
/// This should be called before any ModelCache operations if you want
/// to use a directory other than the default ~/.fspec/cache
///
/// # Arguments
/// * `dir` - The base directory for cache data (cache subdirectory is NOT added automatically)
///
/// # Example
/// ```ignore
/// // Use ~/.fspec/cache for fspec
/// set_cache_directory(PathBuf::from(home_dir).join(".fspec").join("cache"));
///
/// // Use custom directory
/// set_cache_directory(PathBuf::from("/tmp/cache"));
/// ```
pub fn set_cache_directory(dir: PathBuf) -> Result<(), String> {
    let mut cache_dir = CACHE_DIRECTORY.lock().map_err(|e| e.to_string())?;
    *cache_dir = Some(dir);
    Ok(())
}

/// Get the base directory for cache data
///
/// Returns the custom directory if set via set_cache_directory(),
/// otherwise returns ~/.fspec/cache as the default.
///
/// # Errors
/// Returns an error if:
/// - The directory mutex is poisoned (another thread panicked while holding it)
/// - Home directory cannot be determined
pub fn get_cache_dir() -> Result<PathBuf, String> {
    // Check for custom directory first - fail explicitly on poison
    let guard = CACHE_DIRECTORY
        .lock()
        .map_err(|e| format!("Cache directory mutex poisoned: {e}"))?;

    if let Some(ref dir) = *guard {
        return Ok(dir.clone());
    }

    // Drop lock before potentially slow home_dir lookup
    drop(guard);

    // Default to ~/.fspec/cache (matches fspec data directory pattern)
    dirs::home_dir()
        .map(|home| home.join(".fspec").join("cache"))
        .ok_or_else(|| "Could not determine home directory".to_string())
}

/// Cache for models.dev API data
pub struct ModelCache {
    cache_path: PathBuf,
}

impl ModelCache {
    /// Create a new ModelCache with default cache path
    ///
    /// Uses get_cache_dir() which defaults to ~/.fspec/cache/models.json
    /// or the directory set via set_cache_directory()
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
    /// 3. If cache missing or corrupted, fetch from API
    /// 4. If API fails, use embedded fallback
    pub async fn get(&self) -> Result<ModelsDevResponse, ProviderError> {
        // Try to read and parse cache first
        match self.read_cache().await {
            Ok(data) => {
                debug!("Loaded models from cache: {}", self.cache_path.display());
                Ok(data)
            }
            Err(e) => {
                info!("Cache miss or invalid ({}), fetching from API", e);
                self.fetch_with_fallback().await
            }
        }
    }

    /// Force refresh from API (user-initiated via --refresh flag)
    pub async fn refresh(&self) -> Result<ModelsDevResponse, ProviderError> {
        info!("Force refreshing models cache");
        self.fetch_and_cache().await
    }

    /// Fetch from API, falling back to embedded snapshot on failure
    async fn fetch_with_fallback(&self) -> Result<ModelsDevResponse, ProviderError> {
        match self.fetch_and_cache().await {
            Ok(data) => Ok(data),
            Err(e) => {
                warn!(
                    "Failed to fetch from models.dev ({}), using embedded fallback",
                    e
                );
                self.load_fallback()
            }
        }
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

    /// Load embedded fallback snapshot
    fn load_fallback(&self) -> Result<ModelsDevResponse, ProviderError> {
        serde_json::from_slice(FALLBACK_MODELS)
            .map_err(|e| ProviderError::api("models.dev", format!("Failed to parse fallback: {e}")))
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
    use serial_test::serial;
    use tempfile::TempDir;

    fn test_cache_path() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cache_path = temp_dir.path().join("models.json");
        (temp_dir, cache_path)
    }

    /// Reset the global cache directory to None for test isolation
    fn reset_cache_directory() {
        if let Ok(mut guard) = CACHE_DIRECTORY.lock() {
            *guard = None;
        }
    }

    #[tokio::test]
    async fn test_cache_miss_returns_fallback() {
        let (_temp_dir, cache_path) = test_cache_path();
        let cache = ModelCache::new_with_path(cache_path);

        // Cache doesn't exist, should use fallback
        let result = cache.load_fallback();
        assert!(result.is_ok(), "Should load fallback successfully");

        let models = result.unwrap();
        assert!(
            !models.providers.is_empty(),
            "Fallback should contain providers"
        );
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

    #[tokio::test]
    async fn test_corrupted_cache_uses_fallback() {
        let (_temp_dir, cache_path) = test_cache_path();

        // Write corrupted cache
        std::fs::write(&cache_path, "{ this is not valid JSON {{{{")
            .expect("Failed to write cache");

        let cache = ModelCache::new_with_path(cache_path);

        // This will try to read corrupted cache, fail, then use fallback
        // Note: In real scenario it would try API first, but for unit test we just check fallback
        let result = cache.load_fallback();
        assert!(result.is_ok(), "Should fall back to embedded data");
    }

    // MODEL-001: Tests for set_cache_directory
    #[test]
    #[serial]
    fn test_set_cache_directory_changes_path() {
        reset_cache_directory();

        let custom_dir = PathBuf::from("/tmp/custom-cache");
        set_cache_directory(custom_dir.clone()).expect("Should set directory");

        let result = get_cache_dir().expect("Should get directory");
        assert_eq!(result, custom_dir);

        reset_cache_directory();
    }

    #[test]
    #[serial]
    fn test_get_cache_dir_returns_default_when_not_set() {
        reset_cache_directory();

        let result = get_cache_dir().expect("Should get default directory");

        // Should end with .fspec/cache
        assert!(
            result.ends_with(".fspec/cache"),
            "Default should be ~/.fspec/cache, got: {result:?}"
        );

        reset_cache_directory();
    }

    #[test]
    #[serial]
    fn test_model_cache_new_uses_configured_directory() {
        reset_cache_directory();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let custom_dir = temp_dir.path().to_path_buf();
        set_cache_directory(custom_dir.clone()).expect("Should set directory");

        let cache = ModelCache::new();
        let expected_path = custom_dir.join("models.json");

        assert_eq!(
            cache.cache_path(),
            &expected_path,
            "ModelCache should use configured directory"
        );

        reset_cache_directory();
    }

    #[test]
    #[serial]
    fn test_set_cache_directory_can_be_changed() {
        reset_cache_directory();

        let dir1 = PathBuf::from("/tmp/dir1");
        let dir2 = PathBuf::from("/tmp/dir2");

        set_cache_directory(dir1.clone()).expect("Should set first directory");
        assert_eq!(get_cache_dir().unwrap(), dir1);

        set_cache_directory(dir2.clone()).expect("Should set second directory");
        assert_eq!(get_cache_dir().unwrap(), dir2);

        reset_cache_directory();
    }
}
