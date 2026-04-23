#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
//! Shared helpers for PROV-063 custom-provider HTTP lifecycle tests.
//!
//! Included via `#[path = "custom_http_test_helpers.rs"] mod helpers;`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use codelet_providers::custom::{AuthConfig, ModelDef, ProviderConfig};
use tempfile::TempDir;

/// Write a `.rhai` script body to `dir/filename` and return the path.
pub fn write_script(dir: &Path, filename: &str, body: &str) -> PathBuf {
    let path = dir.join(filename);
    fs::write(&path, body).expect("write script");
    path
}

/// Build a minimal `ProviderConfig` pointing at a freshly-written script on
/// disk. The returned `TempDir` must be retained by the caller for the
/// lifetime of the test.
pub fn config_with_script(name: &str, script_body: &str) -> (TempDir, ProviderConfig) {
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_script(tmp.path(), "p.rhai", script_body);

    let mut models: HashMap<String, ModelDef> = HashMap::new();
    models.insert(
        "smart".to_string(),
        ModelDef {
            id: "model-smart-v2".to_string(),
            context_window: 128_000,
            max_output_tokens: 4096,
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );

    let cfg = ProviderConfig {
        name: name.to_string(),
        display_name: "My LLM".to_string(),
        base_url: "https://api.example.com".to_string(),
        script: script_path.to_string_lossy().to_string(),
        facade: None,
        api_key_env_var: None,
        auth: AuthConfig::Bearer {
            env_var: "MY_KEY".to_string(),
            token_prefix: "Bearer".to_string(),
        },
        models,
        defaults: None,
        system_prompt: None,
        tool_style: codelet_providers::custom::ToolStyle::Openai,
        api_style: codelet_providers::custom::ApiStyle::OpenaiChat,
        headers: HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };
    (tmp, cfg)
}

/// Build a ProviderConfig plus a script containing the full set of 7
/// required functions. The script forwards the `model` alias to its body
/// via closure-captured string substitution so tests can easily adjust
/// individual functions.
pub fn config_with_full_script(
    name: &str,
    base_url: &str,
    model_id: &str,
    script_body: &str,
) -> (TempDir, ProviderConfig) {
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_script(tmp.path(), "p.rhai", script_body);

    let mut models: HashMap<String, ModelDef> = HashMap::new();
    models.insert(
        "smart".to_string(),
        ModelDef {
            id: model_id.to_string(),
            context_window: 128_000,
            max_output_tokens: 4096,
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );

    let cfg = ProviderConfig {
        name: name.to_string(),
        display_name: "My LLM".to_string(),
        base_url: base_url.to_string(),
        script: script_path.to_string_lossy().to_string(),
        facade: None,
        api_key_env_var: None,
        auth: AuthConfig::Bearer {
            env_var: "MY_KEY".to_string(),
            token_prefix: "Bearer".to_string(),
        },
        models,
        defaults: None,
        system_prompt: None,
        tool_style: codelet_providers::custom::ToolStyle::Openai,
        api_style: codelet_providers::custom::ApiStyle::OpenaiChat,
        headers: HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };
    (tmp, cfg)
}

/// Full, "happy-path" Rhai script that implements all 7 required functions
/// in a minimally-useful way. Individual tests that need specific
/// behaviour should substitute their own function bodies in-line.
pub const FULL_HAPPY_SCRIPT: &str = r#"
fn build_request(request) {
    #{ messages: request.messages }
}

fn build_headers(config) {
    #{
        "Authorization": "Bearer sk-xxx",
        "Content-Type": "application/json"
    }
}

fn build_url(config) {
    config.base_url + "/v1/chat/completions"
}

fn parse_response(raw) {
    #{
        content: raw.choices[0].message.content,
        stop_reason: "end_turn"
    }
}

fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }

fn map_error(status, body) {
    if status == 401 {
        #{ type: "auth", message: "unauthorized" }
    } else if status == 429 {
        #{ type: "rate_limit", message: "slow down" }
    } else {
        #{ type: "api", message: "server error" }
    }
}
"#;
