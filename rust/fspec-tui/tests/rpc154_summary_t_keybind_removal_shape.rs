//! Feature: spec/features/provider-settings-detail-summary-t-keybind-removal-shape.feature
//!
//! RPC-154 — TS-parity regression-shape tests pinning the absence of the
//! `t` / `T` keybind in `handle_summary_key`. The TS canonical input
//! handler (src/tui/inputHandlers/listModeHandler.ts) does NOT bind
//! `t` to TestProviderConnection on any Detail screen, so the Rust
//! `Detail::Summary` arm that previously emitted
//! `Action::TestProviderConnection` is a Rust-only deviation. After
//! RPC-154 the arm is removed and `t` / `T` fall through to the
//! catch-all that re-enters Summary preserving `last_status`.
//!
//! Five tests:
//!   1. Lowercase `t` in `Detail::Summary { last_status: None }` is
//!      silently consumed (no Action, no state mutation).
//!   2. Uppercase `T` in `Detail::Summary { last_status: Some(Testing) }`
//!      is silently consumed and `last_status` is preserved.
//!   3. Source-shape: `handle_summary_key` body contains zero
//!      `KeyCode::Char('t')` and zero `KeyCode::Char('T')` substrings.
//!   4. Source-shape: `handle_summary_key` body contains zero
//!      `Action::TestProviderConnection` and zero `"Testing…"` substrings.
//!
//! Tests 3–4 read source-byte strings directly and run in
//! sub-milliseconds — they exist so a regression that re-introduces the
//! `t` arm fails CI before any behavioural test even runs. Tests 1 and 2
//! exercise the actual `handle_key` dispatch surface against a real view.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::{
    DetailStatus, DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

// ────────────────────────────────────────────────────────────────────────
// Test harness — keyboard + provider builders (mirrors rpc054 / rpc163)
// ────────────────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn pinfo(id: &str, ctype: &str, configured: bool, models: u32) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured,
        credential_type: ctype.to_string(),
        model_count: models,
        masked_key: None,
        source: None,
    }
}

/// Seed an api_key provider AND manually drop the view into
/// `Detail::Summary { last_status }` so the test does not rely on the
/// legacy `Enter` fallback path (which RPC-103/RPC-162 already routed
/// around for the populated-NavItem case) — this keeps the assertion
/// focused on the SINGLE behaviour under test: what does `handle_key`
/// do for `KeyCode::Char('t')` when the view is in Summary.
fn view_in_summary(provider_id: &str, last_status: Option<DetailStatus>) -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_providers(vec![pinfo(provider_id, "api_key", true, 1)]);
    v.mode = ProviderSettingsMode::Detail {
        provider_id: provider_id.to_string(),
        sub: DetailSub::Summary { last_status },
    };
    v
}

// ────────────────────────────────────────────────────────────────────────
// Source-file readers — pin paths relative to CARGO_MANIFEST_DIR so the
// test still works under `cargo test -p codelet-fspec-tui` from anywhere
// in the workspace.
// ────────────────────────────────────────────────────────────────────────

fn read_view_src(rel: &str) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = [manifest, "src", "views", "provider_settings", rel]
        .iter()
        .collect();
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn detail_rs() -> String {
    read_view_src("detail.rs")
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

/// Extract the brace-balanced body of the named function. Matches on the
/// signature substring `fn <name>(` so it works for both single-line and
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

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing lowercase t in Detail::Summary is silently ignored
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_lowercase_t_in_detail_summary_is_silently_ignored() {
    // @step Given a ProviderSettingsView seeded with one api_key provider "anthropic"
    // @step And the view has been transitioned into Detail::Summary { last_status: None } for "anthropic"
    let mut view = view_in_summary("anthropic", None);
    assert!(
        matches!(
            &view.mode,
            ProviderSettingsMode::Detail { provider_id, sub: DetailSub::Summary { last_status: None } }
                if provider_id == "anthropic"
        ),
        "precondition: view must start in Detail::Summary {{ last_status: None }} for anthropic, got {:?}",
        view.mode,
    );

    // @step When the user presses KeyCode::Char('t')
    let out = view.handle_key(key(KeyCode::Char('t')));

    // @step Then the returned ProviderSettingsEvent is Consumed
    assert!(
        matches!(out, ProviderSettingsEvent::Consumed),
        "RPC-154: `t` in Detail::Summary must be silently consumed (no Action emitted) — TS has no `t` keybind so Rust must not either; got {out:?}"
    );

    // @step And no Action::TestProviderConnection is emitted
    assert!(
        !matches!(
            out,
            ProviderSettingsEvent::Emit(Action::TestProviderConnection(_))
        ),
        "RPC-154: `t` must NOT emit Action::TestProviderConnection — this is the exact deviation RPC-154 closes; got {out:?}"
    );

    // @step And view.mode remains Detail::Summary for provider "anthropic" with last_status None
    match &view.mode {
        ProviderSettingsMode::Detail { provider_id, sub } => {
            assert_eq!(provider_id, "anthropic");
            assert!(
                matches!(sub, DetailSub::Summary { last_status: None }),
                "RPC-154: pressing `t` must NOT mutate last_status to Some(Testing) — sub was {sub:?}"
            );
        }
        other => panic!("RPC-154: view.mode must remain Detail::Summary, got {other:?}"),
    }

    // @step And view.status is the empty string
    assert_eq!(
        view.status, "",
        "RPC-154: view.status must NOT be set to \"Testing…\" by `t` — the only writer of that string in handle_summary_key (the `t` arm) is removed"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Pressing uppercase T in Detail::Summary is silently ignored
// even with existing Testing status
// ────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_uppercase_t_in_detail_summary_preserves_existing_last_status() {
    // @step Given a ProviderSettingsView seeded with one api_key provider "anthropic"
    // @step And view.mode is manually constructed as Detail::Summary { last_status: Some(Testing) } for "anthropic"
    let mut view = view_in_summary("anthropic", Some(DetailStatus::Testing));
    assert!(
        matches!(
            &view.mode,
            ProviderSettingsMode::Detail {
                provider_id,
                sub: DetailSub::Summary { last_status: Some(DetailStatus::Testing) }
            } if provider_id == "anthropic"
        ),
        "precondition: view must start in Detail::Summary {{ last_status: Some(Testing) }}, got {:?}",
        view.mode,
    );

    // @step When the user presses KeyCode::Char('T')
    let out = view.handle_key(key(KeyCode::Char('T')));

    // @step Then the returned ProviderSettingsEvent is Consumed
    assert!(
        matches!(out, ProviderSettingsEvent::Consumed),
        "RPC-154: uppercase `T` must mirror lowercase `t` — silently Consumed, no Action; got {out:?}"
    );

    // @step And no Action::TestProviderConnection is emitted
    assert!(
        !matches!(
            out,
            ProviderSettingsEvent::Emit(Action::TestProviderConnection(_))
        ),
        "RPC-154: uppercase `T` must NOT emit a second Action::TestProviderConnection while a previous test is already in flight; got {out:?}"
    );

    // @step And view.mode remains Detail::Summary for "anthropic" with last_status Some(Testing) preserved
    match &view.mode {
        ProviderSettingsMode::Detail { provider_id, sub } => {
            assert_eq!(provider_id, "anthropic");
            assert!(
                matches!(sub, DetailSub::Summary { last_status: Some(DetailStatus::Testing) }),
                "RPC-154: existing last_status: Some(Testing) must be preserved across the `T` keystroke (catch-all arm semantics); got {sub:?}"
            );
        }
        other => panic!("RPC-154: view.mode must remain Detail::Summary, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_summary_key source body contains zero KeyCode::Char('t')
// or KeyCode::Char('T') matches
// ────────────────────────────────────────────────────────────────────────

#[test]
fn handle_summary_key_source_contains_no_t_or_upper_t_keycode_arm() {
    // @step Given the file rust/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();

    // @step When the byte range delimited by "fn handle_summary_key(" through the next top-level "fn " is extracted
    let body = fn_body(&src, "handle_summary_key");

    // @step Then the substring "KeyCode::Char('t')" occurs zero times in that range
    assert!(
        !body.contains("KeyCode::Char('t')"),
        "RPC-154: handle_summary_key body must NOT match `KeyCode::Char('t')` — TS has no `t` keybind. Body was:\n{body}"
    );

    // @step And the substring "KeyCode::Char('T')" occurs zero times in that range
    assert!(
        !body.contains("KeyCode::Char('T')"),
        "RPC-154: handle_summary_key body must NOT match `KeyCode::Char('T')` (uppercase mirror of `t`). Body was:\n{body}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: handle_summary_key source body contains zero
// Action::TestProviderConnection construction sites and zero "Testing…"
// status writes
// ────────────────────────────────────────────────────────────────────────

#[test]
fn handle_summary_key_source_contains_no_test_provider_connection_dispatch() {
    // @step Given the file rust/fspec-tui/src/views/provider_settings/detail.rs
    let src = detail_rs();

    // @step When the byte range delimited by "fn handle_summary_key(" through the next top-level "fn " is extracted
    let body = fn_body(&src, "handle_summary_key");

    // @step Then the substring "Action::TestProviderConnection" occurs zero times in that range
    assert!(
        !body.contains("Action::TestProviderConnection"),
        "RPC-154: handle_summary_key body must NOT construct `Action::TestProviderConnection` — that was the `t` arm's dispatch site. Body was:\n{body}"
    );

    // @step And the substring "Testing…" (the legacy status text the `t` arm wrote) occurs zero times in that range
    assert!(
        !body.contains("Testing…"),
        "RPC-154: handle_summary_key body must NOT write the literal status string \"Testing…\" — that was set only by the now-removed `t` arm. Body was:\n{body}"
    );
}

