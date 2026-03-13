use codelet_tools::facade::{build_gemini_system_prompt, select_claude_facade};
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
        _ => Err(format!(
            "Unsupported provider for DeepSearch sub-agent: {provider_name}. Supported: claude, openai, gemini, codex, zai"
        )),
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
    use super::{SUB_AGENT_MAX_TOKENS, request_config_for_provider};

    #[test]
    fn codex_request_config_sets_required_responses_api_fields() {
        let config = request_config_for_provider("codex", "gpt-5.1-codex", "deep search prompt", false)
            .expect("codex config should build");

        assert_eq!(config.max_tokens, None);
        assert_eq!(config.additional_params.as_ref().expect("params")["store"], false);
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
        let config = request_config_for_provider("gemini", "gemini-3-pro-preview", "deep search prompt", false)
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
            config.additional_params.as_ref().expect("params")["generationConfig"]["thinkingConfig"]["thinkingLevel"],
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
        assert!(params["system"][0]["text"].as_str().expect("prefix").contains("Claude Code"));
    }
}
