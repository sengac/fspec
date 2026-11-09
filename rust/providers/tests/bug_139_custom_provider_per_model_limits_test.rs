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

// Test functions live in submodules (REFAC-030) so every file stays under 300 lines.
#[path = "bug_139_per_model_limits/scenarios.rs"]
mod scenarios;

// PROV-170: unavailable-limit scenarios for RhaiCustomProvider and config validation.
#[path = "bug_139_per_model_limits/unavailable_limits.rs"]
mod unavailable_limits;

// PROV-172: a declared limit too large for the u32 wire fails the listing.
#[path = "bug_139_per_model_limits/wire_limits.rs"]
mod wire_limits;
