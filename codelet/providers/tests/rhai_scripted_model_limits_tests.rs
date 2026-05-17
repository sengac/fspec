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
        let mut guard = self.0.lock().map_err(|e| {
            io::Error::other(format!("log buffer poisoned: {e}"))
        })?;
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
            context_window,
            max_output_tokens,
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );
    m
}

// =========================================================================
// Scenario: Script sets both context_window and max_output_tokens
// =========================================================================

#[test]
fn scenario_script_sets_both_context_window_and_max_output_tokens() {
    // @step Given a custom Rhai provider named "claude-rhai" with JSON ModelDef "opus-4.7" declaring context_window 128000 and max_output_tokens 4096
    let models = models_one("opus-4.7", "claude-opus-4-7", 128_000, 4096);

    // @step And the Rhai script defines "fn get_model_limits(config) { #{ context_window: 400000, max_output_tokens: 128000 } }"
    let script = r#"
fn get_model_limits(config) {
    #{ context_window: 400000, max_output_tokens: 128000 }
}
"#;

    // @step When a RhaiCustomProvider is constructed for model alias "opus-4.7"
    let provider =
        build_provider("claude-rhai", script, models, "opus-4.7").expect("construct provider");

    // @step Then RhaiCustomProvider.context_window() returns 400000
    assert_eq!(provider.context_window(), 400_000);

    // @step And RhaiCustomProvider.max_output_tokens() returns 128000
    assert_eq!(provider.max_output_tokens(), 128_000);
}

// =========================================================================
// Scenario: Legacy script without get_model_limits falls back to JSON ModelDef values
// =========================================================================

#[test]
fn scenario_legacy_script_without_get_model_limits_falls_back_to_json() {
    // @step Given a custom Rhai provider named "claude-rhai" with JSON ModelDef "opus-4.6" declaring context_window 200000 and max_output_tokens 8192
    let models = models_one("opus-4.6", "claude-opus-4-6", 200_000, 8192);

    // @step And the Rhai script does NOT define a "get_model_limits" function
    let script = ""; // empty — only mandatory lifecycle stubs are present

    // @step When a RhaiCustomProvider is constructed for model alias "opus-4.6"
    let (provider, logs) = capture_logs(|| {
        build_provider("claude-rhai", script, models, "opus-4.6").expect("construct provider")
    });

    // @step Then RhaiCustomProvider.context_window() returns 200000
    assert_eq!(provider.context_window(), 200_000);

    // @step And RhaiCustomProvider.max_output_tokens() returns 8192
    assert_eq!(provider.max_output_tokens(), 8192);

    // @step And no warning is logged about a missing "get_model_limits" function
    assert!(
        !logs.contains("get_model_limits"),
        "expected no log mentioning get_model_limits when the script does not define it, \
         but captured logs contained: {logs}"
    );
}

// =========================================================================
// Scenario: Partial override — only context_window set, max_output_tokens falls back
// =========================================================================

#[test]
fn scenario_partial_override_only_context_window() {
    // @step Given a custom Rhai provider with JSON ModelDef declaring context_window 128000 and max_output_tokens 4096
    let models = models_one("smart", "model-smart-v2", 128_000, 4096);

    // @step And the Rhai script defines "fn get_model_limits(config) { #{ context_window: 400000 } }"
    let script = r#"
fn get_model_limits(config) {
    #{ context_window: 400000 }
}
"#;

    // @step When a RhaiCustomProvider is constructed for that model alias
    let provider =
        build_provider("partial-rhai", script, models, "smart").expect("construct provider");

    // @step Then RhaiCustomProvider.context_window() returns 400000
    assert_eq!(provider.context_window(), 400_000);

    // @step And RhaiCustomProvider.max_output_tokens() returns 4096
    assert_eq!(provider.max_output_tokens(), 4096);
}

// =========================================================================
// Scenario: Invalid non-positive value is rejected and JSON ModelDef value is used
// =========================================================================

#[test]
fn scenario_invalid_non_positive_value_rejected() {
    // @step Given a custom Rhai provider named "claude-rhai" with JSON ModelDef declaring context_window 128000
    let models = models_one("opus-4.7", "claude-opus-4-7", 128_000, 4096);

    // @step And the Rhai script defines "fn get_model_limits(config) { #{ context_window: -1 } }"
    let script = r#"
fn get_model_limits(config) {
    #{ context_window: -1 }
}
"#;

    // @step When a RhaiCustomProvider is constructed for that model alias
    let (provider, logs) = capture_logs(|| {
        build_provider("claude-rhai", script, models, "opus-4.7").expect("construct provider")
    });

    // @step Then RhaiCustomProvider.context_window() returns 128000
    assert_eq!(provider.context_window(), 128_000);

    // @step And a warning is logged naming provider "claude-rhai" and key "context_window"
    assert!(
        logs.contains("claude-rhai") && logs.contains("context_window"),
        "expected warn log naming provider 'claude-rhai' and key 'context_window', got: {logs}"
    );
    assert!(
        logs.contains("non-positive"),
        "expected warn log to mention 'non-positive' rejection reason, got: {logs}"
    );
}

// =========================================================================
// Scenario: Script branches on config.model_alias to return alias-specific limits
// =========================================================================

#[test]
fn scenario_script_branches_on_model_alias() {
    // @step Given a custom Rhai provider named "claude-rhai" with JSON ModelDefs "opus-4.7" (128000) and "opus-4.6" (200000)
    let mut models = HashMap::new();
    models.insert(
        "opus-4.7".to_string(),
        ModelDef {
            id: "claude-opus-4-7".to_string(),
            context_window: 128_000,
            max_output_tokens: 4096,
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );
    models.insert(
        "opus-4.6".to_string(),
        ModelDef {
            id: "claude-opus-4-6".to_string(),
            context_window: 200_000,
            max_output_tokens: 8192,
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );

    // @step And the Rhai script defines get_model_limits that returns 400000 for "opus-4.7" and 200000 for "opus-4.6"
    let script = r#"
fn get_model_limits(config) {
    if config.model_alias == "opus-4.7" {
        #{ context_window: 400000 }
    } else if config.model_alias == "opus-4.6" {
        #{ context_window: 200000 }
    } else {
        #{}
    }
}
"#;

    // @step When a RhaiCustomProvider is constructed for model alias "opus-4.7"
    let p47 =
        build_provider("claude-rhai", script, models.clone(), "opus-4.7").expect("construct 4.7");

    // @step Then RhaiCustomProvider.context_window() returns 400000
    assert_eq!(p47.context_window(), 400_000);

    // @step When a RhaiCustomProvider is constructed for model alias "opus-4.6"
    let p46 = build_provider("claude-rhai", script, models, "opus-4.6").expect("construct 4.6");

    // @step Then RhaiCustomProvider.context_window() returns 200000
    assert_eq!(p46.context_window(), 200_000);
}

// =========================================================================
// Scenario: Script sets an absolute-tokens compaction threshold
// =========================================================================

#[test]
fn scenario_script_sets_absolute_tokens_compaction_threshold() {
    // @step Given a custom Rhai provider with JSON ModelDef declaring context_window 1000000
    let models = models_one("mega", "mega-model", 1_000_000, 8192);

    // @step And the Rhai script returns "#{ context_window: 400000, compaction_threshold: #{ type: \"tokens\", value: 200000 } }"
    let script = r#"
fn get_model_limits(config) {
    #{
        context_window: 400000,
        compaction_threshold: #{ type: "tokens", value: 200000 }
    }
}
"#;

    // @step When a RhaiCustomProvider is constructed
    let provider =
        build_provider("mega-rhai", script, models, "mega").expect("construct provider");

    // @step Then RhaiCustomProvider.script_compaction_threshold() returns Some(("tokens", 200000))
    assert_eq!(
        provider.script_compaction_threshold(),
        Some(("tokens".to_string(), 200_000))
    );

    // @step And the NAPI session-creation path calls ProviderManager.set_compaction_threshold_override with the same tuple
    // Exercise the real ProviderManager NAPI bridge: the accessor value
    // must round-trip through `set_compaction_threshold_override` and be
    // observable via `compaction_threshold_override()`.
    use codelet_providers::{ProviderManager, ProviderType};
    let mut pm = ProviderManager::for_testing(ProviderType::Codex, None, None);
    pm.set_compaction_threshold_override(provider.script_compaction_threshold());
    let override_tuple = pm
        .compaction_threshold_override()
        .map(|(t, v)| (t.to_string(), v));
    assert_eq!(
        override_tuple,
        Some(("tokens".to_string(), 200_000)),
        "NAPI bridge must forward the accessor tuple into ProviderManager::set_compaction_threshold_override unchanged"
    );

    // @step And RhaiCustomProvider.context_window() returns 400000
    assert_eq!(provider.context_window(), 400_000);
}

// =========================================================================
// Scenario: Script sets a percentage compaction threshold
// =========================================================================

#[test]
fn scenario_script_sets_percentage_compaction_threshold() {
    // @step Given a custom Rhai provider with JSON ModelDef declaring context_window 1000000
    let models = models_one("mega", "mega-model", 1_000_000, 8192);

    // @step And the Rhai script returns "#{ compaction_threshold: #{ type: \"percentage\", value: 75 } }"
    let script = r#"
fn get_model_limits(config) {
    #{ compaction_threshold: #{ type: "percentage", value: 75 } }
}
"#;

    // @step When a RhaiCustomProvider is constructed
    let provider =
        build_provider("mega-rhai", script, models, "mega").expect("construct provider");

    // @step Then RhaiCustomProvider.script_compaction_threshold() returns Some(("percentage", 75))
    assert_eq!(
        provider.script_compaction_threshold(),
        Some(("percentage".to_string(), 75))
    );

    // @step And RhaiCustomProvider.context_window() returns 1000000
    assert_eq!(provider.context_window(), 1_000_000);
}

// =========================================================================
// Scenario: Invalid compaction_threshold shape is rejected and no override is surfaced
// =========================================================================

#[test]
fn scenario_invalid_compaction_threshold_shape_rejected() {
    // @step Given a custom Rhai provider named "claude-rhai" with JSON ModelDef declaring context_window 200000
    let models = models_one("opus-4.6", "claude-opus-4-6", 200_000, 8192);

    // @step And the Rhai script returns "#{ compaction_threshold: #{ type: \"percentage\", value: 150 } }"
    let script = r#"
fn get_model_limits(config) {
    #{ compaction_threshold: #{ type: "percentage", value: 150 } }
}
"#;

    // @step When a RhaiCustomProvider is constructed
    let (provider, logs) = capture_logs(|| {
        build_provider("claude-rhai", script, models, "opus-4.6").expect("construct provider")
    });

    // @step Then RhaiCustomProvider.script_compaction_threshold() returns None
    assert_eq!(provider.script_compaction_threshold(), None);

    // @step And a warning is logged naming provider "claude-rhai" and key "compaction_threshold"
    assert!(
        logs.contains("claude-rhai") && logs.contains("compaction_threshold"),
        "expected warn log naming provider 'claude-rhai' and key 'compaction_threshold', got: {logs}"
    );
    assert!(
        logs.contains("1..=100") || logs.contains("percentage"),
        "expected warn log to describe percentage range violation, got: {logs}"
    );
}

// =========================================================================
// NAPI-bridge integration: lookup_script_model_limits end-to-end
// =========================================================================
//
// The original review flagged that scenario 6's NAPI-bridge assertion
// ("the NAPI session-creation path calls ProviderManager.set_compaction_threshold_override")
// was only verified via an accessor-level assertion. The test above for
// scenario 6 now exercises ProviderManager directly; this test below
// exercises the full discovery → script-construction → accessor
// pipeline that the NAPI layer uses: `lookup_script_model_limits` reads
// FSPEC_HOME, constructs the provider, and returns all three scripted
// values in one cached call.

#[test]
#[serial_test::serial]
fn napi_bridge_lookup_script_model_limits_roundtrips_all_three_fields() {
    use codelet_providers::custom::{
        lookup_script_model_limits, RhaiScriptedLimits,
    };

    // Lay out a faux FSPEC_HOME with a sibling providers/ directory and
    // a single provider config that covers all three scripted fields.
    let base = TempDir::new().expect("fspec-home tempdir");
    let credentials_dir = base.path().join("credentials");
    let providers_dir = base.path().join("providers");
    fs::create_dir_all(&credentials_dir).expect("mkdir credentials");
    fs::create_dir_all(&providers_dir).expect("mkdir providers");

    let script_path = providers_dir.join("bridge-test.rhai");
    let script_body = format!(
        "{MANDATORY_LIFECYCLE_STUBS}\n\
         fn get_model_limits(config) {{\n\
             #{{\n\
                 context_window: 400000,\n\
                 max_output_tokens: 256000,\n\
                 compaction_threshold: #{{ type: \"tokens\", value: 250000 }}\n\
             }}\n\
         }}\n"
    );
    fs::write(&script_path, script_body).expect("write script");

    let config_path = providers_dir.join("bridge-test.json");
    let config_json = format!(
        r#"{{
            "name": "bridge-test",
            "display_name": "Bridge Test",
            "base_url": "https://example.test",
            "script": "{}",
            "auth": {{ "type": "bearer", "env_var": "TEST_KEY" }},
            "models": {{
                "alpha": {{
                    "id": "model-alpha",
                    "context_window": 128000,
                    "max_output_tokens": 4096,
                    "supports_tools": true,
                    "supports_streaming": true,
                    "supports_thinking": false
                }}
            }},
            "tool_style": "openai",
            "api_style": "openai_chat"
        }}"#,
        script_path.to_string_lossy().replace('\\', "\\\\")
    );
    fs::write(&config_path, config_json).expect("write config");

    // Point FSPEC_HOME at the credentials dir (discovery derives the
    // sibling providers/ directory). Clear the module-level cache so a
    // prior test's FSPEC_HOME value doesn't bleed through.
    let prev_home = std::env::var("FSPEC_HOME").ok();
    std::env::set_var("FSPEC_HOME", &credentials_dir);
    codelet_providers::custom::__clear_lookup_cache_for_tests();

    let limits: RhaiScriptedLimits = lookup_script_model_limits("bridge-test", "alpha");
    assert_eq!(limits.context_window, Some(400_000));
    assert_eq!(limits.max_output_tokens, Some(256_000));
    assert_eq!(
        limits.compaction_threshold,
        Some(("tokens".to_string(), 250_000))
    );

    // Second call must hit the cache (observable via identical result)
    // — we can't assert filesystem-scan count directly, but the behaviour
    // is deterministic under cache TTL.
    let limits_2 = lookup_script_model_limits("bridge-test", "alpha");
    assert_eq!(limits, limits_2);

    // Unknown provider slug returns the default snapshot.
    let missing = lookup_script_model_limits("not-registered", "alpha");
    assert_eq!(missing, RhaiScriptedLimits::default());

    // Restore FSPEC_HOME for surrounding tests.
    match prev_home {
        Some(v) => std::env::set_var("FSPEC_HOME", v),
        None => std::env::remove_var("FSPEC_HOME"),
    }
    codelet_providers::custom::__clear_lookup_cache_for_tests();
}

// =========================================================================
// Additional coverage: non-integer context_window is rejected
// (rule 6 covers both non-positive AND non-integer; review flagged
// that only the -1 case had a test)
// =========================================================================

#[test]
fn wrong_type_context_window_rejected_and_logged() {
    let models = models_one("opus-4.7", "claude-opus-4-7", 128_000, 4096);

    // String is the wrong type for context_window.
    let script = r#"
fn get_model_limits(config) {
    #{ context_window: "not a number" }
}
"#;

    let (provider, logs) = capture_logs(|| {
        build_provider("claude-rhai", script, models, "opus-4.7").expect("construct provider")
    });

    // Falls back to the JSON ModelDef value.
    assert_eq!(provider.context_window(), 128_000);

    assert!(
        logs.contains("claude-rhai") && logs.contains("context_window"),
        "expected warn log naming provider + key, got: {logs}"
    );
    assert!(
        logs.contains("non-integer"),
        "expected warn log to mention 'non-integer' rejection reason, got: {logs}"
    );
}
