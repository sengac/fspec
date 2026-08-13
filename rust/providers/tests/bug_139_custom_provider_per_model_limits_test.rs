#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-napi-model-limits.feature
//!
//! BUG-139 red-phase regression tests at the Rust layer.
//!
//! Two scenarios covered here:
//!
//! 1. `NAPI listProviders returns per-model limits for custom providers`
//!    — `list_providers_info()` must return a `Vec<ProviderInfo>` whose
//!    `models` field carries per-alias limits (context_window,
//!    max_output_tokens, supports_tools/streaming/thinking), not just
//!    the alias string.
//!
//! 2. `default_context_window() default value changes to 200000`
//!    — The `#[serde(default)]` fallback on `ModelDef.context_window`
//!    must be 200_000 (was 128_000).
//!
//! These tests WILL NOT COMPILE against the current tree — the widening
//! of `ProviderInfo.models` from `Vec<String>` to the new
//! `Vec<ProviderModelInfo>` shape is the very change under test.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use serial_test::serial;
use tempfile::TempDir;

use codelet_providers::custom::{
    list_providers_info, ApiStyle, AuthConfig, ModelDef, ProviderConfig, ProviderModelInfo,
    RhaiCustomProvider, ScriptLoader, ToolStyle,
};
use codelet_providers::LlmProvider;
use std::collections::HashMap;
use std::sync::Arc;

// -------------------------------------------------------------------------
// RAII env-var & cwd guards (mirroring
// custom_provider_manager_integration_test.rs so discovery never reaches
// the real `$HOME/.fspec/providers/` at test time).
// -------------------------------------------------------------------------

struct EnvGuard {
    key: String,
    prior: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: &str) -> Self {
        let prior = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self {
            key: key.to_string(),
            prior,
        }
    }

    fn set_path(key: &str, value: &Path) -> Self {
        let prior = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self {
            key: key.to_string(),
            prior,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.prior.take() {
            Some(v) => std::env::set_var(&self.key, v),
            None => std::env::remove_var(&self.key),
        }
    }
}

struct CwdGuard {
    prior: PathBuf,
}

impl CwdGuard {
    fn set(path: &Path) -> Self {
        let prior = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(path).expect("set cwd");
        Self { prior }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior);
    }
}

struct DiscoveryFixture {
    _home_tmp: TempDir,
    project_tmp: TempDir,
    _home_guard: EnvGuard,
    _fspec_guard: EnvGuard,
    _cwd_guard: CwdGuard,
}

impl DiscoveryFixture {
    fn new() -> Self {
        let home_tmp = TempDir::new().expect("home tempdir");
        let project_tmp = TempDir::new().expect("project tempdir");
        let fspec_dir = home_tmp.path().join(".fspec");
        let credentials_dir = fspec_dir.join("credentials");
        fs::create_dir_all(&credentials_dir).unwrap();
        let home_guard = EnvGuard::set_path("HOME", home_tmp.path());
        let fspec_guard = EnvGuard::set_path("FSPEC_HOME", &credentials_dir);
        let cwd_guard = CwdGuard::set(project_tmp.path());
        Self {
            _home_tmp: home_tmp,
            project_tmp,
            _home_guard: home_guard,
            _fspec_guard: fspec_guard,
            _cwd_guard: cwd_guard,
        }
    }

    fn project_root(&self) -> &Path {
        self.project_tmp.path()
    }
}

/// Write a project-local `claude-rhai`-style config with a single
/// `opus-4.7` model alias carrying the requested per-model limits.
fn write_claude_rhai_config(
    project_root: &Path,
    name: &str,
    context_window: Option<usize>,
    max_output_tokens: Option<usize>,
    supports_tools: Option<bool>,
    supports_streaming: Option<bool>,
    supports_thinking: Option<bool>,
) {
    let providers_dir = project_root.join(".fspec").join("providers");
    fs::create_dir_all(&providers_dir).unwrap();

    let mut model_obj = serde_json::Map::new();
    model_obj.insert("id".into(), json!("claude-opus-4-7"));
    if let Some(cw) = context_window {
        model_obj.insert("context_window".into(), json!(cw));
    }
    if let Some(mo) = max_output_tokens {
        model_obj.insert("max_output_tokens".into(), json!(mo));
    }
    if let Some(b) = supports_tools {
        model_obj.insert("supports_tools".into(), json!(b));
    }
    if let Some(b) = supports_streaming {
        model_obj.insert("supports_streaming".into(), json!(b));
    }
    if let Some(b) = supports_thinking {
        model_obj.insert("supports_thinking".into(), json!(b));
    }

    let cfg = json!({
        "name": name,
        "display_name": format!("Claude Rhai Test {name}"),
        "facade": "claude",
        "base_url": "https://api.anthropic.com",
        "api_key_env_var": "ANTHROPIC_API_KEY",
        "models": { "opus-4.7": serde_json::Value::Object(model_obj) }
    });

    fs::write(
        providers_dir.join(format!("{name}.json")),
        serde_json::to_string_pretty(&cfg).unwrap(),
    )
    .unwrap();
}

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
        model.context_window, 1_000_000,
        "context_window must round-trip from JSON ModelDef, not be replaced by 128k default"
    );

    // @step And that item has maxOutput 128000
    assert_eq!(model.max_output_tokens, 128_000);

    // @step And that item has supportsTools true
    assert!(model.supports_tools);

    // @step And that item has supportsStreaming true
    assert!(model.supports_streaming);

    // @step And that item has supportsThinking true
    assert!(model.supports_thinking);
}

// =========================================================================
// Scenario: default_context_window() default value changes to 200000
// =========================================================================
#[test]
fn default_context_window_is_200000_when_json_omits_context_window() {
    // @step Given a ProviderConfig JSON that omits context_window on every model
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

    // @step Then model.context_window equals 200000
    for (alias, def) in &cfg.models {
        assert_eq!(
            def.context_window, 200_000,
            "model '{alias}' should default to 200000, got {}",
            def.context_window
        );
        // @step And it does NOT equal the previous default 128000
        assert_ne!(
            def.context_window, 128_000,
            "model '{alias}' still uses legacy 128k default"
        );
    }
}

// =========================================================================
// Scenario: JSON omits context_window - new default 200k flows through
// =========================================================================
#[test]
#[serial]
fn json_omits_context_window_list_providers_surfaces_200k_default() {
    // @step Given the claude-rhai provider config declares model "opus-4.7" without a context_window field
    // @step And the Rhai script does not define get_model_limits
    let fx = DiscoveryFixture::new();
    let _api_key = EnvGuard::set("ANTHROPIC_API_KEY", "sk-not-real");
    write_claude_rhai_config(
        fx.project_root(),
        "claude-rhai",
        None, // omit context_window — rely on default
        None,
        None,
        None,
        None,
    );

    // @step When list_providers_info() resolves per-model limits
    // (Rust tier — list_providers_info() is what the selector queries.)
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

    // @step Then the returned model entry's contextWindow equals 200000
    assert_eq!(model.context_window, 200_000);

    // @step And the contextWindow is NOT 128000
    assert_ne!(model.context_window, 128_000);
    // @step And the contextWindow is NOT 120000
    assert_ne!(model.context_window, 120_000);
}

// =========================================================================
// Scenario: Rhai script get_model_limits still wins over JSON (PROV-095 no-regression)
// =========================================================================

/// Mandatory 7 Rhai lifecycle stubs the custom provider constructor
/// validates (copied from `rhai_scripted_model_limits_tests.rs`).
const MANDATORY_LIFECYCLE_STUBS: &str = r#"
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
            context_window: 200_000,
            max_output_tokens: 8192,
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
        400_000,
        "Rhai script get_model_limits must override JSON ModelDef (PROV-095)"
    );

    // @step And the resolved context_window is NOT 200000
    assert_ne!(provider.context_window(), 200_000);
}
