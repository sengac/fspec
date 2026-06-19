//! Feature: spec/features/provider-settings-test-result-clear-on-nav-shape.feature
//!
//! Source-shape regression tests for RPC-151. Pins the structural shape
//! of the test_result-clear-on-arrow-nav behaviour in
//! `provider_settings::list::handle_list_key`. The Up and Down arms
//! must each invoke `view.clear_test_result()` gated by
//! `view.selected_index != before` (TS parity, added by RPC-159), while
//! non-arrow arms (Enter, Tab, Esc, '/', d/D) must NOT call it.
//!
//! These tests run in sub-milliseconds — they only read source strings,
//! no TUI rendering, no key event simulation, no async. The slow path
//! (actual key handling with simulated movement) is exercised by
//! `provider_settings_clear_test_result_on_nav_rpc159.rs`.

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

/// Extract the body of `pub(super) fn handle_list_key(...) -> ProviderSettingsEvent { ... }`.
/// Returns the substring from the opening `{` through the matching `}`.
fn handle_list_key_body(src: &str) -> &str {
    let needle = "fn handle_list_key(";
    let start = src
        .find(needle)
        .unwrap_or_else(|| panic!("handle_list_key function not found in list.rs"));
    let brace_start = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace for handle_list_key not found"));
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
    panic!("matching closing brace for handle_list_key not found");
}

/// Extract the body of a specific match arm whose pattern starts with
/// `pattern_prefix` (e.g. "KeyCode::Up", "KeyCode::Down",
/// "KeyCode::Enter", "KeyCode::Tab", "KeyCode::Esc", "KeyCode::Char('/')",
/// "KeyCode::Char('d') | KeyCode::Char('D')"). Returns the substring
/// from the opening `{` of the arm body through the matching `}`.
///
/// The search is scoped to `body` (already extracted via
/// `handle_list_key_body`) so other functions or unrelated match arms
/// don't pollute the result.
fn arm_body<'a>(body: &'a str, pattern_prefix: &str) -> &'a str {
    let pat_start = body.find(pattern_prefix).unwrap_or_else(|| {
        panic!("match arm starting with `{pattern_prefix}` not found in handle_list_key body")
    });
    // From the arm pattern, find the `=>` and then the opening `{` of
    // the block body. Some arms are expressions without braces (e.g.
    // `KeyCode::Tab => ProviderSettingsEvent::SwitchToModels`); for
    // those we fall back to slicing from `=>` to the next match-arm
    // comma at brace-depth 0.
    let arrow_rel = body[pat_start..]
        .find("=>")
        .unwrap_or_else(|| panic!("expected `=>` after pattern `{pattern_prefix}`"));
    let after_arrow = pat_start + arrow_rel + 2;
    // Skip whitespace.
    let mut i = after_arrow;
    let bytes = body.as_bytes();
    while i < bytes.len()
        && (bytes[i] == b' ' || bytes[i] == b'\n' || bytes[i] == b'\r' || bytes[i] == b'\t')
    {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'{' {
        // Block arm — walk braces.
        let abs_open = i;
        let mut depth: i32 = 0;
        let mut j = abs_open;
        while j < bytes.len() {
            match bytes[j] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return &body[abs_open..=j];
                    }
                }
                _ => {}
            }
            j += 1;
        }
        panic!("matching closing brace for arm `{pattern_prefix}` not found");
    }
    // Expression arm — walk forward until a comma at brace-depth 0.
    let abs_open = i;
    let mut depth: i32 = 0;
    let mut j = abs_open;
    while j < bytes.len() {
        match bytes[j] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b',' if depth == 0 => return &body[abs_open..j],
            _ => {}
        }
        j += 1;
    }
    panic!("end of expression arm `{pattern_prefix}` not found");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_list_key contains exactly two clear_test_result call sites
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_handle_list_key_has_exactly_two_clear_test_result_calls() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I scan the handle_list_key match body
    let body = handle_list_key_body(&src);

    // @step Then the body must contain exactly 2 occurrences of "clear_test_result("
    let count = body.matches("clear_test_result(").count();
    assert_eq!(
        count, 2,
        "handle_list_key must have exactly 2 clear_test_result( call sites (Up + Down), found {count} (RPC-151)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: KeyCode::Up arm clears test_result only on actual movement
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_up_arm_clears_test_result_only_on_actual_movement() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I extract the KeyCode::Up arm body inside handle_list_key
    let body = handle_list_key_body(&src);
    let up = arm_body(body, "KeyCode::Up");

    // @step Then the arm body must contain "let before = view.selected_index;"
    assert!(
        up.contains("let before = view.selected_index;"),
        "Up arm must capture selected_index before move (RPC-151)"
    );

    // @step And the arm body must contain "view.move_clamped(-1);"
    assert!(
        up.contains("view.move_clamped(-1);"),
        "Up arm must call move_clamped(-1) (RPC-151)"
    );

    // @step And the arm body must contain "if view.selected_index != before {"
    assert!(
        up.contains("if view.selected_index != before {"),
        "Up arm must gate test_result clear on actual movement (RPC-151)"
    );

    // @step And the arm body must contain "view.clear_test_result();"
    assert!(
        up.contains("view.clear_test_result();"),
        "Up arm must clear test_result on movement (RPC-151)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: KeyCode::Down arm clears test_result only on actual movement
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_down_arm_clears_test_result_only_on_actual_movement() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I extract the KeyCode::Down arm body inside handle_list_key
    let body = handle_list_key_body(&src);
    let down = arm_body(body, "KeyCode::Down");

    // @step Then the arm body must contain "let before = view.selected_index;"
    assert!(
        down.contains("let before = view.selected_index;"),
        "Down arm must capture selected_index before move (RPC-151)"
    );

    // @step And the arm body must contain "view.move_clamped(1);"
    assert!(
        down.contains("view.move_clamped(1);"),
        "Down arm must call move_clamped(1) (RPC-151)"
    );

    // @step And the arm body must contain "if view.selected_index != before {"
    assert!(
        down.contains("if view.selected_index != before {"),
        "Down arm must gate test_result clear on actual movement (RPC-151)"
    );

    // @step And the arm body must contain "view.clear_test_result();"
    assert!(
        down.contains("view.clear_test_result();"),
        "Down arm must clear test_result on movement (RPC-151)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Non-arrow arms must NOT clear test_result
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_non_arrow_arms_must_not_clear_test_result() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/list.rs
    let src = list_rs();

    // @step When I extract each non-arrow arm body (Enter, Tab, Esc, '/', d/D) inside handle_list_key
    let body = handle_list_key_body(&src);
    let enter = arm_body(body, "KeyCode::Enter");
    let tab = arm_body(body, "KeyCode::Tab");
    let esc = arm_body(body, "KeyCode::Esc");
    let slash = arm_body(body, "KeyCode::Char('/')");
    let dd = arm_body(body, "KeyCode::Char('d') | KeyCode::Char('D')");

    // @step Then the Enter arm body must NOT contain "clear_test_result("
    assert!(
        !enter.contains("clear_test_result("),
        "Enter arm must NOT clear test_result (RPC-151)"
    );

    // @step And the Tab arm body must NOT contain "clear_test_result("
    assert!(
        !tab.contains("clear_test_result("),
        "Tab arm must NOT clear test_result (RPC-151)"
    );

    // @step And the Esc arm body must NOT contain "clear_test_result("
    assert!(
        !esc.contains("clear_test_result("),
        "Esc arm must NOT clear test_result (RPC-151)"
    );

    // @step And the '/' filter-mode arm body must NOT contain "clear_test_result("
    assert!(
        !slash.contains("clear_test_result("),
        "'/' filter-mode arm must NOT clear test_result (RPC-151)"
    );

    // @step And the d/D arm body must NOT contain "clear_test_result("
    assert!(
        !dd.contains("clear_test_result("),
        "d/D arm must NOT clear test_result (RPC-151)"
    );
}
