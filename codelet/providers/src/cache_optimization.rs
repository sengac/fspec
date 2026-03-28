//! PROV-051: Cache optimization facade for OpenAI-compatible providers
//!
//! Encapsulates session affinity header logic using the Facade pattern.
//! Separates cache optimization concerns from provider construction,
//! making it extensible for future provider-specific optimizations.
//!
//! Feature: spec/features/openai-session-affinity.feature

use http::{HeaderMap, HeaderName, HeaderValue};
use uuid::Uuid;

/// Configuration for session affinity cache optimization.
///
/// Encapsulates all inputs needed to determine the correct
/// cache optimization headers for an OpenAI-compatible provider.
#[derive(Debug, Clone)]
pub struct SessionAffinityConfig {
    /// The codelet session UUID — used as default affinity value
    pub session_id: Uuid,
    /// Optional override from OPENAI_SESSION_AFFINITY env var
    pub affinity_override: Option<String>,
    /// Whether a custom base URL is set (i.e., not using default OpenAI API)
    pub has_custom_base_url: bool,
}

impl SessionAffinityConfig {
    /// Create a new SessionAffinityConfig from runtime context.
    ///
    /// Reads OPENAI_SESSION_AFFINITY from environment for override value.
    ///
    /// # Arguments
    /// * `session_id` - The codelet session UUID
    /// * `has_custom_base_url` - Whether OPENAI_BASE_URL is set
    pub fn from_env(session_id: Uuid, has_custom_base_url: bool) -> Self {
        let affinity_override = std::env::var("OPENAI_SESSION_AFFINITY").ok();
        Self {
            session_id,
            affinity_override,
            has_custom_base_url,
        }
    }

    /// Create a config with explicit values (for testing).
    pub fn new(
        session_id: Uuid,
        affinity_override: Option<String>,
        has_custom_base_url: bool,
    ) -> Self {
        Self {
            session_id,
            affinity_override,
            has_custom_base_url,
        }
    }

    /// Get the effective affinity value.
    ///
    /// Returns the OPENAI_SESSION_AFFINITY env var if set,
    /// otherwise the session UUID string.
    pub fn affinity_value(&self) -> String {
        self.affinity_override
            .clone()
            .unwrap_or_else(|| self.session_id.to_string())
    }

    /// Whether session affinity headers should be applied.
    ///
    /// Only applies when using a custom base URL (Fireworks, vLLM, etc.).
    /// Default OpenAI API handles caching server-side without headers.
    pub fn should_apply(&self) -> bool {
        self.has_custom_base_url
    }
}

/// Facade for building cache optimization HTTP headers.
///
/// Centralizes the logic for determining which headers to send
/// based on provider configuration. Currently supports:
/// - `x-session-affinity` for Fireworks.ai (and any provider that honors it)
///
/// Designed to be extensible for future provider-specific headers
/// (e.g., OpenAI's `prompt_cache_key`, Groq-specific headers, etc.).
pub struct CacheOptimizationFacade;

impl CacheOptimizationFacade {
    /// Build HTTP headers for cache optimization.
    ///
    /// Returns a HeaderMap containing any cache optimization headers
    /// that should be applied to the OpenAI-compatible client.
    ///
    /// # Arguments
    /// * `config` - Session affinity configuration
    ///
    /// # Returns
    /// A HeaderMap (possibly empty if no headers should be applied)
    pub fn build_headers(config: &SessionAffinityConfig) -> HeaderMap {
        let mut headers = HeaderMap::new();

        if config.should_apply() {
            let value = config.affinity_value();
            if let Ok(header_value) = HeaderValue::from_str(&value) {
                headers.insert(
                    HeaderName::from_static("x-session-affinity"),
                    header_value,
                );
            }
        }

        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Feature: spec/features/openai-session-affinity.feature
    //
    // This test module validates the acceptance criteria defined in the feature file.
    // Scenarios map directly to Gherkin scenarios.
    // =========================================================================

    // =========================================================================
    // Scenario: Session affinity header is set when using custom base URL
    // =========================================================================
    #[test]
    #[serial_test::serial]
    fn test_session_affinity_header_set_with_custom_base_url() {
        // @step Given OPENAI_BASE_URL is set to "https://api.fireworks.ai/inference"
        // (Simulated by has_custom_base_url = true)

        // @step And OPENAI_API_KEY is set to "fw-test-key"
        // (Not relevant for header construction — handled by provider)

        // @step And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
        std::env::remove_var("OPENAI_SESSION_AFFINITY");
        let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        // @step When an OpenAI provider is created with that session ID
        let config = SessionAffinityConfig::new(session_id, None, true);
        let headers = CacheOptimizationFacade::build_headers(&config);

        // @step Then the rig client headers should include "x-session-affinity" with value "550e8400-e29b-41d4-a716-446655440000"
        assert_eq!(
            headers.get("x-session-affinity").unwrap().to_str().unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    // =========================================================================
    // Scenario: Session affinity header uses custom value from environment
    // =========================================================================
    #[test]
    #[serial_test::serial]
    fn test_session_affinity_header_uses_custom_env_value() {
        // @step Given OPENAI_BASE_URL is set to "https://api.fireworks.ai/inference"
        // (has_custom_base_url = true)

        // @step And OPENAI_API_KEY is set to "fw-test-key"
        // (Not relevant for header construction)

        // @step And OPENAI_SESSION_AFFINITY is set to "my-custom-session"
        std::env::set_var("OPENAI_SESSION_AFFINITY", "my-custom-session");

        // @step And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
        let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        // @step When an OpenAI provider is created with that session ID
        let config = SessionAffinityConfig::from_env(session_id, true);
        let headers = CacheOptimizationFacade::build_headers(&config);

        // @step Then the rig client headers should include "x-session-affinity" with value "my-custom-session"
        assert_eq!(
            headers.get("x-session-affinity").unwrap().to_str().unwrap(),
            "my-custom-session"
        );

        // Cleanup
        std::env::remove_var("OPENAI_SESSION_AFFINITY");
    }

    // =========================================================================
    // Scenario: Session affinity header is sent for any custom base URL endpoint
    // =========================================================================
    #[test]
    #[serial_test::serial]
    fn test_session_affinity_header_sent_for_any_custom_url() {
        // @step Given OPENAI_BASE_URL is set to "http://localhost:8888"
        // (has_custom_base_url = true — vLLM local server)

        // @step And OPENAI_API_KEY is set to "test-key"
        // (Not relevant for header construction)

        // @step And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
        std::env::remove_var("OPENAI_SESSION_AFFINITY");
        let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        // @step When an OpenAI provider is created with that session ID
        let config = SessionAffinityConfig::new(session_id, None, true);
        let headers = CacheOptimizationFacade::build_headers(&config);

        // @step Then the rig client headers should include "x-session-affinity" with value "550e8400-e29b-41d4-a716-446655440000"
        assert_eq!(
            headers.get("x-session-affinity").unwrap().to_str().unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    // =========================================================================
    // Scenario: No session affinity header when using default OpenAI API
    // =========================================================================
    #[test]
    #[serial_test::serial]
    fn test_no_session_affinity_header_for_default_openai() {
        // @step Given OPENAI_BASE_URL is not set
        // (has_custom_base_url = false)

        // @step And OPENAI_API_KEY is set to "sk-test-key"
        // (Not relevant for header construction)

        // @step And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
        std::env::remove_var("OPENAI_SESSION_AFFINITY");
        let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        // @step When an OpenAI provider is created with that session ID
        let config = SessionAffinityConfig::new(session_id, None, false);
        let headers = CacheOptimizationFacade::build_headers(&config);

        // @step Then the rig client headers should not include "x-session-affinity"
        assert!(headers.get("x-session-affinity").is_none());
    }

    // =========================================================================
    // Additional unit tests for SessionAffinityConfig
    // =========================================================================

    #[test]
    fn test_affinity_value_defaults_to_session_id() {
        let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let config = SessionAffinityConfig::new(session_id, None, true);
        assert_eq!(config.affinity_value(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_affinity_value_uses_override() {
        let session_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let config = SessionAffinityConfig::new(
            session_id,
            Some("custom-affinity".to_string()),
            true,
        );
        assert_eq!(config.affinity_value(), "custom-affinity");
    }

    #[test]
    fn test_should_apply_true_for_custom_url() {
        let config = SessionAffinityConfig::new(Uuid::new_v4(), None, true);
        assert!(config.should_apply());
    }

    #[test]
    fn test_should_apply_false_for_default_url() {
        let config = SessionAffinityConfig::new(Uuid::new_v4(), None, false);
        assert!(!config.should_apply());
    }
}
