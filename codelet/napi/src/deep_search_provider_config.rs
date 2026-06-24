//! DeepSearch sub-agent request configuration per provider.
//!
//! Feature: spec/features/github-copilot-end-to-end-integration.feature
//! Feature: spec/features/glm-zai-deepsearch-fails-with-500-internal-server-error.feature
//!
//! This module validates the acceptance criteria defined in those feature files.
//! Scenarios map directly to Gherkin scenarios in the tests below.

use codelet_tools::facade::{
    build_gemini_system_prompt, select_claude_facade, BoxedSystemPromptFacade,
    OpenAISystemPromptFacade,
};
use serde_json::json;

pub(crate) const SUB_AGENT_MAX_TOKENS: u64 = 8192;

#[derive(Debug, Clone)]
pub(crate) struct DeepSearchRequestConfig {
    pub preamble: String,
    pub additional_params: Option<serde_json::Value>,
    pub max_tokens: Option<u64>,
}

pub(crate) fn request_config_for_provider(
    provider_name: &str,
    model_name: &str,
    system_prompt: &str,
    is_claude_oauth: bool,
) -> Result<DeepSearchRequestConfig, String> {
    match provider_name {
        "claude" => Ok(claude_request_config(system_prompt, is_claude_oauth)),
        "openai" => Ok(default_request_config(system_prompt)),
        "gemini" => Ok(gemini_request_config(model_name, system_prompt)),
        "codex" => Ok(codex_request_config(system_prompt)),
        "zai" => Ok(zai_request_config(system_prompt)),
        "github-copilot" | "copilot" => Ok(copilot_request_config(model_name, system_prompt)),
        _ => Err(format!(
            "Unsupported provider for DeepSearch sub-agent: {provider_name}. Supported: claude, openai, gemini, codex, zai, github-copilot"
        )),
    }
}

/// PROV-057 Layer 3 — Select the appropriate system-prompt facade for a
/// GitHub Copilot model.
///
/// Copilot's `/chat/completions` endpoint is wire-compatible with OpenAI
/// (see `codelet/providers/src/copilot/system_prompt_facade.rs`), so we
/// reuse [`OpenAISystemPromptFacade`] here rather than introducing a
/// parallel facade hierarchy. The `model_name` argument is accepted so
/// this selector can grow a model-family-aware dispatch (mirroring
/// `select_copilot_behavior_facade` in `codelet/providers/src/copilot/`)
/// without changing the call-site signature.
///
/// Feature: spec/features/github-copilot-end-to-end-integration.feature
#[must_use]
pub(crate) fn select_copilot_facade(_model_name: &str) -> BoxedSystemPromptFacade {
    Box::new(OpenAISystemPromptFacade)
}

fn copilot_request_config(model_name: &str, system_prompt: &str) -> DeepSearchRequestConfig {
    let facade = select_copilot_facade(model_name);
    DeepSearchRequestConfig {
        preamble: facade.transform_preamble(system_prompt),
        additional_params: Some(json!({
            "metadata": { "mode": "agent" }
        })),
        max_tokens: Some(SUB_AGENT_MAX_TOKENS),
    }
}

fn default_request_config(system_prompt: &str) -> DeepSearchRequestConfig {
    DeepSearchRequestConfig {
        preamble: system_prompt.to_string(),
        additional_params: None,
        max_tokens: Some(SUB_AGENT_MAX_TOKENS),
    }
}

fn claude_request_config(system_prompt: &str, is_oauth: bool) -> DeepSearchRequestConfig {
    let facade = select_claude_facade(is_oauth);
    DeepSearchRequestConfig {
        preamble: facade.transform_preamble(system_prompt),
        additional_params: Some(json!({
            "system": facade.format_for_api(system_prompt)
        })),
        max_tokens: Some(SUB_AGENT_MAX_TOKENS),
    }
}

fn gemini_request_config(model_name: &str, system_prompt: &str) -> DeepSearchRequestConfig {
    let mut generation_config = json!({
        "temperature": 1.0,
        "topP": 0.95,
        "topK": 64
    });

    if model_name.contains("gemini-3") {
        generation_config["thinkingConfig"] = json!({
            "includeThoughts": true,
            "thinkingLevel": "high"
        });
    }

    DeepSearchRequestConfig {
        preamble: build_gemini_system_prompt(model_name, Some(system_prompt)),
        additional_params: Some(json!({
            "generationConfig": generation_config
        })),
        max_tokens: Some(SUB_AGENT_MAX_TOKENS),
    }
}

fn codex_request_config(system_prompt: &str) -> DeepSearchRequestConfig {
    DeepSearchRequestConfig {
        preamble: system_prompt.to_string(),
        additional_params: Some(json!({
            "store": false,
            "include": ["reasoning.encrypted_content"],
            "reasoning": {
                "effort": "high",
                "summary": "auto"
            }
        })),
        max_tokens: None,
    }
}

fn zai_request_config(system_prompt: &str) -> DeepSearchRequestConfig {
    DeepSearchRequestConfig {
        preamble: system_prompt.to_string(),
        additional_params: Some(json!({
            "temperature": 1.0,
            "top_p": 0.95,
            "max_tokens": SUB_AGENT_MAX_TOKENS
        })),
        max_tokens: Some(SUB_AGENT_MAX_TOKENS),
    }
}

#[cfg(test)]
mod tests {
    use super::{request_config_for_provider, SUB_AGENT_MAX_TOKENS};

    #[test]
    fn codex_request_config_sets_required_responses_api_fields() {
        let config =
            request_config_for_provider("codex", "gpt-5.1-codex", "deep search prompt", false)
                .expect("codex config should build");

        assert_eq!(config.max_tokens, None);
        assert_eq!(
            config.additional_params.as_ref().expect("params")["store"],
            false
        );
        assert_eq!(
            config.additional_params.as_ref().expect("params")["include"][0],
            "reasoning.encrypted_content"
        );
        assert_eq!(
            config.additional_params.as_ref().expect("params")["reasoning"]["effort"],
            "high"
        );
    }

    #[test]
    fn gemini_request_config_uses_model_aware_prompt_and_generation_defaults() {
        let config = request_config_for_provider(
            "gemini",
            "gemini-3-pro-preview",
            "deep search prompt",
            false,
        )
        .expect("gemini config should build");

        assert_eq!(config.max_tokens, Some(SUB_AGENT_MAX_TOKENS));
        assert!(config.preamble.contains("interactive CLI agent"));
        assert!(config.preamble.contains("deep search prompt"));
        assert_eq!(
            config.additional_params.as_ref().expect("params")["generationConfig"]["temperature"],
            1.0
        );
        assert_eq!(
            config.additional_params.as_ref().expect("params")["generationConfig"]["topP"],
            0.95
        );
        assert_eq!(
            config.additional_params.as_ref().expect("params")["generationConfig"]
                ["thinkingConfig"]["thinkingLevel"],
            "high"
        );
    }

    // Scenario: ZAI DeepSearch config includes max_tokens in additional_params
    // Feature: spec/features/glm-zai-deepsearch-fails-with-500-internal-server-error.feature

    // @step Given a DeepSearch request config is built for provider "zai"
    #[test]
    fn zai_request_config_sets_glm_defaults() {
        let config = request_config_for_provider("zai", "glm-4.7", "deep search prompt", false)
            .expect("zai config should build");

        // @step When the config is serialized for the HTTP request
        let params = config.additional_params.as_ref().expect("params");

        assert_eq!(config.max_tokens, Some(SUB_AGENT_MAX_TOKENS));

        // @step Then the additional_params includes max_tokens set to 8192
        assert_eq!(params["max_tokens"], SUB_AGENT_MAX_TOKENS);

        // @step Then the additional_params includes temperature and top_p
        assert_eq!(params["temperature"], 1.0);
        assert_eq!(params["top_p"], 0.95);
    }

    #[test]
    fn claude_request_config_uses_oauth_facade_formatting() {
        let config = request_config_for_provider(
            "claude",
            "claude-sonnet-4-20250514",
            "deep search prompt",
            true,
        )
        .expect("claude config should build");

        assert_eq!(config.max_tokens, Some(SUB_AGENT_MAX_TOKENS));
        assert!(config.preamble.contains("You are Claude Code"));
        let params = config.additional_params.as_ref().expect("params");
        assert!(params["system"].is_array());
        assert!(params["system"][0]["text"]
            .as_str()
            .expect("prefix")
            .contains("Claude Code"));
    }

    // =========================================================================
    // Feature: spec/features/github-copilot-end-to-end-integration.feature
    // PROV-057 Layer 3 — DeepSearch sub-agent support for github-copilot
    // =========================================================================

    // Scenario: DeepSearch sub-agents can use github-copilot as their provider

    // @step Given a session is configured with provider "github-copilot"
    // @step When a DeepSearch sub-agent is spawned
    // @step Then request_config_for_provider("github-copilot", model, prompt, false) returns Ok
    // @step And the returned config preamble is built using select_copilot_facade
    // @step And the returned config does NOT trigger the "Unsupported provider for DeepSearch sub-agent" error
    #[test]
    fn github_copilot_request_config_uses_select_copilot_facade() {
        // @step Given a session is configured with provider "github-copilot"
        // @step When a DeepSearch sub-agent is spawned
        // @step Then request_config_for_provider("github-copilot", model, prompt, false) returns Ok
        let config =
            request_config_for_provider("github-copilot", "gpt-4o", "deep search prompt", false)
                .expect("github-copilot config should build");

        // @step And the returned config preamble is built using select_copilot_facade
        // The preamble must match exactly what select_copilot_facade applied to
        // the prompt would produce — this proves we routed through the
        // facade instead of a trivial passthrough.
        let facade = super::select_copilot_facade("gpt-4o");
        let expected_preamble = facade.transform_preamble("deep search prompt");
        assert_eq!(
            config.preamble, expected_preamble,
            "preamble must be built by select_copilot_facade"
        );

        // @step And the returned config does NOT trigger the "Unsupported provider for DeepSearch sub-agent" error
        assert_eq!(config.max_tokens, Some(SUB_AGENT_MAX_TOKENS));
    }

    // @step Given a session is configured with provider "copilot"
    // @step When a DeepSearch sub-agent is spawned
    // @step Then request_config_for_provider("copilot", model, prompt, false) returns Ok
    // @step And the returned config does NOT trigger the "Unsupported provider for DeepSearch sub-agent" error
    #[test]
    fn copilot_short_alias_request_config_builds_successfully() {
        let config = request_config_for_provider(
            "copilot",
            "claude-sonnet-4.5",
            "deep search prompt",
            false,
        )
        .expect("copilot alias should build a config");

        assert_eq!(config.max_tokens, Some(SUB_AGENT_MAX_TOKENS));
        assert!(config.preamble.contains("deep search prompt"));
    }

    #[test]
    fn unknown_provider_still_returns_unsupported_error() {
        let result = request_config_for_provider(
            "not-a-real-provider",
            "gpt-4o",
            "deep search prompt",
            false,
        );
        let err = result.expect_err("unknown provider must return error");
        assert!(err.contains("Unsupported provider for DeepSearch sub-agent"));
    }
}
