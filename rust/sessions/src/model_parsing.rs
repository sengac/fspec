//! Model parsing helper — RPC-424.
//!
//! Extracts the duplicated model string parsing logic from three call sites
//! in `session_manager.rs` into a single shared function.
//!
//! ## Feature
//! `spec/features/extract-model-parsing-helper.feature`

/// Result of parsing a model string.
///
/// Contains all fields extracted from the model string:
/// - `registry_provider`: The provider name (e.g., "anthropic")
/// - `model_part`: The model identifier (e.g., "claude-opus-4-5")
/// - `is_profile_model`: True if the model string has a colon prefix (e.g., "profile:anthropic/claude-opus-4")
/// - `is_codex_model`: True if the model string starts with "codex/"
/// - `is_custom_model`: True if the provider is registered as a custom provider
///   (only meaningful when not a profile or codex model)
#[derive(Debug)]
pub struct ModelParseResult<'a> {
    /// The provider name extracted from the model string.
    pub registry_provider: &'a str,
    /// The model identifier extracted from the model string.
    pub model_part: &'a str,
    /// True if the model string has a colon prefix (profile model).
    pub is_profile_model: bool,
    /// True if the model string starts with "codex/".
    pub is_codex_model: bool,
    /// True if the provider is registered as a custom provider.
    /// Only meaningful when `is_profile_model` and `is_codex_model` are both false.
    pub is_custom_model: bool,
}

/// Parse a model string into its components.
///
/// Accepts model strings in the format `provider/model-id` with optional
/// profile prefix (`profile:provider/model-id`) or codex prefix (`codex/model-id`).
///
/// # Errors
/// Returns an error if:
/// - The model string is empty
/// - The model string does not contain a '/'
/// - The provider or model part is empty after parsing
///
/// # Examples
/// ```ignore
/// // Standard provider/model
/// parse_model_string("anthropic/claude-sonnet-4")
/// // Returns: registry_provider="anthropic", model_part="claude-sonnet-4", is_profile=false, is_codex=false
///
/// // Profile model
/// parse_model_string("profile:anthropic/claude-opus-4")
/// // Returns: registry_provider="anthropic", model_part="claude-opus-4", is_profile=true
///
/// // Codex model
/// parse_model_string("codex/codex-model")
/// // Returns: registry_provider="codex", model_part="codex-model", is_codex=true
/// ```
pub fn parse_model_string(model: &str) -> Result<ModelParseResult<'_>, String> {
    // Validate model string contains '/' and is not empty
    if !model.contains('/') || model.is_empty() {
        return Err(format!(
            "Invalid model string '{model}': must be in 'provider/model-id' format (e.g., 'anthropic/claude-opus-4-5')"
        ));
    }

    let is_profile_model = model.contains(':') && model.find(':') < model.find('/');
    let is_codex_model = model.starts_with("codex/");

    let (registry_provider, model_part) = if is_profile_model {
        let colon_idx = model
            .find(':')
            .ok_or_else(|| format!("Invalid profile model string '{model}': missing ':'"))?;
        let slash_idx = model
            .find('/')
            .ok_or_else(|| format!("Invalid profile model string '{model}': missing '/'"))?;
        // For profile models (e.g., "profile:anthropic/claude-opus-4"),
        // extract the provider between ':' and '/'
        let provider = &model[colon_idx + 1..slash_idx];
        let model_id = &model[slash_idx + 1..];
        (provider, model_id)
    } else {
        let parts: Vec<&str> = model.splitn(2, '/').collect();
        (parts[0], parts.get(1).copied().unwrap_or(""))
    };

    // Validate non-empty provider and model part
    if registry_provider.is_empty() || model_part.is_empty() {
        return Err(format!(
            "Invalid model string '{model}': must be in 'provider/model-id' format (e.g., 'anthropic/claude-opus-4-5')"
        ));
    }

    let is_custom_model = !is_profile_model
        && !is_codex_model
        && codelet_providers::custom_provider_registered(registry_provider);

    Ok(ModelParseResult {
        registry_provider,
        model_part,
        is_profile_model,
        is_codex_model,
        is_custom_model,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Feature: spec/features/extract-model-parsing-helper.feature
    /// Scenario: Parse standard provider/model string
    #[test]
    fn parse_standard_model_string() {
        // @step Given the model string "anthropic/claude-sonnet-4"
        let model = "anthropic/claude-sonnet-4";

        // @step When parse_model_string is called
        let result = parse_model_string(model).expect("should parse successfully");

        // @step Then it returns registry_provider "anthropic" and model_part "claude-sonnet-4"
        assert_eq!(result.registry_provider, "anthropic");
        assert_eq!(result.model_part, "claude-sonnet-4");

        // @step And is_profile_model is false
        assert!(!result.is_profile_model);

        // @step And is_codex_model is false
        assert!(!result.is_codex_model);
    }

    /// Feature: spec/features/extract-model-parsing-helper.feature
    /// Scenario: Parse profile model string with colon prefix
    #[test]
    fn parse_profile_model_string() {
        // @step Given the model string "profile:anthropic/claude-opus-4"
        let model = "profile:anthropic/claude-opus-4";

        // @step When parse_model_string is called
        let result = parse_model_string(model).expect("should parse successfully");

        // @step Then it returns registry_provider "anthropic" and model_part "claude-opus-4"
        assert_eq!(result.registry_provider, "anthropic");
        assert_eq!(result.model_part, "claude-opus-4");

        // @step And is_profile_model is true
        assert!(result.is_profile_model);
    }

    /// Feature: spec/features/extract-model-parsing-helper.feature
    /// Scenario: Parse codex model string
    #[test]
    fn parse_codex_model_string() {
        // @step Given the model string "codex/codex-model"
        let model = "codex/codex-model";

        // @step When parse_model_string is called
        let result = parse_model_string(model).expect("should parse successfully");

        // @step Then it returns registry_provider "codex" and model_part "codex-model"
        assert_eq!(result.registry_provider, "codex");
        assert_eq!(result.model_part, "codex-model");

        // @step And is_codex_model is true
        assert!(result.is_codex_model);
    }

    /// Feature: spec/features/extract-model-parsing-helper.feature
    /// Scenario: Reject invalid model string without slash
    #[test]
    fn reject_model_string_without_slash() {
        // @step Given the model string "invalid"
        let model = "invalid";

        // @step When parse_model_string is called
        let result = parse_model_string(model);

        // @step Then it returns an error with a validation message
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Invalid model string"));
        assert!(error.contains("must be in 'provider/model-id' format"));
    }

    /// Feature: spec/features/extract-model-parsing-helper.feature
    /// Scenario: Reject empty model string
    #[test]
    fn reject_empty_model_string() {
        // @step Given the model string ""
        let model = "";

        // @step When parse_model_string is called
        let result = parse_model_string(model);

        // @step Then it returns an error with a validation message
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Invalid model string"));
    }

    /// Feature: spec/features/extract-model-parsing-helper.feature
    /// Scenario: Model string with empty provider part
    #[test]
    fn reject_model_string_with_empty_provider() {
        // @step Given the model string "/model-id"
        let model = "/model-id";

        // @step When parse_model_string is called
        let result = parse_model_string(model);

        // @step Then it returns an error with a validation message
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Invalid model string"));
    }

    /// Feature: spec/features/extract-model-parsing-helper.feature
    /// Scenario: Model string with empty model part
    #[test]
    fn reject_model_string_with_empty_model_part() {
        // @step Given the model string "provider/"
        let model = "provider/";

        // @step When parse_model_string is called
        let result = parse_model_string(model);

        // @step Then it returns an error with a validation message
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Invalid model string"));
    }

    /// Feature: spec/features/extract-model-parsing-helper.feature
    /// Scenario: All three call sites use the shared helper
    #[test]
    fn all_three_call_sites_use_shared_helper() {
        // @step Given the parse_model_string helper exists in model_parsing.rs
        let model_parsing_content = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("model_parsing.rs"),
        )
        .expect("model_parsing.rs should exist");
        assert!(
            model_parsing_content.contains("pub fn parse_model_string"),
            "model_parsing.rs must contain the parse_model_string function"
        );

        // @step When session_manager.rs is examined
        let sm_content = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("session_manager.rs"),
        )
        .expect("session_manager.rs should exist");

        // @step Then create_session_with_id calls parse_model_string
        assert!(
            sm_content.contains("crate::model_parsing::parse_model_string"),
            "session_manager.rs must call crate::model_parsing::parse_model_string"
        );

        // @step And create_session_from_manifest calls parse_model_string
        let call_count = sm_content.matches("crate::model_parsing::parse_model_string").count();
        assert!(
            call_count >= 3,
            "session_manager.rs must call parse_model_string at least 3 times (found {call_count})"
        );

        // @step And create_isolated_session_with_id calls parse_model_string
        let old_pattern = "let is_profile_model = model.contains(':')";
        assert!(
            !sm_content.contains(old_pattern),
            "session_manager.rs should not contain the old inline is_profile_model pattern"
        );
    }
}
