//! Feature: spec/features/provider-settings-delete-confirm-yn-shortcut-shape.feature
//!
//! Source-shape regression tests for RPC-156. Pins the presence of the
//! y/Y → Primary and n/N → Cancel shortcut keybinds (added by RPC-164)
//! in `ConfirmDialog::handle_key` so the TS parity behaviour cannot
//! silently regress without paying the full ratatui integration-test
//! compile cost on every CI run.
//!
//! These tests run in sub-milliseconds — they only read source strings,
//! no TUI rendering, no key event simulation, no async. The slow path
//! (actual key handling) is exercised by `confirm_dialog_yn_shortcut_rpc164.rs`.

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::path::PathBuf;

fn read_source(rel: &str) -> Result<String, codelet_test_helpers::TestError> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = [manifest, rel].iter().collect();
    Ok(fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {}", path.display(), err))?)
}

fn confirm_dialog_rs() -> Result<String, codelet_test_helpers::TestError> {
    read_source("src/views/agent/confirm_dialog.rs")
}

/// Extract the body of `pub fn handle_key(...) -> ConfirmDialogOutcome { ... }`.
/// Returns the substring from the opening brace of the function through
/// the matching closing brace. Used to scope assertions to the dialog
/// key handler so other unrelated arms in the file don't pollute the
/// shape check.
fn handle_key_body(src: &str) -> Result<&str, codelet_test_helpers::TestError> {
    let needle = "fn handle_key(";
    let start = src
        .find(needle)
        .or_fail("handle_key function not found in confirm_dialog.rs")?;
    let brace_start = src[start..]
        .find('{')
        .or_fail("opening brace for handle_key not found")?;
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
                    return Ok(&src[abs_open..=i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err("matching closing brace for handle_key not found".into())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: confirm_dialog.rs handle_key binds n/N as cancel shortcut
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_handle_key_binds_n_and_capital_n_as_cancel_shortcut() -> TestResult {
    // @step Given I read the source of rust/fspec-tui/src/views/agent/confirm_dialog.rs
    let src = confirm_dialog_rs()?;

    // @step When I scan the handle_key match body
    let body = handle_key_body(&src)?;

    // @step Then the source must contain "KeyCode::Char('n') | KeyCode::Char('N')"
    assert!(
        body.contains("KeyCode::Char('n') | KeyCode::Char('N')"),
        "ConfirmDialog::handle_key must bind n/N as cancel-shortcut (TS parity, RPC-156 / RPC-164)"
    );
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: confirm_dialog.rs handle_key binds y/Y as primary shortcut
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_handle_key_binds_y_and_capital_y_as_primary_shortcut() -> TestResult {
    // @step Given I read the source of rust/fspec-tui/src/views/agent/confirm_dialog.rs
    let src = confirm_dialog_rs()?;

    // @step When I scan the handle_key match body
    let body = handle_key_body(&src)?;

    // @step Then the source must contain "KeyCode::Char('y') | KeyCode::Char('Y')"
    assert!(
        body.contains("KeyCode::Char('y') | KeyCode::Char('Y')"),
        "ConfirmDialog::handle_key must bind y/Y as primary-shortcut (TS parity, RPC-156 / RPC-164)"
    );
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: n/N arm is wired to the cancel-index outcome path (not focused-index)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_n_arm_wires_to_cancel_index_outcome_not_focused_index() -> TestResult {
    // @step Given I read the source of rust/fspec-tui/src/views/agent/confirm_dialog.rs
    let src = confirm_dialog_rs()?;

    // @step When I locate the handle_key match arm for KeyCode::Char('n')
    let body = handle_key_body(&src)?;

    // @step Then the source must contain "outcome_for_index(self.cancel_index())"
    assert!(
        body.contains("outcome_for_index(self.cancel_index())"),
        "n/N arm must dispatch via outcome_for_index(self.cancel_index()) — focus state must NOT be consulted (RPC-156)"
    );
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_key modifier guard rejects Ctrl/Alt + y|Y|n|N
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_handle_key_modifier_guard_rejects_ctrl_alt_modifiers() -> TestResult {
    // @step Given I read the source of rust/fspec-tui/src/views/agent/confirm_dialog.rs
    let src = confirm_dialog_rs()?;

    // @step When I scan the top of the handle_key body
    let body = handle_key_body(&src)?;

    // @step Then the source must contain "mods.contains(KeyModifiers::CONTROL)"
    assert!(
        body.contains("mods.contains(KeyModifiers::CONTROL)"),
        "handle_key must retain the CONTROL modifier guard (RPC-156)"
    );

    // @step And the source must contain "mods.contains(KeyModifiers::ALT)"
    assert!(
        body.contains("mods.contains(KeyModifiers::ALT)"),
        "handle_key must retain the ALT modifier guard (RPC-156)"
    );

    // @step And the source must contain "ConfirmDialogOutcome::Ignored"
    assert!(
        body.contains("ConfirmDialogOutcome::Ignored"),
        "modifier guard must return ConfirmDialogOutcome::Ignored for Ctrl/Alt + y|Y|n|N (RPC-156)"
    );
    Ok(())
}
