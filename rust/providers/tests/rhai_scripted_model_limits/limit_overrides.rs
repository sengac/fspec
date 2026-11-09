//! rhai_scripted_model_limits_tests.rs: context-window and max-output override scenarios.

use super::*;

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
    assert_eq!(provider.context_window(), Some(400_000));

    // @step And RhaiCustomProvider.max_output_tokens() returns 128000
    assert_eq!(provider.max_output_tokens(), Some(128_000));
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
    assert_eq!(provider.context_window(), Some(200_000));

    // @step And RhaiCustomProvider.max_output_tokens() returns 8192
    assert_eq!(provider.max_output_tokens(), Some(8192));

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
    assert_eq!(provider.context_window(), Some(400_000));

    // @step And RhaiCustomProvider.max_output_tokens() returns 4096
    assert_eq!(provider.max_output_tokens(), Some(4096));
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
    assert_eq!(provider.context_window(), Some(128_000));

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
            context_window: Some(128_000),
            max_output_tokens: Some(4096),
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
            context_window: Some(200_000),
            max_output_tokens: Some(8192),
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
    assert_eq!(p47.context_window(), Some(400_000));

    // @step When a RhaiCustomProvider is constructed for model alias "opus-4.6"
    let p46 = build_provider("claude-rhai", script, models, "opus-4.6").expect("construct 4.6");

    // @step Then RhaiCustomProvider.context_window() returns 200000
    assert_eq!(p46.context_window(), Some(200_000));
}
