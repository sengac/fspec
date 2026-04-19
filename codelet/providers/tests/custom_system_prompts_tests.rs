#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-rhai-scriptable-system-prompts.feature
//!
//! Integration tests for PROV-065: RhaiSystemPromptFacade that implements
//! the SystemPromptFacade trait for custom providers and delegates to
//! optional Rhai script functions (identity_prefix, transform_preamble,
//! format_system_prompt) with safe fallbacks.
//!
//! These tests exercise `codelet_providers::custom::system_prompt` which
//! does not yet exist — they must fail to compile in the red phase.

use std::sync::Arc;

#[path = "custom_system_prompts_test_helpers.rs"]
mod helpers;

use helpers::{compile_script, config_dynamic, test_engine};

use codelet_providers::custom::system_prompt::RhaiSystemPromptFacade;
use codelet_tools::facade::{prepend_fspec_guidance, SystemPromptFacade};
use codelet_tools::FSPEC_WORKFLOW_GUIDANCE;

// =========================================================================
// Scenario: Identity prefix from Rhai function
// =========================================================================
#[test]
fn identity_prefix_from_rhai_function() {
    // @step Given a Rhai script that defines identity_prefix returning "You are MyBot"
    let engine = test_engine();
    let script = r#"
        fn identity_prefix(config) { "You are MyBot" }
    "#;
    let ast = compile_script(&engine, script);

    // @step When I build a RhaiSystemPromptFacade from that script
    let facade = RhaiSystemPromptFacade::new(
        "my-llm".to_string(),
        Arc::clone(&engine),
        ast,
        config_dynamic("my-llm"),
    );

    // @step Then facade.identity_prefix() returns Some("You are MyBot")
    let prefix = facade.identity_prefix();
    assert_eq!(
        prefix,
        Some("You are MyBot"),
        "identity_prefix should return Some(\"You are MyBot\"), got {prefix:?}"
    );
}

// =========================================================================
// Scenario: Identity prefix defaults to None
// =========================================================================
#[test]
fn identity_prefix_defaults_to_none() {
    // @step Given a Rhai script that does not define identity_prefix
    let engine = test_engine();
    let script = r#"
        fn other_fn() { 42 }
    "#;
    let ast = compile_script(&engine, script);

    // @step When I build a RhaiSystemPromptFacade from that script
    let facade = RhaiSystemPromptFacade::new(
        "my-llm".to_string(),
        Arc::clone(&engine),
        ast,
        config_dynamic("my-llm"),
    );

    // @step Then facade.identity_prefix() returns None
    assert_eq!(
        facade.identity_prefix(),
        None,
        "identity_prefix should default to None when not defined"
    );
}

// =========================================================================
// Scenario: Default transform_preamble prepends fspec guidance
// =========================================================================
#[test]
fn default_transform_preamble_prepends_fspec_guidance() {
    // @step Given a Rhai script that does not define transform_preamble
    let engine = test_engine();
    let script = r#"
        fn unused_marker() { () }
    "#;
    let ast = compile_script(&engine, script);
    let facade = RhaiSystemPromptFacade::new(
        "my-llm".to_string(),
        Arc::clone(&engine),
        ast,
        config_dynamic("my-llm"),
    );

    // @step When I call facade.transform_preamble("user text")
    let result = facade.transform_preamble("user text");

    // @step Then the result equals FSPEC_WORKFLOW_GUIDANCE concatenated with
    //       two newlines and "user text"
    let expected = prepend_fspec_guidance("user text");
    assert_eq!(result, expected, "default transform_preamble should match prepend_fspec_guidance");
    assert!(
        result.starts_with(FSPEC_WORKFLOW_GUIDANCE),
        "result should start with FSPEC_WORKFLOW_GUIDANCE"
    );
    assert!(
        result.ends_with("user text"),
        "result should end with the user preamble"
    );
    assert!(
        result.contains("\n\nuser text"),
        "result should separate guidance from preamble with two newlines"
    );
}

// =========================================================================
// Scenario: Custom transform_preamble overrides default
// =========================================================================
#[test]
fn custom_transform_preamble_overrides_default() {
    // @step Given a Rhai script whose transform_preamble returns "PREFIX: " + preamble
    let engine = test_engine();
    let script = r#"
        fn transform_preamble(config, preamble, fspec_guidance) {
            "PREFIX: " + preamble
        }
    "#;
    let ast = compile_script(&engine, script);
    let facade = RhaiSystemPromptFacade::new(
        "my-llm".to_string(),
        Arc::clone(&engine),
        ast,
        config_dynamic("my-llm"),
    );

    // @step When I call facade.transform_preamble("body")
    let result = facade.transform_preamble("body");

    // @step Then the result equals "PREFIX: body"
    assert_eq!(result, "PREFIX: body");
}

// =========================================================================
// Scenario: Default format_for_api returns plain JSON string
// =========================================================================
#[test]
fn default_format_for_api_returns_plain_json_string() {
    // @step Given a Rhai script with no system prompt functions defined
    let engine = test_engine();
    let script = r#"
        fn unrelated() { 1 }
    "#;
    let ast = compile_script(&engine, script);
    let facade = RhaiSystemPromptFacade::new(
        "my-llm".to_string(),
        Arc::clone(&engine),
        ast,
        config_dynamic("my-llm"),
    );

    // @step When I call facade.format_for_api("body")
    let value = facade.format_for_api("body");

    // @step Then the result is a JSON String whose value starts with
    //       FSPEC_WORKFLOW_GUIDANCE and ends with "body"
    assert!(
        value.is_string(),
        "default format_for_api should produce a JSON String, got {value:?}"
    );
    let text = value.as_str().expect("string value");
    assert!(
        text.starts_with(FSPEC_WORKFLOW_GUIDANCE),
        "text should start with FSPEC_WORKFLOW_GUIDANCE"
    );
    assert!(text.ends_with("body"), "text should end with the preamble body");
}

// =========================================================================
// Scenario: format_system_prompt returning array produces JSON array with cache_control
// =========================================================================
#[test]
fn format_system_prompt_array_preserves_cache_control() {
    // @step Given a Rhai script whose format_system_prompt returns a map with
    //       format "array" and two blocks including cache_control ephemeral on
    //       the second
    let engine = test_engine();
    let script = r#"
        fn format_system_prompt(config, preamble, fspec_guidance) {
            #{
                format: "array",
                blocks: [
                    #{ type: "text", text: "prefix" },
                    #{
                        type: "text",
                        text: preamble,
                        cache_control: #{ type: "ephemeral" },
                    },
                ],
            }
        }
    "#;
    let ast = compile_script(&engine, script);
    let facade = RhaiSystemPromptFacade::new(
        "my-llm".to_string(),
        Arc::clone(&engine),
        ast,
        config_dynamic("my-llm"),
    );

    // @step When I call facade.format_for_api("body")
    let value = facade.format_for_api("body");

    // @step Then the result is a JSON array whose second block contains
    //       cache_control.type equal to "ephemeral"
    assert!(
        value.is_array(),
        "array-format result should produce a JSON array, got {value:?}"
    );
    let arr = value.as_array().expect("array");
    assert_eq!(arr.len(), 2, "expected 2 blocks, got {}", arr.len());
    assert_eq!(
        arr[0]["type"].as_str(),
        Some("text"),
        "first block should be a text block"
    );
    assert_eq!(
        arr[0]["text"].as_str(),
        Some("prefix"),
        "first block text should be 'prefix'"
    );
    assert_eq!(
        arr[1]["text"].as_str(),
        Some("body"),
        "second block text should echo the preamble"
    );
    assert_eq!(
        arr[1]["cache_control"]["type"].as_str(),
        Some("ephemeral"),
        "second block should preserve cache_control.type = 'ephemeral'"
    );
}

// =========================================================================
// Scenario: format_system_prompt returning string produces plain JSON string
// =========================================================================
#[test]
fn format_system_prompt_string_produces_plain_json_string() {
    // @step Given a Rhai script whose format_system_prompt returns the plain string "abc"
    let engine = test_engine();
    let script = r#"
        fn format_system_prompt(config, preamble, fspec_guidance) {
            "abc"
        }
    "#;
    let ast = compile_script(&engine, script);
    let facade = RhaiSystemPromptFacade::new(
        "my-llm".to_string(),
        Arc::clone(&engine),
        ast,
        config_dynamic("my-llm"),
    );

    // @step When I call facade.format_for_api("body")
    let value = facade.format_for_api("body");

    // @step Then the result is a JSON String equal to "abc"
    assert!(value.is_string(), "result should be a JSON String, got {value:?}");
    assert_eq!(value.as_str(), Some("abc"));
}

// =========================================================================
// Scenario: Facade reports custom provider name
// =========================================================================
#[test]
fn facade_reports_custom_provider_name() {
    // @step Given a ProviderConfig with name "my-llm"
    let engine = test_engine();
    let script = r#"
        fn unrelated() { 1 }
    "#;
    let ast = compile_script(&engine, script);

    // @step When I build a RhaiSystemPromptFacade from that config
    let facade = RhaiSystemPromptFacade::new(
        "my-llm".to_string(),
        Arc::clone(&engine),
        ast,
        config_dynamic("my-llm"),
    );

    // @step Then facade.provider() returns "my-llm"
    assert_eq!(
        facade.provider(),
        "my-llm",
        "provider() should return the custom provider name"
    );
}

// =========================================================================
// Scenario: Runtime error in format_system_prompt falls back gracefully
// =========================================================================
#[test]
fn runtime_error_in_format_system_prompt_falls_back() {
    // @step Given a Rhai script whose format_system_prompt throws a runtime error
    let engine = test_engine();
    let script = r#"
        fn format_system_prompt(config, preamble, fspec_guidance) {
            throw "boom"
        }
    "#;
    let ast = compile_script(&engine, script);
    let facade = RhaiSystemPromptFacade::new(
        "my-llm".to_string(),
        Arc::clone(&engine),
        ast,
        config_dynamic("my-llm"),
    );

    // @step When I call facade.format_for_api("body")
    // @step Then the process does not panic and the result is a JSON String
    //       containing the default formatted preamble
    let value = facade.format_for_api("body");

    assert!(
        value.is_string(),
        "fallback result should be a JSON String, got {value:?}"
    );
    let text = value.as_str().expect("string value");
    let expected_default = prepend_fspec_guidance("body");
    assert_eq!(
        text, expected_default,
        "fallback should equal prepend_fspec_guidance(preamble)"
    );
}
