#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/custom-provider-rhai-scriptable-tool-facades.feature
//!
//! Integration tests for PROV-066: the Rhai-scriptable tool facade
//! adapter. `resolve_tools` walks either the Rhai script's
//! `define_tools` return value or the preset matching the config's
//! `tool_style` and produces a `Vec<RhaiToolDef>`. The adapter
//! (`RhaiToolFacadeAdapter`) is a getters-only adapter that surfaces
//! the Rhai-supplied tool name and schema and delegates
//! parameter mapping to the optional `map_tool_params` Rhai function,
//! falling back to default field-by-field deserialization into
//! `codelet_tools::facade::InternalFileParams`.
//!
//! These tests exercise `codelet_providers::custom::tool_facade`,
//! `::tool_resolve` and `::tool_presets` which do not yet exist —
//! the entire file fails to compile in the red phase.

use std::sync::Arc;

#[path = "custom_tool_facades_test_helpers.rs"]
mod helpers;

use helpers::{
    facade_config_with_script, make_loader, DEFINE_TOOLS_AND_RENAMING_MAP,
    DEFINE_TOOLS_AND_UNIT_MAP, DEFINE_TOOLS_READ_FILE, DEFINE_TOOLS_RUNTIME_ERROR,
    DEFINE_TOOLS_TWO_ENTRIES, DEFINE_TOOLS_UNKNOWN_MAPS_TO, NO_TOOL_FNS_SCRIPT,
};

use codelet_providers::custom::tool_facade::{
    apply_map_tool_params, default_to_internal_file, RhaiToolDef, RhaiToolFacadeAdapter,
};
use codelet_providers::custom::tool_presets::preset_tools;
use codelet_providers::custom::tool_resolve::resolve_tools;
use codelet_providers::custom::ToolStyle;
use codelet_tools::facade::InternalFileParams;

// =========================================================================
// Scenario: define_tools produces custom tool definitions
// =========================================================================
#[test]
fn define_tools_produces_custom_tool_definitions() {
    // @step Given a Rhai script whose define_tools returns a list containing a read_file entry with maps_to "file:read"
    let (_tmp, mut cfg) =
        facade_config_with_script("my-llm", DEFINE_TOOLS_READ_FILE, ToolStyle::Claude);
    let loader = make_loader();

    // @step When I resolve the tool list for that provider
    let tools = resolve_tools(&mut cfg, &loader).expect("resolve succeeds");

    // @step Then the resolved list contains a RhaiToolDef with name "read_file" and maps_to "file:read"
    assert_eq!(
        tools.len(),
        1,
        "expected exactly one tool, got {}",
        tools.len()
    );
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(tools[0].maps_to, "file:read");
}

// =========================================================================
// Scenario: tool_style openai preset generates snake_case tool names
// =========================================================================
#[test]
fn tool_style_openai_preset_generates_snake_case_tool_names() {
    // @step Given a ProviderConfig with tool_style "openai" and no define_tools function
    let (_tmp, mut cfg) =
        facade_config_with_script("my-llm", NO_TOOL_FNS_SCRIPT, ToolStyle::Openai);
    let loader = make_loader();

    // @step When I resolve the tool list
    let tools = resolve_tools(&mut cfg, &loader).expect("resolve succeeds");

    // @step Then the list contains read_file, write_file, edit_file, run_bash,
    //       grep_search, glob_search, list_dir, and web_search
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    for expected in [
        "read_file",
        "write_file",
        "edit_file",
        "run_bash",
        "grep_search",
        "glob_search",
        "list_dir",
        "web_search",
    ] {
        assert!(
            names.contains(&expected),
            "expected openai preset to include {expected}, got {names:?}"
        );
    }
}

// =========================================================================
// Scenario: Default tool_style claude generates PascalCase tool names
// =========================================================================
#[test]
fn default_tool_style_claude_generates_pascal_case_tool_names() {
    // @step Given a ProviderConfig with no tool_style and no define_tools
    // The "no tool_style" shape is encoded as the resolver default, which
    // is ToolStyle::Claude (rule 3 of the feature).
    let (_tmp, mut cfg) =
        facade_config_with_script("my-llm", NO_TOOL_FNS_SCRIPT, ToolStyle::Claude);
    let loader = make_loader();

    // @step When I resolve the tool list
    let tools = resolve_tools(&mut cfg, &loader).expect("resolve succeeds");

    // @step Then the list contains Read, Write, Edit, Bash, Grep, Glob, LS, and WebSearch
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    for expected in [
        "Read",
        "Write",
        "Edit",
        "Bash",
        "Grep",
        "Glob",
        "LS",
        "WebSearch",
    ] {
        assert!(
            names.contains(&expected),
            "expected claude preset to include {expected}, got {names:?}"
        );
    }
}

// =========================================================================
// Scenario: map_tool_params renames parameter names
// =========================================================================
#[test]
fn map_tool_params_renames_parameter_names() {
    // @step Given a Rhai script whose map_tool_params renames the incoming
    //       "filepath" to "file_path" for file:read
    let (_tmp, mut cfg) =
        facade_config_with_script("my-llm", DEFINE_TOOLS_AND_RENAMING_MAP, ToolStyle::Claude);
    let loader = make_loader();
    let tools = resolve_tools(&mut cfg, &loader).expect("resolve succeeds");
    let def = tools
        .iter()
        .find(|t| t.maps_to == "file:read")
        .expect("file:read tool");
    let adapter = RhaiToolFacadeAdapter::new(
        Arc::new(def.clone()),
        Arc::new(cfg.clone()),
        Arc::clone(&loader),
    )
    .expect("build adapter");

    // @step When the adapter maps tool params {"filepath": "a.txt"}
    let raw = serde_json::json!({ "filepath": "a.txt" });
    let mapped = apply_map_tool_params(&adapter, raw).expect("map_tool_params runs");
    let internal = default_to_internal_file(&def.maps_to, &mapped).expect("internal file params");

    // @step Then the resulting InternalFileParams::Read has file_path equal to "a.txt"
    match internal {
        InternalFileParams::Read { file_path, .. } => {
            assert_eq!(file_path, "a.txt");
        }
        other => panic!("expected InternalFileParams::Read, got {other:?}"),
    }
}

// =========================================================================
// Scenario: map_tool_params returning unit uses default mapping
// =========================================================================
#[test]
fn map_tool_params_returning_unit_uses_default_mapping() {
    // @step Given a Rhai script whose map_tool_params returns () for all tools
    let (_tmp, mut cfg) =
        facade_config_with_script("my-llm", DEFINE_TOOLS_AND_UNIT_MAP, ToolStyle::Claude);
    let loader = make_loader();
    let tools = resolve_tools(&mut cfg, &loader).expect("resolve succeeds");
    let def = tools
        .iter()
        .find(|t| t.maps_to == "file:read")
        .expect("file:read tool");
    let adapter = RhaiToolFacadeAdapter::new(
        Arc::new(def.clone()),
        Arc::new(cfg.clone()),
        Arc::clone(&loader),
    )
    .expect("build adapter");

    // @step When the adapter maps tool params {"file_path":"a.txt"} for file:read
    let raw = serde_json::json!({ "file_path": "a.txt" });
    let mapped = apply_map_tool_params(&adapter, raw).expect("map_tool_params runs");
    let internal = default_to_internal_file(&def.maps_to, &mapped).expect("internal file params");

    // @step Then the resulting InternalFileParams::Read has file_path equal to "a.txt"
    //       via default field-by-field deserialization
    match internal {
        InternalFileParams::Read { file_path, .. } => {
            assert_eq!(file_path, "a.txt");
        }
        other => panic!("expected InternalFileParams::Read, got {other:?}"),
    }
}

// =========================================================================
// Scenario: Partial tool list hides unlisted categories
// =========================================================================
#[test]
fn partial_tool_list_hides_unlisted_categories() {
    // @step Given a Rhai script whose define_tools returns only a file:read
    //       tool and a bash tool
    let (_tmp, mut cfg) =
        facade_config_with_script("my-llm", DEFINE_TOOLS_TWO_ENTRIES, ToolStyle::Claude);
    let loader = make_loader();

    // @step When I resolve the tool list
    let tools = resolve_tools(&mut cfg, &loader).expect("resolve succeeds");

    // @step Then the list contains exactly two RhaiToolDef entries and no others
    assert_eq!(
        tools.len(),
        2,
        "expected exactly two tools, got {}",
        tools.len()
    );
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(
        names.contains(&"read_file"),
        "expected read_file in {names:?}"
    );
    assert!(
        names.contains(&"run_bash"),
        "expected run_bash in {names:?}"
    );
}

// =========================================================================
// Scenario: Rhai error in define_tools falls back to preset
// =========================================================================
#[test]
fn rhai_error_in_define_tools_falls_back_to_preset() {
    // @step Given a Rhai script whose define_tools throws a runtime error
    //       and tool_style is "claude"
    let (_tmp, mut cfg) =
        facade_config_with_script("my-llm", DEFINE_TOOLS_RUNTIME_ERROR, ToolStyle::Claude);
    let loader = make_loader();

    // @step When I resolve the tool list
    let tools = resolve_tools(&mut cfg, &loader).expect("resolve falls back to preset");

    // @step Then the resolved list matches the claude preset
    let claude_preset = preset_tools(ToolStyle::Claude);
    let resolved_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    let preset_names: Vec<&str> = claude_preset.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        resolved_names, preset_names,
        "resolved names {resolved_names:?} should match claude preset {preset_names:?}"
    );
}

// =========================================================================
// Scenario: Unknown maps_to identifier is rejected with clear error
// =========================================================================
#[test]
fn unknown_maps_to_identifier_is_rejected_with_clear_error() {
    // @step Given a Rhai script whose define_tools returns a tool with
    //       maps_to "mystery:foo"
    let (_tmp, mut cfg) =
        facade_config_with_script("my-llm", DEFINE_TOOLS_UNKNOWN_MAPS_TO, ToolStyle::Claude);
    let loader = make_loader();

    // @step When I resolve the tool list
    let result = resolve_tools(&mut cfg, &loader);

    // @step Then I receive an error whose message contains "mystery:foo"
    //       and lists valid identifiers like "file:read"
    let err = result.expect_err("resolve should fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("mystery:foo"),
        "error should mention offending maps_to 'mystery:foo', got {msg}"
    );
    assert!(
        msg.contains("file:read"),
        "error should list valid identifier 'file:read', got {msg}"
    );
}

// =========================================================================
// Scenario: Adapter exposes Rhai-provided name and definition
// =========================================================================
#[test]
fn adapter_exposes_rhai_provided_name_and_definition() {
    // @step Given a RhaiToolDef with name "my_read", description "read a file",
    //       and parameters schema {type:"object", properties:{path:{type:"string"}}}
    let parameters = serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" }
        }
    });
    let def = RhaiToolDef {
        name: "my_read".to_string(),
        description: "read a file".to_string(),
        parameters: parameters.clone(),
        maps_to: "file:read".to_string(),
    };
    let (_tmp, cfg) = facade_config_with_script("my-llm", NO_TOOL_FNS_SCRIPT, ToolStyle::Claude);
    let loader = make_loader();

    // @step When I build a RhaiToolFacadeAdapter for that tool
    let adapter = RhaiToolFacadeAdapter::new(Arc::new(def), Arc::new(cfg), Arc::clone(&loader))
        .expect("build adapter");

    // @step Then adapter.name() returns "my_read" and adapter.parameters_schema() returns the JSON schema
    //       that matches the supplied parameters
    assert_eq!(adapter.name(), "my_read");
    assert_eq!(adapter.parameters_schema(), &parameters);
}

// =========================================================================
// Scenario: Resolved tools are cached for system prompt introspection
// =========================================================================
#[test]
fn resolved_tools_are_cached_for_system_prompt_introspection() {
    // @step Given a resolved tool list computed for a provider
    let (_tmp, mut cfg) =
        facade_config_with_script("my-llm", DEFINE_TOOLS_READ_FILE, ToolStyle::Claude);
    let loader = make_loader();
    let tools = resolve_tools(&mut cfg, &loader).expect("resolve succeeds");

    // @step When I inspect ProviderConfig.resolved_tools after resolution
    let cached = cfg
        .resolved_tools
        .as_ref()
        .expect("resolved_tools populated");

    // @step Then the field contains exactly the RhaiToolDef entries returned
    //       by resolution
    assert_eq!(
        cached.len(),
        tools.len(),
        "cached len {} should equal returned len {}",
        cached.len(),
        tools.len()
    );
    for (i, expected) in tools.iter().enumerate() {
        assert_eq!(cached[i].name, expected.name);
        assert_eq!(cached[i].maps_to, expected.maps_to);
        assert_eq!(cached[i].parameters, expected.parameters);
    }
}
