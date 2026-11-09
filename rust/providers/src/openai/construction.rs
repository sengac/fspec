//! OpenAIProvider constructors and endpoint accessors (REFAC-031: moved from openai.rs).

use super::{normalize_base_url, OpenAIEnvLimits, OpenAIProvider};
use crate::{
    cache_optimization::{CacheOptimizationFacade, SessionAffinityConfig},
    detect_credential_from_env, validate_api_key_static, ProviderError,
};
use rig::providers::openai;

impl OpenAIProvider {
    /// Create a new OpenAIProvider using API key from environment
    ///
    /// Checks for API key in OPENAI_API_KEY environment variable.
    /// Model MUST be provided via OPENAI_MODEL environment variable.
    ///
    /// PROV-006: Also checks OPENAI_BASE_URL for custom endpoint,
    /// OPENAI_CONTEXT_WINDOW and OPENAI_MAX_OUTPUT_TOKENS for custom limits.
    ///
    /// Uses shared detect_credential_from_env() helper (REFAC-013).
    pub fn new() -> Result<Self, ProviderError> {
        // Use shared credential detection helper (REFAC-013)
        let api_key = detect_credential_from_env("openai", &["OPENAI_API_KEY"])?;

        // Model is REQUIRED via OPENAI_MODEL env var
        let model_name = std::env::var("OPENAI_MODEL").map_err(|_| {
            ProviderError::config(
                "openai",
                "Model is required. Set OPENAI_MODEL environment variable.",
            )
        })?;

        // PROV-006: Check for custom base URL and normalize it
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|url| normalize_base_url(&url));

        Self::from_api_key_with_options(&api_key, &model_name, base_url.as_deref(), None)
    }

    /// Create a new OpenAIProvider with an explicit API key and model
    ///
    /// Uses shared validate_api_key_static() helper (REFAC-013).
    /// PROV-051: Delegates to from_api_key_with_options with no session_id.
    /// Use from_api_key_with_session for session-aware construction.
    pub fn from_api_key(api_key: &str, model: &str) -> Result<Self, ProviderError> {
        // Check for custom base URL from environment and normalize it
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|url| normalize_base_url(&url));
        Self::from_api_key_with_options(api_key, model, base_url.as_deref(), None)
    }

    /// Create a new OpenAIProvider with explicit API key, model, and session ID
    ///
    /// PROV-051: Session-aware constructor that reads OPENAI_BASE_URL from env
    /// and applies cache optimization headers when a custom base URL is set.
    pub fn from_api_key_with_session(
        api_key: &str,
        model: &str,
        session_id: uuid::Uuid,
    ) -> Result<Self, ProviderError> {
        let base_url = std::env::var("OPENAI_BASE_URL")
            .ok()
            .map(|url| normalize_base_url(&url));
        Self::from_api_key_with_options(api_key, model, base_url.as_deref(), Some(session_id))
    }

    /// PROV-164: send the session's resolved output limit (profile Max Output
    /// Tokens, model registry, or env override) instead of the constructor's
    /// env value, so the wire limit matches the output reservation.
    ///
    /// PROV-174: the limit is positive by type; a zero limit is the caller's
    /// error to report, never silently ignored.
    pub fn with_max_output_tokens(mut self, max_output_tokens: std::num::NonZeroUsize) -> Self {
        self.max_output_tokens = Some(max_output_tokens.get());
        self
    }

    /// Create a new OpenAIProvider with explicit API key, model, and optional base URL
    ///
    /// PROV-006: New constructor that supports custom base URLs for local model servers.
    /// PROV-051: Accepts optional session_id for cache optimization (session affinity headers).
    ///
    /// # Arguments
    /// * `api_key` - The API key (any non-empty value for local servers)
    /// * `model` - The model name
    /// * `base_url` - Optional custom base URL (e.g., "http://localhost:8888")
    /// * `session_id` - Optional session UUID for cache optimization headers
    pub fn from_api_key_with_options(
        api_key: &str,
        model: &str,
        base_url: Option<&str>,
        session_id: Option<uuid::Uuid>,
    ) -> Result<Self, ProviderError> {
        // Use shared validation helper (REFAC-013)
        validate_api_key_static("openai", api_key)?;

        // PROV-171: explicit limit overrides; unset is None, invalid is an error.
        let OpenAIEnvLimits {
            context_window,
            max_output_tokens,
        } = OpenAIEnvLimits::from_env()?;

        // PROV-140: read the per-profile streaming toggle. Streaming stays
        // enabled unless OPENAI_STREAMING is explicitly "false".
        let streaming = std::env::var("OPENAI_STREAMING")
            .map(|value| !value.trim().eq_ignore_ascii_case("false"))
            .unwrap_or(true);

        // Build rig completions client
        // PROV-006: Use custom base_url if provided (for vLLM, Ollama, etc.)
        // PROV-051: Apply cache optimization headers via facade when session_id is available
        let cache_headers = session_id
            .map(|sid| {
                let config = SessionAffinityConfig::from_env(sid, base_url.is_some());
                CacheOptimizationFacade::build_headers(&config)
            })
            .unwrap_or_default();

        let rig_client = if let Some(url) = base_url {
            let mut builder = openai::CompletionsClient::builder()
                .api_key(api_key)
                .base_url(url);
            if !cache_headers.is_empty() {
                builder = builder.http_headers(cache_headers);
            }
            builder.build().map_err(|e| {
                ProviderError::config(
                    "openai",
                    format!("Failed to build OpenAI client with custom base URL: {e}"),
                )
            })?
        } else {
            openai::CompletionsClient::builder()
                .api_key(api_key)
                .build()
                .map_err(|e| {
                    ProviderError::config("openai", format!("Failed to build OpenAI client: {e}"))
                })?
        };

        // Create completion model using the client
        // PROV-140: thread the per-profile streaming flag onto the model so its
        // `stream()` picks the streaming vs non-streaming request path.
        let completion_model = openai::completion::CompletionModel::new(rig_client.clone(), model)
            .with_stream(streaming);

        Ok(Self {
            completion_model,
            rig_client,
            model_name: model.to_string(),
            base_url: base_url.map(String::from),
            context_window,
            max_output_tokens,
            streaming,
        })
    }

    /// Get the custom base URL if set
    ///
    /// PROV-006: Returns the custom base URL for local model servers.
    /// Returns None if using the default OpenAI API endpoint.
    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    /// Check if this provider is using a custom (local) endpoint
    ///
    /// PROV-006: Returns true if OPENAI_BASE_URL was set.
    pub fn is_local_endpoint(&self) -> bool {
        self.base_url.is_some()
    }
}
