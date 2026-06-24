#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/rhai-thinking-config-in-build-request.feature
//!
//! This test file validates the acceptance criteria for PROV-090: the
//! custom-provider request bridge must accept a `thinking_config`
//! parameter and expose it to Rhai scripts under
//! `request.thinking_config`. It also exercises the wiring through
//! `RhaiCustomProvider::invoke_build_request` /
//! `invoke_build_stream_request` and the `CustomProvider::create_rig_agent`
//! signature parity with `ClaudeProvider::create_rig_agent`.
//!
//! These tests call `request_to_rhai` with an extra
//! `Option<&serde_json::Value>` argument and `invoke_build_request` /
//! `invoke_build_stream_request` with a `Option<serde_json::Value>`
//! tail argument. They therefore fail to compile until PROV-090 lands —
//! this is the red phase.

#[path = "custom_http_test_helpers.rs"]
mod helpers;

use std::sync::Arc;

use codelet_common::Message;
use codelet_providers::custom::request_bridge::request_to_rhai;
use codelet_providers::custom::{CustomProvider, ProviderConfig, RhaiCustomProvider, ScriptLoader};
use helpers::{config_with_full_script, config_with_script};
use rhai::{Dynamic, Map};
use uuid::Uuid;

/// Build a fresh `RhaiCustomProvider` from a config + inline script using a
/// newly-allocated `ScriptLoader`.
fn build_provider(cfg: ProviderConfig, model_alias: &str) -> RhaiCustomProvider {
    let loader = Arc::new(ScriptLoader::with_default_engine());
    RhaiCustomProvider::new(Arc::new(cfg), loader, model_alias.to_string())
        .expect("construct RhaiCustomProvider")
}

// =========================================================================
// Scenario: request_to_rhai bridges Some thinking_config into the request map
// =========================================================================
#[test]
fn request_to_rhai_bridges_some_thinking_config_into_the_request_map() {
    // @step Given a messages slice and a tools slice and a JSON value {"type":"enabled","budget_tokens":10000}
    let messages: Vec<Message> = vec![Message::user("hi")];
    let tools = Vec::new();
    let thinking = serde_json::json!({
        "type": "enabled",
        "budget_tokens": 10000i64,
    });

    // @step When I call request_to_rhai with Some(thinking_config)
    let dyn_map: Dynamic =
        request_to_rhai(&messages, &tools, Some(&thinking)).expect("request_to_rhai returns Ok");

    // @step Then the returned Dynamic is a map containing messages tools and thinking_config
    let outer: Map = dyn_map
        .try_cast::<Map>()
        .expect("outer Dynamic is a Rhai Map");
    assert!(outer.contains_key("messages"), "missing messages key");
    assert!(outer.contains_key("tools"), "missing tools key");
    assert!(
        outer.contains_key("thinking_config"),
        "missing thinking_config key"
    );

    // @step And the thinking_config entry is a map whose type field is "enabled" and whose budget_tokens field is 10000
    let tc = outer
        .get("thinking_config")
        .cloned()
        .expect("thinking_config present")
        .try_cast::<Map>()
        .expect("thinking_config is a Rhai Map");
    let ty = tc
        .get("type")
        .cloned()
        .expect("type field present")
        .into_string()
        .expect("type is a string");
    assert_eq!(ty, "enabled");
    let budget = tc
        .get("budget_tokens")
        .cloned()
        .expect("budget_tokens field present")
        .as_int()
        .expect("budget_tokens is an int");
    assert_eq!(budget, 10000);
}

// =========================================================================
// Scenario: request_to_rhai bridges None thinking_config as Rhai unit
// =========================================================================
#[test]
fn request_to_rhai_bridges_none_thinking_config_as_rhai_unit() {
    // @step Given a messages slice and a tools slice
    let messages: Vec<Message> = vec![Message::user("hi")];
    let tools = Vec::new();

    // @step When I call request_to_rhai with None for thinking_config
    let dyn_map: Dynamic =
        request_to_rhai(&messages, &tools, None).expect("request_to_rhai returns Ok");

    // @step Then the returned Dynamic is a map containing a thinking_config key whose value is Rhai unit
    let outer: Map = dyn_map
        .try_cast::<Map>()
        .expect("outer Dynamic is a Rhai Map");
    let tc = outer
        .get("thinking_config")
        .cloned()
        .expect("thinking_config key present even when None");
    assert!(
        tc.is_unit(),
        "expected thinking_config to be Rhai unit when None, got {:?}",
        tc.type_name()
    );
}

// =========================================================================
// Scenario: A Rhai build_request script uses thinking_config to populate
//           the outgoing request body
// =========================================================================
#[tokio::test]
async fn rhai_build_request_uses_thinking_config_to_populate_body() {
    // @step Given a RhaiCustomProvider whose build_request script copies request.thinking_config.budget_tokens into the body as thinking.budget_tokens when thinking_config is present
    let script = r#"
fn build_request(request) {
    if type_of(request.thinking_config) == "map" {
        #{
            messages: request.messages,
            thinking: #{ budget_tokens: request.thinking_config.budget_tokens }
        }
    } else {
        #{ messages: request.messages }
    }
}
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
"#;
    let (_tmp, cfg) = config_with_full_script(
        "my-llm",
        "https://api.example.com",
        "model-smart-v2",
        script,
    );
    let provider = build_provider(cfg, "smart");

    // @step When I invoke invoke_build_request with Some thinking_config {"type":"enabled","budget_tokens":8192}
    let messages = vec![Message::user("hi")];
    let thinking = serde_json::json!({
        "type": "enabled",
        "budget_tokens": 8192i64,
    });
    let body_json = provider
        .invoke_build_request(&messages, &[], Some(thinking))
        .await
        .expect("build_request returns JSON");

    // @step Then the resulting JSON body contains thinking.budget_tokens equal to 8192
    let budget = body_json
        .get("thinking")
        .and_then(|v| v.get("budget_tokens"))
        .and_then(serde_json::Value::as_i64)
        .expect("thinking.budget_tokens present");
    assert_eq!(budget, 8192);
}

// =========================================================================
// Scenario: CustomProvider::create_rig_agent accepts a thinking_config
//           parameter
// =========================================================================
#[tokio::test]
async fn custom_provider_create_rig_agent_accepts_thinking_config_parameter() {
    // @step Given a valid custom provider config discoverable on disk
    let script_body = r#"
fn build_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
"#;
    // Reuse config_with_script for tempdir hosting, but we also need to
    // write the <name>.json + sibling .rhai script under
    // `<project_root>/.fspec/providers/` so discovery picks it up.
    let (_tmp_unused, _cfg_unused) = config_with_script("prov-090-dummy", script_body);

    // Use a temp HOME so global discovery cannot accidentally satisfy
    // the lookup, then place the config in the project-local
    // `.fspec/providers/` directory. Discovery resolves project-local
    // relative to the *current working directory*, so we also redirect
    // CWD for the duration of this test.
    let home = tempfile::TempDir::new().expect("home tempdir");
    let prior_home = std::env::var_os("HOME");
    std::env::set_var("HOME", home.path());
    let prior_fspec = std::env::var_os("FSPEC_HOME");
    std::env::remove_var("FSPEC_HOME");

    let project = tempfile::TempDir::new().expect("project root");
    let prior_cwd = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(project.path()).expect("chdir project");
    struct Restore {
        home: Option<std::ffi::OsString>,
        fspec: Option<std::ffi::OsString>,
        cwd: std::path::PathBuf,
    }
    impl Drop for Restore {
        fn drop(&mut self) {
            match self.home.take() {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match self.fspec.take() {
                Some(v) => std::env::set_var("FSPEC_HOME", v),
                None => std::env::remove_var("FSPEC_HOME"),
            }
            let _ = std::env::set_current_dir(&self.cwd);
        }
    }
    let _restore = Restore {
        home: prior_home,
        fspec: prior_fspec,
        cwd: prior_cwd,
    };

    let providers_dir = project.path().join(".fspec").join("providers");
    std::fs::create_dir_all(&providers_dir).expect("mkdir -p .fspec/providers");
    let name = "prov090-custom";
    std::fs::write(providers_dir.join(format!("{name}.rhai")), script_body).expect("write rhai");
    let cfg_json = serde_json::json!({
        "name": name,
        "display_name": "PROV-090 Custom",
        "base_url": "https://api.example.com",
        "script": format!("{name}.rhai"),
        "facade": null,
        "auth": { "type": "bearer", "env_var": "PROV090_KEY", "token_prefix": "Bearer" },
        "models": {
            "smart": {
                "id": "model-smart-v2",
                "context_window": 128000,
                "max_output_tokens": 4096,
                "supports_tools": true,
                "supports_streaming": true,
                "supports_thinking": true,
            }
        },
        "tool_style": "openai",
        "api_style": "openai_chat",
        "headers": {},
    });
    std::fs::write(
        providers_dir.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&cfg_json).unwrap(),
    )
    .expect("write json");

    // @step When I call CustomProvider::create_rig_agent passing Some(thinking_config) as the new parameter
    let thinking = serde_json::json!({
        "type": "enabled",
        "budget_tokens": 4096i64,
    });
    let agent = CustomProvider::create_rig_agent(
        project.path(),
        name,
        "smart",
        Uuid::new_v4(),
        None,
        Some(thinking),
    )
    .expect("create_rig_agent returns Ok");

    // @step Then the call returns Ok(CustomRigAgent) with the same wiring invariants as before
    assert_eq!(agent.provider_name(), name);
    assert!(agent.uses_rhai_system_prompt_facade());
}
