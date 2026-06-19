//! Feature: spec/features/provider-settings-list-mode-keybinds-shape.feature
//!
//! Source-shape regression tests for RPC-149. Pins the absence of
//! Rust-only deviations (r/R refresh-models, wrap-around,
//! PageUp/PageDown/Home/End) and the presence of the exact TS contract
//! surface in the provider-settings list-mode key handler.
//!
//! These tests run in sub-milliseconds — they only read source strings,
//! no TUI rendering, no key event simulation, no async. The slow path
//! (actual key handling) is exercised by the existing
//! `provider_settings_*` integration tests in the same crate.

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

/// Extract the body of `pub(super) fn handle_list_key(...) -> ProviderSettingsEvent { ... }`.
/// Returns the substring from the function signature through the matching
/// closing brace. Used to scope assertions to the list-mode handler so
/// detail-mode arms (which legitimately keep `r/R`) don't pollute the
/// shape check.
fn handle_list_key_body(src: &str) -> &str {
    let needle = "fn handle_list_key(";
    let start = src
        .find(needle)
        .unwrap_or_else(|| panic!("handle_list_key function not found in list.rs"));
    let brace_start = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace for handle_list_key not found"));
    let abs_open = start + brace_start;
    // walk braces to find matching close
    let bytes = src.as_bytes();
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
    panic!("matching closing brace for handle_list_key not found");
}

/// Extract the body of `pub(crate) fn move_clamped(...)`.
fn move_clamped_body(src: &str) -> &str {
    let needle = "fn move_clamped(";
    let start = src
        .find(needle)
        .unwrap_or_else(|| panic!("move_clamped function not found in mod.rs"));
    let brace_start = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace for move_clamped not found"));
    let abs_open = start + brace_start;
    let bytes = src.as_bytes();
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
    panic!("matching closing brace for move_clamped not found");
}

#[test]
fn scenario_list_rs_handle_list_key_has_no_refresh_models_keybind_arms() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I scan the handle_list_key match body
    let body = handle_list_key_body(&src);

    // @step Then the source must NOT contain "KeyCode::Char('r')"
    assert!(
        !body.contains("KeyCode::Char('r')"),
        "list.rs handle_list_key must NOT bind `r` (refresh-models is a Rust-only deviation, RPC-149)"
    );

    // @step And the source must NOT contain "KeyCode::Char('R')"
    assert!(
        !body.contains("KeyCode::Char('R')"),
        "list.rs handle_list_key must NOT bind `R` (refresh-models is a Rust-only deviation, RPC-149)"
    );
}

#[test]
fn scenario_list_rs_handle_list_key_has_no_page_or_jump_key_arms() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I scan the handle_list_key match body
    let body = handle_list_key_body(&src);

    // @step Then the source must NOT contain "KeyCode::PageUp"
    assert!(
        !body.contains("KeyCode::PageUp"),
        "list.rs handle_list_key must NOT bind PageUp (Rust-only deviation, RPC-149)"
    );

    // @step And the source must NOT contain "KeyCode::PageDown"
    assert!(
        !body.contains("KeyCode::PageDown"),
        "list.rs handle_list_key must NOT bind PageDown (Rust-only deviation, RPC-149)"
    );

    // @step And the source must NOT contain "KeyCode::Home"
    assert!(
        !body.contains("KeyCode::Home"),
        "list.rs handle_list_key must NOT bind Home (Rust-only deviation, RPC-149)"
    );

    // @step And the source must NOT contain "KeyCode::End"
    assert!(
        !body.contains("KeyCode::End"),
        "list.rs handle_list_key must NOT bind End (Rust-only deviation, RPC-149)"
    );
}

#[test]
fn scenario_move_clamped_clamps_at_boundary_instead_of_wrapping() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/mod.rs
    let src = mod_rs();

    // @step When I locate the move_clamped function body
    let body = move_clamped_body(&src);

    // @step Then the source must contain ".clamp("
    assert!(
        body.contains(".clamp("),
        "move_clamped must use .clamp(...) to bound the selected index (TS parity, RPC-149)"
    );

    // @step And the source must NOT contain "% total"
    assert!(
        !body.contains("% total"),
        "move_clamped must NOT use modulo wrap-around against `total` (Rust-only deviation, RPC-149)"
    );

    // @step And the source must NOT contain "% max"
    assert!(
        !body.contains("% max"),
        "move_clamped must NOT use modulo wrap-around against `max` (Rust-only deviation, RPC-149)"
    );
}

#[test]
fn scenario_handle_list_key_match_arms_enumerate_exactly_the_ts_contract_surface() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I scan the handle_list_key match body
    let body = handle_list_key_body(&src);

    // @step Then the source must contain "KeyCode::Esc"
    assert!(
        body.contains("KeyCode::Esc"),
        "Esc arm required (TS parity)"
    );

    // @step And the source must contain "KeyCode::Char('/')"
    assert!(
        body.contains("KeyCode::Char('/')"),
        "`/` filter-mode arm required (TS parity)"
    );

    // @step And the source must contain "KeyCode::Tab"
    assert!(
        body.contains("KeyCode::Tab"),
        "Tab arm required (TS parity, RPC-160)"
    );

    // @step And the source must contain "KeyCode::Up"
    assert!(
        body.contains("KeyCode::Up"),
        "Up arrow arm required (TS parity)"
    );

    // @step And the source must contain "KeyCode::Down"
    assert!(
        body.contains("KeyCode::Down"),
        "Down arrow arm required (TS parity)"
    );

    // @step And the source must contain "KeyCode::Enter"
    assert!(
        body.contains("KeyCode::Enter"),
        "Enter arm required (TS parity)"
    );

    // @step And the source must contain "KeyCode::Char('d') | KeyCode::Char('D')"
    assert!(
        body.contains("KeyCode::Char('d') | KeyCode::Char('D')"),
        "delete-credentials d/D arm required (TS parity)"
    );
}
