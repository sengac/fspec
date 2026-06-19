//! Feature: spec/features/provider-settings-api-key-edit-delete-key-removes-draft-chars-in-addition-to-backspace.feature
//!
//! Source-shape regression tests for RPC-155. Pins:
//!   * presence of the merged `KeyCode::Backspace | KeyCode::Delete =>`
//!     arm inside the `handle_edit_key` function body in
//!     `codelet/fspec-tui/src/views/provider_settings/detail.rs`
//!   * the brace-balanced body of that merged arm contains
//!     `draft.pop()` — proving Delete (and Backspace) actually deletes
//!     a trailing draft character
//!   * absence of any standalone `KeyCode::Delete =>` arm — Delete may
//!     only appear in the merged form, so the two key paths cannot
//!     diverge under future refactors
//!   * byte-offset ORDER: the merged `KeyCode::Backspace | KeyCode::Delete`
//!     arm appears BEFORE the `KeyCode::Char(c) =>` arm inside
//!     `handle_edit_key`, mirroring the TS handler at
//!     `src/tui/inputHandlers/apiKeyEditModeHandler.ts:46`
//!     (`if (key.backspace || key.delete) { … } return;`)
//!
//! These tests run in sub-milliseconds — they only read source strings,
//! no TUI rendering, no key event simulation, no async. The behavioural
//! path (actual Delete keystroke routed through ratatui) is exercised
//! by RPC-163's runtime tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn read_source(rel: &str) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = [manifest, rel].iter().collect();
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {}", path.display(), err))
}

fn detail_rs() -> String {
    read_source("src/views/provider_settings/detail.rs")
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

/// Extract the body of the named function. Matches on the signature
/// substring `fn <name>(` so it works for both single-line and
/// multi-line signatures.
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

/// Extract the brace-balanced body of a match arm whose pattern matches
/// the given substring (e.g. `"KeyCode::Backspace | KeyCode::Delete =>"`).
/// The needle MUST contain `=>` so we can anchor on the arrow and walk
/// forward to the opening brace.
fn match_arm_body<'a>(src: &'a str, arm_pattern_with_arrow: &str) -> &'a str {
    assert!(
        arm_pattern_with_arrow.contains("=>"),
        "arm needle must contain `=>`"
    );
    let arrow_start = src
        .find(arm_pattern_with_arrow)
        .unwrap_or_else(|| panic!("match arm `{arm_pattern_with_arrow}` not found"));
    let after_arrow = arrow_start + arm_pattern_with_arrow.len();
    let brace_rel = src[after_arrow..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace after `{arm_pattern_with_arrow}` not found"));
    brace_balanced(src, after_arrow + brace_rel)
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_edit_key body contains the merged
//           KeyCode::Backspace | KeyCode::Delete arm
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_handle_edit_key_body_contains_merged_backspace_delete_arm() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();

    // @step When I extract the handle_edit_key function body
    let body = fn_body(&src, "handle_edit_key");

    // @step Then the function body must contain "KeyCode::Backspace | KeyCode::Delete =>"
    assert!(
        body.contains("KeyCode::Backspace | KeyCode::Delete =>"),
        "handle_edit_key body must merge Backspace and Delete into a single arm `KeyCode::Backspace | KeyCode::Delete =>` so the two deletion key paths cannot diverge (RPC-155 / RPC-163 TS parity with `key.backspace || key.delete`)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: merged Backspace|Delete arm body contains draft.pop()
//           deletion call
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_merged_backspace_delete_arm_body_contains_draft_pop() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();
    let body = fn_body(&src, "handle_edit_key");

    // @step When I extract the brace-balanced body of the "KeyCode::Backspace | KeyCode::Delete =>" arm inside handle_edit_key
    let arm_body = match_arm_body(body, "KeyCode::Backspace | KeyCode::Delete =>");

    // @step Then the arm body must contain "draft.pop()"
    assert!(
        arm_body.contains("draft.pop()"),
        "merged Backspace|Delete arm body must invoke draft.pop() — TS parity with `setDraft((d) => d.slice(0, -1))` at src/tui/inputHandlers/apiKeyEditModeHandler.ts:46 (RPC-155 / RPC-163). Arm body was:\n{arm_body}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_edit_key body contains zero standalone
//           KeyCode::Delete arms
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_handle_edit_key_body_contains_zero_standalone_delete_arms() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();

    // @step When I extract the handle_edit_key function body
    let body = fn_body(&src, "handle_edit_key");

    // @step Then the function body must contain zero occurrences of the standalone substring "KeyCode::Delete =>" with no preceding "Backspace | " prefix
    //
    // Walk every occurrence of `KeyCode::Delete =>` and assert each is
    // immediately preceded by the substring `KeyCode::Backspace | ` so
    // the Delete keycode only ever appears in the merged form.
    let needle = "KeyCode::Delete =>";
    let prefix = "KeyCode::Backspace | ";
    let mut search_from = 0usize;
    let mut count_total = 0usize;
    let mut count_standalone = 0usize;
    while let Some(rel) = body[search_from..].find(needle) {
        let abs = search_from + rel;
        count_total += 1;
        let prefix_starts_at = abs.checked_sub(prefix.len());
        let is_merged = match prefix_starts_at {
            Some(start) => &body[start..abs] == prefix,
            None => false,
        };
        if !is_merged {
            count_standalone += 1;
        }
        search_from = abs + needle.len();
    }
    assert!(
        count_total > 0,
        "expected at least one occurrence of `KeyCode::Delete =>` in handle_edit_key body (merged with Backspace); found none"
    );
    assert_eq!(
        count_standalone, 0,
        "handle_edit_key body must NOT contain a standalone `KeyCode::Delete =>` arm — Delete may only appear merged with Backspace (`KeyCode::Backspace | KeyCode::Delete =>`) so the two key paths cannot diverge. Found {count_standalone} standalone occurrence(s) out of {count_total} total (RPC-155 / RPC-163)."
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: merged Backspace|Delete arm precedes the
//           KeyCode::Char(c) arm in handle_edit_key
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_merged_backspace_delete_arm_precedes_char_arm_in_handle_edit_key() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();

    // @step When I extract the handle_edit_key function body
    let body = fn_body(&src, "handle_edit_key");

    // @step Then the function body must contain "KeyCode::Backspace | KeyCode::Delete =>"
    let merged_offset = body
        .find("KeyCode::Backspace | KeyCode::Delete =>")
        .expect("handle_edit_key body must contain the merged `KeyCode::Backspace | KeyCode::Delete =>` arm (RPC-155)");

    // @step And the function body must contain "KeyCode::Char(c) =>"
    let char_offset = body.find("KeyCode::Char(c) =>").expect(
        "handle_edit_key body must contain the `KeyCode::Char(c) =>` arm (RPC-155 / RPC-161)",
    );

    // @step And the offset of "KeyCode::Backspace | KeyCode::Delete =>" must be less than the offset of "KeyCode::Char(c) =>"
    assert!(
        merged_offset < char_offset,
        "merged Backspace|Delete arm must appear BEFORE the KeyCode::Char(c) arm inside handle_edit_key — this documents the intended evaluation order of deletion vs printable-append paths (RPC-155 / RPC-163 TS parity). merged_offset={merged_offset}, char_offset={char_offset}"
    );
}
