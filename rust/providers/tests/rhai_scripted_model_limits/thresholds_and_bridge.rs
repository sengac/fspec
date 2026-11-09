//! rhai_scripted_model_limits_tests.rs: compaction-threshold scenarios, the NAPI bridge and wrong-type rejection.

use super::*;

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
    let provider = build_provider("mega-rhai", script, models, "mega").expect("construct provider");

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
    assert_eq!(provider.context_window(), Some(400_000));
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
    let provider = build_provider("mega-rhai", script, models, "mega").expect("construct provider");

    // @step Then RhaiCustomProvider.script_compaction_threshold() returns Some(("percentage", 75))
    assert_eq!(
        provider.script_compaction_threshold(),
        Some(("percentage".to_string(), 75))
    );

    // @step And RhaiCustomProvider.context_window() returns 1000000
    assert_eq!(provider.context_window(), Some(1_000_000));
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
    use codelet_providers::custom::{lookup_script_model_limits, RhaiScriptedLimits};

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
    assert_eq!(provider.context_window(), Some(128_000));

    assert!(
        logs.contains("claude-rhai") && logs.contains("context_window"),
        "expected warn log naming provider + key, got: {logs}"
    );
    assert!(
        logs.contains("non-integer"),
        "expected warn log to mention 'non-integer' rejection reason, got: {logs}"
    );
}
