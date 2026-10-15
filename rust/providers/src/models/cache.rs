//! ModelCache - Fetches and caches models.dev API data
//!
//! Cache strategy: TTL with stale-while-revalidate:
//! - Fresh cache (age < TTL) → served from disk, no network
//! - Stale cache (age >= TTL) → the snapshot is served IMMEDIATELY and a
//!   background revalidation is kicked off; the next read sees the new data
//! - Missing / corrupt cache → synchronous fetch (must succeed or error)
//! - Explicit [`ModelCache::refresh`] → synchronous fetch, bypassing the TTL
//!
//! The previous strategy was an INDEFINITE cache: the snapshot was only
//! refetched when the file was missing/corrupt or the user explicitly asked.
//! No production Rust caller could reach `refresh()` (only the retired NAPI
//! binding did), so a months-old snapshot silently froze the model selector —
//! models released after the snapshot date never appeared.
//!
//! Cache location is derived from the global data directory:
//! {data_dir}/cache/models.json

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tracing::{debug, info, warn};

use super::types::ModelsDevResponse;
use crate::error::ProviderError;

/// URL for models.dev API
const MODELS_DEV_URL: &str = "https://models.dev/api.json";

/// How long a cached models.dev snapshot is considered fresh.
///
/// models.dev only changes when a provider ships a new model, so a day is
/// plenty. Anything longer silently freezes the model selector: a snapshot
/// older than a model release means that model never appears.
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// Minimum gap between revalidation attempts.
///
/// `build_cloud_registry()` (sessions) constructs a NEW `ModelCache` on every
/// `list_providers` call, so the throttle cannot live on the instance — it is
/// process-wide, keyed on the cache file itself.
const REVALIDATE_COOLDOWN: Duration = Duration::from_secs(5 * 60);

/// Network timeout for the cold-start path (no usable cache on disk).
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// Network timeout for revalidation of an existing snapshot.
///
/// Shorter than [`FETCH_TIMEOUT`]: a stale snapshot is already on disk, so a
/// slow upstream should fail fast rather than stall the caller.
const REVALIDATE_TIMEOUT: Duration = Duration::from_secs(10);

/// Unix seconds of the last revalidation attempt (0 = never).
static LAST_REVALIDATE_ATTEMPT: AtomicU64 = AtomicU64::new(0);

/// Get the cache directory for model data
///
/// Derives from the global data directory: {data_dir}/cache
pub fn get_cache_dir() -> Result<PathBuf, String> {
    codelet_common::get_data_dir().map(|data_dir| data_dir.join("cache"))
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Claim the revalidation slot for this process, honouring
/// [`REVALIDATE_COOLDOWN`]. Returns `false` when another attempt was started
/// too recently (or won the race), so callers must not revalidate.
fn try_claim_revalidation_slot() -> bool {
    let now = now_unix_secs();
    loop {
        let last = LAST_REVALIDATE_ATTEMPT.load(Ordering::Acquire);
        let last = Duration::from_secs(last);
        if last != Duration::ZERO && now.saturating_sub(last.as_secs()) < REVALIDATE_COOLDOWN.as_secs()
        {
            return false;
        }
        match LAST_REVALIDATE_ATTEMPT.compare_exchange_weak(
            last.as_secs(),
            now,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return true,
            Err(current) => {
                if Duration::from_secs(current) != last {
                    continue;
                }
                return false;
            }
        }
    }
}

/// Reset the process-wide revalidation throttle.
///
/// Tests exercise the revalidation path repeatedly against throwaway cache
/// files; the production throttle exists to protect the network, not tests.
#[doc(hidden)]
pub fn reset_revalidation_throttle() {
    LAST_REVALIDATE_ATTEMPT.store(0, Ordering::Release);
}

/// Whether a revalidation attempt has been claimed since the last
/// [`reset_revalidation_throttle`].
///
/// The claim happens synchronously (before any task is spawned), so tests can
/// observe "a revalidation was attempted" without racing the network.
#[cfg(test)]
#[doc(hidden)]
pub fn revalidation_slot_claimed() -> bool {
    LAST_REVALIDATE_ATTEMPT.load(Ordering::Acquire) != 0
}

/// An owned fetch target so revalidation can run on a detached task without
/// borrowing `ModelCache` (which is not `'static`).
#[derive(Clone)]
struct Fetcher {
    cache_path: PathBuf,
    api_url: String,
}

impl Fetcher {
    /// Fetch from the models.dev API and write the snapshot to disk.
    async fn fetch_and_cache(&self, timeout: Duration) -> Result<ModelsDevResponse, ProviderError> {
        let client = reqwest::Client::new();
        let response = client
            .get(&self.api_url)
            .header("User-Agent", "rust/0.1")
            .timeout(timeout)
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
        fs::write(&self.cache_path, &data).await.map_err(|e| {
            ProviderError::api("models.dev", format!("Failed to write cache: {e}"))
        })?;

        info!("Models cache updated: {}", self.cache_path.display());
        Ok(parsed)
    }
}

/// Cache for models.dev API data
pub struct ModelCache {
    fetcher: Fetcher,
    ttl: Duration,
}

impl ModelCache {
    /// Create a new ModelCache with default cache path
    ///
    /// Uses get_cache_dir() which derives from the global data directory.
    /// Returns an error if the data directory has not been initialized.
    pub fn new() -> Result<Self, ProviderError> {
        let cache_dir = get_cache_dir().map_err(|e| {
            ProviderError::config(
                "models",
                format!(
                    "Data directory not initialized. Call set_data_directory() at startup. Error: {e}"
                ),
            )
        })?;

        let mut cache = Self::new_with_path(cache_dir.join("models.json"));
        // Test seam (BUG-207): redirect the catalog endpoint without touching
        // the real models.dev. Same env-override precedent as
        // FSPEC_UPDATE_BASE_URL in fspec-core's update module.
        if let Ok(url) = std::env::var("FSPEC_MODELS_DEV_URL") {
            cache = cache.with_api_url(&url);
        }
        Ok(cache)
    }

    /// Create a new ModelCache with a custom cache path (for testing)
    pub fn new_with_path(cache_path: PathBuf) -> Self {
        Self {
            fetcher: Fetcher {
                cache_path,
                api_url: MODELS_DEV_URL.to_string(),
            },
            ttl: DEFAULT_CACHE_TTL,
        }
    }

    /// Override how long a snapshot stays fresh (builder, for testing)
    #[must_use]
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Override the catalog endpoint (builder, for testing against a mock)
    #[must_use]
    pub fn with_api_url(mut self, url: &str) -> Self {
        self.fetcher.api_url = url.to_string();
        self
    }

    /// Get models data. Uses cache when fresh, revalidates when stale.
    ///
    /// Strategy:
    /// 1. Read + parse the cache file
    /// 2. Fresh → return it, no network
    /// 3. Stale → return it immediately AND revalidate in the background, so
    ///    the UI never blocks on the network and the NEXT read is fresh
    /// 4. Missing / corrupt → fetch from the API (must succeed or error)
    pub async fn get(&self) -> Result<ModelsDevResponse, ProviderError> {
        match self.read_cache().await {
            Ok(data) => {
                if self.is_fresh().await {
                    debug!("Loaded models from fresh cache: {}", self.cache_path().display());
                    return Ok(data);
                }
                // Stale-while-revalidate: serve the snapshot now, refresh soon.
                debug!(
                    "Models cache is stale ({}); serving snapshot and revalidating",
                    self.cache_path().display()
                );
                self.spawn_revalidate();
                Ok(data)
            }
            Err(e) => {
                info!("Cache miss or invalid ({e}), fetching from API");
                self.fetch_from_api().await
            }
        }
    }

    /// Force refresh from the API, bypassing the TTL and the cooldown.
    pub async fn refresh(&self) -> Result<ModelsDevResponse, ProviderError> {
        info!("Force refreshing models cache");
        reset_revalidation_throttle();
        self.fetcher.fetch_and_cache(FETCH_TIMEOUT).await
    }

    /// Revalidate now if the on-disk snapshot is stale (or unreadable).
    ///
    /// Returns `true` when a fetch was attempted. This is the synchronous
    /// counterpart of the detached task [`ModelCache::spawn_revalidate`]
    /// spawns — callers that want deterministic completion use this one.
    pub async fn revalidate_if_stale(&self) -> bool {
        if self.is_fresh().await {
            return false;
        }
        if !try_claim_revalidation_slot() {
            return false;
        }
        match self.fetcher.fetch_and_cache(REVALIDATE_TIMEOUT).await {
            Ok(_) => {
                info!("models catalog revalidated: {}", self.cache_path().display());
                true
            }
            Err(e) => {
                warn!("models.dev revalidation failed: {e}");
                true
            }
        }
    }

    /// Kick off a background revalidation; never blocks the caller.
    fn spawn_revalidate(&self) {
        if !try_claim_revalidation_slot() {
            return;
        }
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let fetcher = self.fetcher.clone();
        handle.spawn(async move {
            match fetcher.fetch_and_cache(REVALIDATE_TIMEOUT).await {
                Ok(_) => info!("models catalog revalidated in the background"),
                Err(e) => warn!("background models.dev revalidation failed: {e}"),
            }
        });
    }

    /// Whether the on-disk snapshot is still within the TTL.
    async fn is_fresh(&self) -> bool {
        match self.cache_age().await {
            Some(age) => age < self.ttl,
            // Unreadable metadata → treat as stale, so the catalog can never
            // be pinned forever by a filesystem we cannot introspect.
            None => false,
        }
    }

    /// Age of the cache file. `None` when the file/metadata is unavailable;
    /// a future mtime (clock skew) counts as age zero.
    async fn cache_age(&self) -> Option<Duration> {
        let meta = fs::metadata(self.cache_path()).await.ok()?;
        let modified = meta.modified().ok()?;
        match SystemTime::now().duration_since(modified) {
            Ok(age) => Some(age),
            Err(_) => Some(Duration::ZERO),
        }
    }

    /// Fetch from API - must succeed or error
    async fn fetch_from_api(&self) -> Result<ModelsDevResponse, ProviderError> {
        self.fetcher
            .fetch_and_cache(FETCH_TIMEOUT)
            .await
            .map_err(|e| {
                ProviderError::api(
                    "models.dev",
                    format!("Failed to fetch models (cache miss/invalid, API unreachable): {e}"),
                )
            })
    }

    /// Read and parse cache file
    async fn read_cache(&self) -> Result<ModelsDevResponse, CacheError> {
        let data = fs::read_to_string(self.cache_path())
            .await
            .map_err(|e| CacheError::NotFound(e.to_string()))?;

        serde_json::from_str(&data).map_err(|e| CacheError::ParseError(e.to_string()))
    }

    /// Get the cache file path
    pub fn cache_path(&self) -> &Path {
        &self.fetcher.cache_path
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

    const VALID_CACHE: &str = r#"{
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

    #[tokio::test]
    async fn test_cache_hit() {
        let (_temp_dir, cache_path) = test_cache_path();
        std::fs::write(&cache_path, VALID_CACHE).expect("Failed to write cache");

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

    /// Scenario: A stale models.dev snapshot is revalidated instead of served
    /// forever. This is the regression that hid GPT-6.
    #[serial]
    #[tokio::test]
    async fn stale_snapshot_is_revalidated() {
        reset_revalidation_throttle();
        let (_temp_dir, cache_path) = test_cache_path();
        std::fs::write(&cache_path, VALID_CACHE).expect("Failed to write cache");

        // @step Given the models.dev snapshot on disk is older than the cache TTL
        let cache = ModelCache::new_with_path(cache_path.clone()).with_ttl(Duration::ZERO);
        assert!(!cache.is_fresh().await, "ttl=0 makes any snapshot stale");

        // Unreachable endpoint: the attempt is still made, and the stale
        // snapshot keeps being served rather than erroring.
        let cache = cache.with_api_url("http://127.0.0.1:1/api.json");
        // @step When the model catalog is read
        // @step Then the stale snapshot is served immediately
        let models = cache.get().await.expect("stale snapshot served immediately");
        assert!(models.providers.contains_key("anthropic"));
        // @step And a revalidation of the catalog is attempted
        // The slot is claimed synchronously inside get() before the detached
        // fetch is spawned, so this is race-free.
        assert!(
            revalidation_slot_claimed(),
            "a stale snapshot must trigger a revalidation attempt"
        );
    }

    /// Scenario: A fresh models.dev snapshot is served without any network
    /// request.
    #[serial]
    #[tokio::test]
    async fn fresh_snapshot_is_not_revalidated() {
        reset_revalidation_throttle();
        let (_temp_dir, cache_path) = test_cache_path();
        std::fs::write(&cache_path, VALID_CACHE).expect("Failed to write cache");

        // @step Given the models.dev snapshot on disk is younger than the cache TTL
        let cache = ModelCache::new_with_path(cache_path)
            .with_ttl(Duration::from_secs(3600))
            .with_api_url("http://127.0.0.1:1/api.json");
        assert!(cache.is_fresh().await, "just-written snapshot is fresh");
        // @step When the model catalog is read
        // @step Then the snapshot is served from disk
        let models = cache.get().await.expect("served from disk");
        assert!(models.providers.contains_key("anthropic"));
        // @step And no revalidation of the catalog is attempted
        assert!(
            !revalidation_slot_claimed(),
            "fresh → no revalidation attempt may be claimed"
        );
    }

    /// Scenario: Revalidation attempts are throttled.
    ///
    /// The cooldown throttle is process-wide and blocks repeat attempts.
    #[serial]
    #[tokio::test]
    async fn revalidation_is_throttled() {
        reset_revalidation_throttle();
        let (_temp_dir, cache_path) = test_cache_path();
        std::fs::write(&cache_path, VALID_CACHE).expect("Failed to write cache");

        // @step Given the models.dev snapshot on disk is older than the cache TTL
        let cache = ModelCache::new_with_path(cache_path)
            .with_ttl(Duration::ZERO)
            .with_api_url("http://127.0.0.1:1/api.json");
        // @step When the model catalog is revalidated twice in a row
        // @step Then only the first revalidation reaches the network
        assert!(cache.revalidate_if_stale().await, "first attempt claims the slot");
        assert!(
            !cache.revalidate_if_stale().await,
            "second attempt within the cooldown is suppressed"
        );
    }

    /// Scenario: An unreachable catalog never empties the model selector.
    #[serial]
    #[tokio::test]
    async fn unreachable_catalog_still_serves_the_stale_snapshot() {
        reset_revalidation_throttle();
        let (_temp_dir, cache_path) = test_cache_path();
        std::fs::write(&cache_path, VALID_CACHE).expect("Failed to write cache");

        // @step Given the models.dev snapshot on disk is older than the cache TTL
        let cache = ModelCache::new_with_path(cache_path).with_ttl(Duration::ZERO);
        // @step And the catalog endpoint is unreachable
        let cache = cache.with_api_url("http://127.0.0.1:1/api.json");
        // @step When the model catalog is read
        // @step Then the stale snapshot is still served
        let models = cache
            .get()
            .await
            .expect("a snapshot on disk must never turn into an error");
        assert!(models.providers.contains_key("anthropic"));
        // @step And the failure is only logged
        // The revalidation is detached and its failure is warn!-only: it can
        // never surface through get(), and the snapshot on disk survives it.
        assert!(
            revalidation_slot_claimed(),
            "the unreachable endpoint is still attempted (and only logged)"
        );
        let models = cache.get().await.expect("still served after the failed revalidation");
        assert!(models.providers.contains_key("anthropic"));
    }
}
