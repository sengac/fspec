//! Feature: spec/features/provider-settings-tab-switch-to-models-shape.feature
//!
//! Source-shape regression tests for RPC-152. Pins:
//!   * presence of the `SwitchToModels` variant in the
//!     `ProviderSettingsEvent` enum (`mod.rs`)
//!   * `KeyCode::Tab => ProviderSettingsEvent::SwitchToModels` arm
//!     inside `handle_list_key` (`list.rs`)
//!   * ordering: filter_mode guard appears BEFORE the Tab arm so Tab
//!     while typing in the filter never escalates to the model picker
//!   * `handle_filter_key` body contains no occurrence of
//!     `SwitchToModels`
//!
//! These tests run in sub-milliseconds — they only read source strings,
//! no TUI rendering, no key event simulation, no async. The slow path
//! (actual key handling with simulated Tab events across all sub-modes)
//! is exercised by `provider_settings_tab_switch_to_models_rpc160.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn read_source(rel: &str) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = [manifest, rel].iter().collect();
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {}", path.display(), err))
}

fn list_rs() -> String {
    read_source("src/views/provider_settings/list.rs")
}

fn mod_rs() -> String {
    read_source("src/views/provider_settings/mod.rs")
}

/// Walk braces forward from `abs_open` (which must be the index of an
/// opening `{`) until the matching close, returning the inclusive
/// substring.
fn brace_balanced(src: &str, abs_open: usize) -> &str {
    let bytes = src.as_bytes();
    assert!(bytes[abs_open] == b'{', "abs_open must point at `{{`");
    let mut depth: i32 = 0;
    let mut i = abs_open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[abs_open..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("matching closing brace for opener at {abs_open} not found");
}

/// Extract the body of the named function (e.g. `"handle_list_key"`,
/// `"handle_filter_key"`). Returns the substring from the opening `{`
/// through the matching `}`.
fn fn_body<'a>(src: &'a str, fn_name: &str) -> &'a str {
    let needle = format!("fn {fn_name}(");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("`fn {fn_name}(` not found"));
    let brace_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace for `fn {fn_name}` not found"));
    brace_balanced(src, start + brace_rel)
}

/// Extract the body of the named enum (e.g. `"ProviderSettingsEvent"`).
fn enum_body<'a>(src: &'a str, enum_name: &str) -> &'a str {
    let needle = format!("pub enum {enum_name} {{");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("`pub enum {enum_name} {{` not found"));
    let brace_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace for `enum {enum_name}` not found"));
    brace_balanced(src, start + brace_rel)
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: ProviderSettingsEvent enum declares SwitchToModels variant
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_provider_settings_event_enum_declares_switch_to_models_variant() {
    // @step Given I read the source of rust/fspec-tui/src/views/provider_settings/mod.rs
    let src = mod_rs();

    // @step When I extract the body of the "pub enum ProviderSettingsEvent" declaration
    let body = enum_body(&src, "ProviderSettingsEvent");

    // @step Then the enum body must contain "SwitchToModels,"
    assert!(
        body.contains("SwitchToModels,"),
        "ProviderSettingsEvent must declare the SwitchToModels variant (TS parity, RPC-152 / RPC-160)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_list_key body binds Tab to SwitchToModels via an expression arm
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_handle_list_key_binds_tab_to_switch_to_models() {
    // @step Given I read the source of rust/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I scan the handle_list_key match body
    let body = fn_body(&src, "handle_list_key");

    // @step Then the body must contain "KeyCode::Tab => ProviderSettingsEvent::SwitchToModels"
    assert!(
        body.contains("KeyCode::Tab => ProviderSettingsEvent::SwitchToModels"),
        "handle_list_key must dispatch Tab to SwitchToModels via an expression arm (TS parity, RPC-152 / RPC-160)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_list_key checks filter_mode BEFORE dispatching Tab to SwitchToModels
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_filter_mode_guard_appears_before_tab_arm() {
    // @step Given I read the source of rust/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I scan the handle_list_key body
    let body = fn_body(&src, "handle_list_key");

    // @step Then the body must contain "if view.filter_mode {"
    let filter_offset = body
        .find("if view.filter_mode {")
        .expect("handle_list_key must guard on filter_mode (RPC-152)");

    // @step And the body must contain "KeyCode::Tab => ProviderSettingsEvent::SwitchToModels"
    let tab_offset = body
        .find("KeyCode::Tab => ProviderSettingsEvent::SwitchToModels")
        .expect("handle_list_key must contain the Tab → SwitchToModels arm (RPC-152)");

    // @step And the offset of "if view.filter_mode {" must be less than the offset of "KeyCode::Tab => ProviderSettingsEvent::SwitchToModels"
    assert!(
        filter_offset < tab_offset,
        "filter_mode guard must appear BEFORE the Tab arm so filter-mode Tab routes to handle_filter_key (RPC-152). filter_offset={filter_offset}, tab_offset={tab_offset}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_filter_key body does NOT emit SwitchToModels
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_handle_filter_key_does_not_emit_switch_to_models() {
    // @step Given I read the source of rust/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I extract the handle_filter_key function body
    let body = fn_body(&src, "handle_filter_key");

    // @step Then the function body must NOT contain "SwitchToModels"
    assert!(
        !body.contains("SwitchToModels"),
        "handle_filter_key must NOT emit SwitchToModels — Tab in filter mode stays in filter mode (RPC-152)"
    );
}
