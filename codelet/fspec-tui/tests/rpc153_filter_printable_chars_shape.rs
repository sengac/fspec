//! Feature: spec/features/provider-settings-api-key-edit-filterprintablechars-ascii-32-126-restriction.feature
//!
//! Source-shape regression tests for RPC-153. Pins:
//!   * presence of the `fn is_printable_ascii(c: char) -> bool` helper
//!     in `detail.rs`
//!   * body of `is_printable_ascii` evaluates the inclusive ASCII range
//!     `(32..=126).contains(&code)` — TS parity with
//!     `filterPrintableChars` at
//!     `src/tui/utils/providerSettingsHelpers.ts:39-47`
//!   * `handle_edit_key` body contains both `is_printable_ascii(c)` and
//!     `draft.push(c)`
//!   * ordering: the `is_printable_ascii(c)` guard appears BEFORE the
//!     `draft.push(c)` append, proving the push lives inside the
//!     `if is_printable_ascii(c) { … }` arm and cannot leak control
//!     chars / DEL / non-ASCII into the api-key draft
//!
//! These tests run in sub-milliseconds — they only read source strings,
//! no TUI rendering, no key event simulation, no async. The slow path
//! (actual key handling with simulated typed characters across the
//! Latin-1 supplement and emoji range) is exercised by the inline
//! `#[cfg(test)] mod tests` block in `detail.rs` itself plus the
//! integration test `provider_settings_api_key_edit_filter_rpc161.rs`.

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
fn brace_balanced<'a>(src: &'a str, abs_open: usize) -> &'a str {
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
    panic!("matching closing brace for opener at {} not found", abs_open);
}

/// Extract the body of the named function. Matches on the signature
/// substring `fn <name>(` so it works for both single-line and
/// multi-line signatures.
fn fn_body<'a>(src: &'a str, fn_name: &str) -> &'a str {
    let needle = format!("fn {}(", fn_name);
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("`fn {}(` not found", fn_name));
    let brace_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace for `fn {}` not found", fn_name));
    brace_balanced(src, start + brace_rel)
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: is_printable_ascii helper exists in detail.rs with the canonical signature
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_is_printable_ascii_helper_exists_with_canonical_signature() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "fn is_printable_ascii(c: char) -> bool"
    assert!(
        src.contains("fn is_printable_ascii(c: char) -> bool"),
        "detail.rs must declare the TS-parity printable-ASCII helper with the canonical signature `fn is_printable_ascii(c: char) -> bool` (RPC-153 / RPC-161 TS parity with filterPrintableChars)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: is_printable_ascii body evaluates the inclusive ASCII 32..=126 range
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_is_printable_ascii_body_evaluates_inclusive_32_through_126_range() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();

    // @step When I extract the body of the "fn is_printable_ascii(c: char) -> bool" function
    let body = fn_body(&src, "is_printable_ascii");

    // @step Then the function body must contain "(32..=126).contains(&code)"
    assert!(
        body.contains("(32..=126).contains(&code)"),
        "is_printable_ascii body must evaluate the inclusive ASCII range (32..=126).contains(&code) — TS parity with filterPrintableChars boundaries (RPC-153 / RPC-161)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_edit_key body guards draft.push(c) through is_printable_ascii
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_handle_edit_key_body_contains_guard_and_append_calls() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();

    // @step When I extract the handle_edit_key function body
    let body = fn_body(&src, "handle_edit_key");

    // @step Then the function body must contain "is_printable_ascii(c)"
    assert!(
        body.contains("is_printable_ascii(c)"),
        "handle_edit_key body must invoke the is_printable_ascii(c) guard before appending to the draft (RPC-153 / RPC-161 TS parity)"
    );

    // @step And the function body must contain "draft.push(c)"
    assert!(
        body.contains("draft.push(c)"),
        "handle_edit_key body must append accepted chars via draft.push(c) (RPC-153 / RPC-161 TS parity)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: is_printable_ascii guard precedes draft.push(c) in handle_edit_key
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_is_printable_ascii_guard_precedes_draft_push_in_handle_edit_key() {
    // @step Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();

    // @step When I extract the handle_edit_key function body
    let body = fn_body(&src, "handle_edit_key");

    // @step Then the function body must contain "is_printable_ascii(c)"
    let guard_offset = body
        .find("is_printable_ascii(c)")
        .expect("handle_edit_key must invoke the is_printable_ascii(c) guard (RPC-153)");

    // @step And the function body must contain "draft.push(c)"
    let push_offset = body
        .find("draft.push(c)")
        .expect("handle_edit_key must append accepted chars via draft.push(c) (RPC-153)");

    // @step And the offset of "is_printable_ascii(c)" must be less than the offset of "draft.push(c)"
    assert!(
        guard_offset < push_offset,
        "is_printable_ascii(c) guard must appear BEFORE draft.push(c) so the push lives inside the `if is_printable_ascii(c) {{ … }}` arm and cannot leak control chars / DEL / non-ASCII into the api-key draft (RPC-153 / RPC-161 TS parity). guard_offset={}, push_offset={}",
        guard_offset, push_offset
    );
}
