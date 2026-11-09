//! bug_139_custom_provider_per_model_limits_test.rs: per-model limit, unavailable-limit (PROV-170) and script-override scenarios.

use super::*;

// =========================================================================
// Scenario: NAPI listProviders returns per-model limits for custom providers
// =========================================================================
#[test]
#[serial]
fn napi_list_providers_returns_per_model_limits_for_custom_providers() {
    // @step Given the claude-rhai provider config declares model "opus-4.7" with context_window 1000000 and max_output_tokens 128000 and supports_tools true and supports_streaming true and supports_thinking true
    let fx = DiscoveryFixture::new();
    let _api_key = EnvGuard::set("ANTHROPIC_API_KEY", "sk-not-real");
    write_claude_rhai_config(
        fx.project_root(),
        "claude-rhai",
        Some(1_000_000),
        Some(128_000),
        Some(true),
        Some(true),
        Some(true),
    );

    // @step When I call list_providers_info() via the custom-provider NAPI surface
    let providers = list_providers_info().expect("list ok");

    // @step Then the returned "claude-rhai" entry has models containing one item
    let entry = providers
        .iter()
        .find(|p| p.name == "claude-rhai")
        .expect("claude-rhai present");
    assert_eq!(
        entry.models.len(),
        1,
        "claude-rhai should report exactly one model"
    );
    let model: &ProviderModelInfo = &entry.models[0];

    // @step And that item has id "opus-4.7"
    assert_eq!(model.id, "opus-4.7");

    // @step And that item has contextWindow 1000000
    assert_eq!(
        model.context_window,
        Some(1_000_000),
        "context_window must round-trip from JSON ModelDef"
    );

    // @step And that item has maxOutput 128000
    assert_eq!(model.max_output_tokens, Some(128_000));

    // @step And that item has supportsTools true
    assert!(model.supports_tools);

    // @step And that item has supportsStreaming true
    assert!(model.supports_streaming);

    // @step And that item has supportsThinking true
    assert!(model.supports_thinking);
}

// =========================================================================
// Scenario: A ProviderConfig model that declares no limits has them unavailable
// =========================================================================
#[test]
fn a_provider_config_model_that_declares_no_limits_has_them_unavailable() {
    // @step Given a ProviderConfig JSON that omits context_window and max_output_tokens on every model
    let tmp = TempDir::new().expect("tempdir");
    let script_path = tmp.path().join("noop.rhai");
    fs::write(&script_path, "fn noop() { }\n").unwrap();

    let cfg_json = json!({
        "name": "default-ctx-check",
        "display_name": "Default CW Check",
        "base_url": "https://example.test",
        "script": script_path.file_name().unwrap().to_string_lossy(),
        "auth": { "type": "bearer", "env_var": "UNUSED_KEY" },
        "models": {
            "a": { "id": "model-a" },
            "b": { "id": "model-b" }
        }
    });
    let cfg_path = tmp.path().join("default-ctx-check.json");
    fs::write(&cfg_path, serde_json::to_string_pretty(&cfg_json).unwrap()).unwrap();

    // @step When serde deserializes the config
    let cfg = ProviderConfig::from_file(&cfg_path).expect("config loads");

    for (alias, def) in &cfg.models {
        // @step Then every model's context_window is unavailable
        assert_eq!(
            def.context_window, None,
            "model '{alias}' guessed a context window"
        );
        // @step And every model's max_output_tokens is unavailable
        assert_eq!(
            def.max_output_tokens, None,
            "model '{alias}' guessed an output limit"
        );
    }
}

// =========================================================================
// Scenario: JSON omits context_window - list_providers_info reports it unavailable
// =========================================================================
#[test]
#[serial]
fn json_omits_context_window_list_providers_reports_it_unavailable() {
    // @step Given the claude-rhai provider config declares model "opus-4.7" without a context_window field
    // @step And the Rhai script does not define get_model_limits
    let fx = DiscoveryFixture::new();
    let _api_key = EnvGuard::set("ANTHROPIC_API_KEY", "sk-not-real");
    write_claude_rhai_config(
        fx.project_root(),
        "claude-rhai",
        None,
        None,
        None,
        None,
        None,
    );

    // @step When list_providers_info() resolves per-model limits
    let providers = list_providers_info().expect("list ok");
    let entry = providers
        .iter()
        .find(|p| p.name == "claude-rhai")
        .expect("claude-rhai present");
    let model = entry
        .models
        .iter()
        .find(|m| m.id == "opus-4.7")
        .expect("opus-4.7 alias present");

    // @step Then the returned model entry's contextWindow is unavailable
    assert_eq!(model.context_window, None);

    // @step And the returned model entry's maxOutput is unavailable
    assert_eq!(model.max_output_tokens, None);
}

// =========================================================================
// Scenario: Rhai script get_model_limits still wins over JSON (PROV-095 no-regression)
// =========================================================================

/// Mandatory 7 Rhai lifecycle stubs the custom provider constructor
/// validates (copied from `rhai_scripted_model_limits_tests.rs`).
pub(crate) const MANDATORY_LIFECYCLE_STUBS: &str = r#"
fn build_url(config)             { "" }
fn build_headers(config)         { #{} }
fn build_request(request)        { #{} }
fn build_stream_request(request) { #{} }
fn parse_response(raw)           { #{ content: [], stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk)     { #{} }
fn map_error(status, body)       { #{} }
"#;

#[test]
fn rhai_script_get_model_limits_still_wins_over_json() {
    // @step Given the claude-rhai provider config declares model "opus-4.7" with context_window 200000
    let mut models = HashMap::new();
    models.insert(
        "opus-4.7".to_string(),
        ModelDef {
            id: "claude-opus-4-7".to_string(),
            context_window: Some(200_000),
            max_output_tokens: Some(8192),
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );

    // @step And the Rhai script defines get_model_limits returning "#{ context_window: 400000 }"
    let script_body = r#"
fn get_model_limits(config) {
    #{ context_window: 400000 }
}
"#;

    let tmp = TempDir::new().expect("tempdir");
    let script_path = tmp.path().join("claude-rhai.rhai");
    let full_script = format!("{MANDATORY_LIFECYCLE_STUBS}\n{script_body}\n");
    std::fs::write(&script_path, &full_script).expect("write rhai script");

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
        defaults: None,
        system_prompt: None,
        tool_style: ToolStyle::Openai,
        api_style: ApiStyle::OpenaiChat,
        headers: HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };
    std::mem::forget(tmp);

    let loader = Arc::new(ScriptLoader::with_default_engine());

    // @step When lookup_script_model_limits is invoked for the selected model
    let provider = RhaiCustomProvider::new(Arc::new(cfg), loader, "opus-4.7".to_string())
        .expect("provider builds");

    // @step Then the resolved context_window equals 400000
    // (Rust tier — the provider.context_window() is the authoritative
    //  resolved value the NAPI session_set_model_profile chain reads.)
    assert_eq!(
        provider.context_window(),
        Some(400_000),
        "Rhai script get_model_limits must override JSON ModelDef (PROV-095)"
    );

    // @step And the resolved context_window is NOT 200000
    assert_ne!(provider.context_window(), Some(200_000));
}
