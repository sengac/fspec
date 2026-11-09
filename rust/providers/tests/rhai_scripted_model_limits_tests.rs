#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/rhai-scripted-model-limits.feature
//!
//! PROV-095 — the `RhaiCustomProvider` must honor an optional
//! `get_model_limits(config) -> Map` entry point in the user's Rhai
//! script, overriding the JSON `ModelDef` values for `context_window`,
//! `max_output_tokens`, and optionally surfacing a
//! `compaction_threshold` the NAPI session-creation path can wire into
//! [`codelet_providers::ProviderManager::set_compaction_threshold_override`].
//!
//! Every Gherkin scenario is exercised end-to-end including the
//! log-capture assertions (scenarios 2/4/8) and the NAPI bridge
//! assertion (scenario 6) — we construct a real
//! [`codelet_providers::ProviderManager`] via
//! [`codelet_providers::ProviderManager::for_testing`] and verify the
//! scripted threshold round-trips through
//! `set_compaction_threshold_override` / `compaction_threshold_override`.
//!
//! Scenarios are mapped 1:1 to `spec/features/rhai-scripted-model-limits.feature`
//! with full `@step` annotations on every Gherkin step.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::sync::{Arc, Mutex};

use codelet_providers::custom::{
    ApiStyle, AuthConfig, ModelDef, ProviderConfig, RhaiCustomProvider, ScriptLoader, ToolStyle,
};
use codelet_providers::LlmProvider;
use tempfile::TempDir;
use tracing_subscriber::fmt::MakeWriter;

// -------------------------------------------------------------------------
// Log capture — tracing-subscriber writer used by the log-assertion
// scenarios (2 / 4 / 8). Runs the guarded block with a thread-local
// dispatcher so parallel tests don't interfere.
// -------------------------------------------------------------------------

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl LogBuffer {
    fn contents(&self) -> String {
        let guard = self.0.lock().expect("log buffer lock");
        String::from_utf8_lossy(&guard).into_owned()
    }
}

impl io::Write for LogBuffer {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let mut guard = self
            .0
            .lock()
            .map_err(|e| io::Error::other(format!("log buffer poisoned: {e}")))?;
        guard.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogBuffer {
    type Writer = LogBuffer;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Run `f` with a thread-local tracing subscriber that captures every
/// emitted event into the returned string. The subscriber is torn down
/// on drop so parallel tests can set up their own without interference.
fn capture_logs<T>(f: impl FnOnce() -> T) -> (T, String) {
    let buffer = LogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buffer.clone())
        .with_max_level(tracing::Level::TRACE)
        .without_time()
        .with_ansi(false)
        .finish();
    let value = tracing::subscriber::with_default(subscriber, f);
    (value, buffer.contents())
}

// -------------------------------------------------------------------------
// Shared helpers
// -------------------------------------------------------------------------

/// Every `ProviderConfig` validated by `ScriptLoader` requires the 7
/// lifecycle functions. These stubs satisfy that requirement without
/// pulling in any network or tool plumbing; the PROV-095 tests only
/// care about the NEW optional `get_model_limits` hook.
const MANDATORY_LIFECYCLE_STUBS: &str = r#"
fn build_url(config)             { "" }
fn build_headers(config)         { #{} }
fn build_request(request)        { #{} }
fn build_stream_request(request) { #{} }
fn parse_response(raw)           { #{ content: [], stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk)     { #{} }
fn map_error(status, body)       { #{} }
"#;

/// Build an inline `RhaiCustomProvider` from an extra script body that is
/// appended AFTER the 7 mandatory lifecycle stubs. `models` is the full
/// `ModelDef` map seeded into the config; `model_alias` is the key to
/// construct the provider for.
fn build_provider(
    provider_name: &str,
    extra_script_body: &str,
    models: HashMap<String, ModelDef>,
    model_alias: &str,
) -> Result<RhaiCustomProvider, String> {
    let tmp = TempDir::new().expect("tempdir");
    let script_path = tmp.path().join(format!("{provider_name}.rhai"));
    let full_script = format!("{MANDATORY_LIFECYCLE_STUBS}\n{extra_script_body}\n");
    fs::write(&script_path, &full_script).expect("write rhai script");

    let cfg = ProviderConfig {
        name: provider_name.to_string(),
        display_name: provider_name.to_string(),
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

    // The script file must outlive the test — intentionally leak the
    // TempDir (scripts reference paths, not handles).
    std::mem::forget(tmp);

    let loader = Arc::new(ScriptLoader::with_default_engine());
    RhaiCustomProvider::new(Arc::new(cfg), loader, model_alias.to_string())
        .map_err(|e| format!("RhaiCustomProvider::new failed: {e:?}"))
}

/// Convenience: a single-model `ModelDef` map.
fn models_one(
    alias: &str,
    model_id: &str,
    context_window: usize,
    max_output_tokens: usize,
) -> HashMap<String, ModelDef> {
    let mut m = HashMap::new();
    m.insert(
        alias.to_string(),
        ModelDef {
            id: model_id.to_string(),
            context_window: Some(context_window),
            max_output_tokens: Some(max_output_tokens),
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );
    m
}

// Test functions live in submodules (REFAC-030) so every file stays under 300 lines.
#[path = "rhai_scripted_model_limits/limit_overrides.rs"]
mod limit_overrides;
#[path = "rhai_scripted_model_limits/thresholds_and_bridge.rs"]
mod thresholds_and_bridge;
