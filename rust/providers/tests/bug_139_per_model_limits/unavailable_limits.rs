//! PROV-170: RhaiCustomProvider reports undeclared limits as unavailable, an undeclared
//! model output limit resolves to the declared `defaults.max_output_tokens`, and a declared
//! 0 limit is rejected when the config loads.

use super::scenarios::MANDATORY_LIFECYCLE_STUBS;
use super::*;
use codelet_providers::custom::Defaults;

/// Build a `RhaiCustomProvider` for model alias `opus-4.7` with the given model limits and
/// `defaults.max_output_tokens`; the script defines only the mandatory lifecycle hooks.
fn rhai_provider(
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
    default_max_output_tokens: Option<usize>,
) -> RhaiCustomProvider {
    let tmp = TempDir::new().expect("tempdir");
    let script_path = tmp.path().join("claude-rhai.rhai");
    fs::write(&script_path, MANDATORY_LIFECYCLE_STUBS).expect("write rhai script");
    let mut models = HashMap::new();
    models.insert(
        "opus-4.7".to_string(),
        ModelDef {
            id: "claude-opus-4-7".to_string(),
            context_window,
            max_output_tokens,
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );
    let cfg = ProviderConfig {
        name: "claude-rhai".to_string(),
        display_name: "Claude Rhai".to_string(),
        base_url: "https://example.test".to_string(),
        script: script_path.to_string_lossy().to_string(),
        facade: None,
        api_key_env_var: None,
        auth: AuthConfig::Bearer {
            env_var: "TEST_KEY".to_string(),
            token_prefix: "Bearer".to_string(),
        },
        models,
        defaults: default_max_output_tokens.map(|max_output_tokens| Defaults {
            model: None,
            temperature: None,
            max_output_tokens: Some(max_output_tokens),
        }),
        system_prompt: None,
        tool_style: ToolStyle::Openai,
        api_style: ApiStyle::OpenaiChat,
        headers: HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };
    let loader = Arc::new(ScriptLoader::with_default_engine());
    let provider = RhaiCustomProvider::new(Arc::new(cfg), loader, "opus-4.7".to_string())
        .expect("provider builds");
    drop(tmp);
    provider
}

#[test]
fn rhai_custom_provider_reports_undeclared_limits_as_unavailable() {
    // @step Given a Rhai provider config whose model "opus-4.7" declares no context_window and no max_output_tokens
    // @step And the config declares no defaults.max_output_tokens
    // @step And the Rhai script does not define get_model_limits
    // @step When the RhaiCustomProvider is constructed for "opus-4.7"
    let provider = rhai_provider(None, None, None);

    // @step Then its context window is unavailable
    assert_eq!(provider.context_window(), None);

    // @step And its max output tokens is unavailable
    assert_eq!(provider.max_output_tokens(), None);
}

#[test]
fn a_model_without_max_output_tokens_uses_the_declared_defaults_max_output_tokens() {
    // @step Given a Rhai provider config whose model "opus-4.7" declares no max_output_tokens
    // @step And the config declares defaults.max_output_tokens 8192
    // @step When the RhaiCustomProvider is constructed for "opus-4.7"
    let provider = rhai_provider(Some(200_000), None, Some(8192));

    // @step Then its max output tokens is 8192
    assert_eq!(provider.max_output_tokens(), Some(8192));
    assert_eq!(provider.context_window(), Some(200_000));
}

/// Write a facade config (no script needed) and load it through `ProviderConfig::from_file`.
fn load_config(model: serde_json::Value, defaults: Option<serde_json::Value>) -> String {
    let tmp = TempDir::new().expect("tempdir");
    let mut cfg = json!({
        "name": "zero-check",
        "display_name": "Zero Check",
        "facade": "claude",
        "base_url": "https://example.test",
        "models": { "opus-4.7": model }
    });
    if let Some(defaults) = defaults {
        cfg["defaults"] = defaults;
    }
    let cfg_path = tmp.path().join("zero-check.json");
    fs::write(&cfg_path, serde_json::to_string_pretty(&cfg).unwrap()).unwrap();
    ProviderConfig::from_file(&cfg_path)
        .expect_err("a declared 0 limit must be rejected")
        .to_string()
}

#[test]
fn a_declared_zero_limit_is_rejected_when_the_config_is_loaded() {
    // @step Given a ProviderConfig JSON whose model "opus-4.7" declares context_window 0
    let model = json!({ "id": "claude-opus-4-7", "context_window": 0 });

    // @step When the config is loaded from disk
    let error = load_config(model, None);

    // @step Then loading fails with an error naming "models.opus-4.7.context_window"
    assert!(
        error.contains("models.opus-4.7.context_window"),
        "error must name the field: {error}"
    );

    // @step And a config declaring defaults.max_output_tokens 0 fails naming "defaults.max_output_tokens"
    let error = load_config(
        json!({ "id": "claude-opus-4-7", "context_window": 200000 }),
        Some(json!({ "max_output_tokens": 0 })),
    );
    assert!(
        error.contains("defaults.max_output_tokens"),
        "error must name the field: {error}"
    );
}
