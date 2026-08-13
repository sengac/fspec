//! RPC-054 — ProviderSettingsView unit + view-routing tests.
//!
//! Feature: spec/features/rpc054-provider-settings-view.feature
//!
//! REVISION 2026-06-01: rewritten to drive the new full-screen
//! mode-view shape (ResumeSessionView pattern from RPC-026) — title +
//! separator + body + footer Layout, ConfirmDialog for destructive 'd',
//! Detail::Summary / Detail::EditApiKey / Detail::OAuthNotice sub-views,
//! filter mode + Esc-cascade.
//!
//! These tests drive the ProviderSettingsView in isolation (no App /
//! backend) — keyboard input → emitted Action / mode transition.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::{
    DetailStatus, DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

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

fn list_view_with(providers: Vec<ProviderCredentialInfo>) -> ProviderSettingsView {
    let mut v = ProviderSettingsView::new();
    v.set_providers(providers);
    v
}

// ────────────────────────────────────────────────────────────────────────
// List mode — navigation, scrolling, no-providers placeholder
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Open view with no providers shows the centred placeholder
#[test]
fn empty_view_enter_and_d_are_noop_esc_closes() {
    // @step Given the ProviderSettingsView is in List mode with an empty providers list
    let mut view = ProviderSettingsView::new();
    // @step When the view is rendered into a 80x24 area
    // @step Then the body row shows the centred placeholder "(no providers configured)"
    assert!(view.providers.is_empty());
    // @step And pressing Enter is a no-op
    let out = view.handle_key(key(KeyCode::Enter));
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    assert!(matches!(view.mode, ProviderSettingsMode::List));
    // @step And pressing "d" is a no-op
    let out = view.handle_key(key(KeyCode::Char('d')));
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    assert!(view.delete_confirm.is_none());
    // @step And pressing Esc emits ProviderSettingsEvent::Close
    let out = view.handle_key(key(KeyCode::Esc));
    assert!(matches!(out, ProviderSettingsEvent::Close));
}

/// Scenario: ↓ scrolls the window when the selection moves past the visible rows
#[test]
fn down_scrolls_window_past_visible_rows() {
    // @step Given the ProviderSettingsView is in List mode with 40 providers
    let providers: Vec<ProviderCredentialInfo> = (0..40)
        .map(|i| pinfo(&format!("p{i}"), "api_key", false, 0))
        .collect();
    let mut view = list_view_with(providers);
    // @step And the render area body height is 18 rows
    view.set_visible_rows(18);
    // @step When the user presses ↓ twenty times
    for _ in 0..20 {
        view.handle_key(key(KeyCode::Down));
    }
    // @step Then selected_index equals 20
    assert_eq!(view.selected_index, 20);
    // @step And scroll_offset has advanced so row 20 falls inside the visible window
    assert!(view.scroll_offset <= 20 && 20 < view.scroll_offset + 18);
    // @step And the rendered list shows the row at index 20 (verified via ensure_visible math)
    // (ensure_visible was applied during each Down press)
}

// ────────────────────────────────────────────────────────────────────────
// List → Detail transitions on Enter
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Enter on an api_key row transitions to Detail::Summary
#[test]
fn enter_on_api_key_row_transitions_to_detail_summary() {
    // @step Given the ProviderSettingsView is in List mode with "anthropic" focused
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    // @step And the anthropic row's credential_type is "api_key"
    // @step When the user presses Enter
    view.handle_key(key(KeyCode::Enter));
    // @step Then the view's mode is Detail { provider_id: "anthropic", sub: Summary { last_status: None } }
    match &view.mode {
        ProviderSettingsMode::Detail { provider_id, sub } => {
            assert_eq!(provider_id, "anthropic");
            assert!(matches!(sub, DetailSub::Summary { last_status: None }));
        }
        _ => panic!("expected Detail::Summary mode, got {:?}", view.mode),
    }
    // @step And the footer hint reads "r: refresh models · Esc: back" (RPC-154 dropped `t: test ·` for TS parity)
    let hint = view.footer_hint();
    assert!(
        !hint.contains("t: test"),
        "RPC-154: Summary footer hint must NOT advertise `t: test` — the `t` keybind is removed; hint was {hint:?}"
    );
    assert!(hint.contains("r: refresh models"));
    assert!(hint.contains("Esc: back"));
}

/// Scenario: Enter on an oauth row transitions directly to Detail::OAuthNotice
#[test]
fn enter_on_oauth_row_transitions_to_oauth_notice() {
    // @step Given the ProviderSettingsView is in List mode with "codex" focused
    let mut view = list_view_with(vec![pinfo("codex", "oauth", false, 0)]);
    // @step And the codex row's credential_type is "oauth"
    // @step When the user presses Enter
    view.handle_key(key(KeyCode::Enter));
    // @step Then the view's mode is Detail { provider_id: "codex", sub: OAuthNotice }
    match &view.mode {
        ProviderSettingsMode::Detail { provider_id, sub } => {
            assert_eq!(provider_id, "codex");
            assert!(matches!(sub, DetailSub::OAuthNotice));
        }
        _ => panic!("expected Detail::OAuthNotice mode, got {:?}", view.mode),
    }
    // @step And the body shows the read-only text "codex uses OAuth which is not yet supported in the Rust frontend"
    // @step And the footer hint reads "Esc: back" (RPC-106 bullet/lowercase-colon style)
    assert!(view.footer_hint().contains("Esc: back"));
}

// ────────────────────────────────────────────────────────────────────────
// Detail::Summary — t (test) and r (refresh) actions
// ────────────────────────────────────────────────────────────────────────

/// Scenario: t inside Detail::Summary is silently ignored (RPC-154 — TS parity)
///
/// RPC-054 originally asserted `t` emitted Action::TestProviderConnection.
/// RPC-154 removed that arm from handle_summary_key (TS binds no `t` on
/// any Detail surface — src/tui/inputHandlers/listModeHandler.ts), so
/// `t` now falls through the catch-all and is silently consumed with
/// `last_status` preserved. The canonical assertions for the new
/// behaviour live in tests/rpc154_summary_t_keybind_removal_shape.rs;
/// this test stays here as a smoke-check that nothing has re-introduced
/// the deviation under the rpc054 harness.
#[test]
fn t_inside_detail_summary_is_silently_ignored_rpc154() {
    // @step Given the ProviderSettingsView is in Detail { provider_id: "openai", sub: Summary { last_status: None } }
    let mut view = list_view_with(vec![pinfo("openai", "api_key", true, 4)]);
    view.handle_key(key(KeyCode::Enter)); // List → Detail::Summary
                                          // @step When the user presses "t"
    let out = view.handle_key(key(KeyCode::Char('t')));
    // @step Then the emitted ProviderSettingsEvent is Consumed (no Action)
    assert!(
        matches!(out, ProviderSettingsEvent::Consumed),
        "RPC-154: `t` in Detail::Summary must be silently Consumed (no Action); got {out:?}"
    );
    assert!(
        !matches!(
            out,
            ProviderSettingsEvent::Emit(Action::TestProviderConnection(_))
        ),
        "RPC-154: `t` must NOT emit Action::TestProviderConnection — that was the Rust-only deviation; got {out:?}"
    );
    // @step And view.mode remains Detail::Summary with last_status: None
    if let ProviderSettingsMode::Detail {
        sub: DetailSub::Summary { last_status },
        ..
    } = &view.mode
    {
        assert!(
            last_status.is_none(),
            "RPC-154: last_status must remain None — the catch-all preserves it; got {last_status:?}"
        );
    } else {
        panic!("expected Detail::Summary, got {:?}", view.mode);
    }
}

/// Scenario: r inside Detail::Summary emits RefreshProviderModels
#[test]
fn r_inside_detail_summary_emits_refresh_provider_models() {
    // @step Given the ProviderSettingsView is in Detail { provider_id: "openai", sub: Summary { last_status: None } }
    let mut view = list_view_with(vec![pinfo("openai", "api_key", true, 4)]);
    view.handle_key(key(KeyCode::Enter));
    // @step When the user presses "r"
    let out = view.handle_key(key(KeyCode::Char('r')));
    // @step Then the emitted ProviderSettingsEvent is Emit(Action::RefreshProviderModels("openai"))
    match out {
        ProviderSettingsEvent::Emit(Action::RefreshProviderModels(id)) => {
            assert_eq!(id, "openai");
        }
        _ => panic!("expected RefreshProviderModels action, got {out:?}"),
    }
    // @step And the last_status is updated to RefreshingModels
    // @step And the body shows "Refreshing models…"
    if let ProviderSettingsMode::Detail {
        sub: DetailSub::Summary { last_status },
        ..
    } = &view.mode
    {
        assert!(matches!(last_status, Some(DetailStatus::RefreshingModels)));
    } else {
        panic!("expected Detail::Summary, got {:?}", view.mode);
    }
}

// ────────────────────────────────────────────────────────────────────────
// Detail::Summary → Detail::EditApiKey on Enter
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Enter inside Detail::Summary on api_key provider opens EditApiKey
#[test]
fn enter_inside_detail_summary_on_api_key_opens_edit_form() {
    // @step Given the ProviderSettingsView is in Detail::Summary for "anthropic" (credential_type api_key)
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.handle_key(key(KeyCode::Enter)); // List → Detail::Summary
                                          // @step When the user presses Enter
    view.handle_key(key(KeyCode::Enter));
    // @step Then the view's mode is Detail { provider_id: "anthropic", sub: EditApiKey { draft: "" } }
    match &view.mode {
        ProviderSettingsMode::Detail {
            provider_id,
            sub: DetailSub::EditApiKey { draft },
        } => {
            assert_eq!(provider_id, "anthropic");
            assert!(draft.is_empty());
        }
        _ => panic!("expected Detail::EditApiKey, got {:?}", view.mode),
    }
    // @step And the body shows "Key: " followed by an empty masked input
    // @step And the footer hint reads "Enter: save · Esc: cancel" (RPC-106)
    let hint = view.footer_hint();
    assert!(hint.contains("Enter: save"));
    assert!(hint.contains("Esc: cancel"));
}

/// Scenario: Typing characters in EditApiKey grows the draft
#[test]
fn typing_in_edit_api_key_grows_draft() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.handle_key(key(KeyCode::Enter)); // → Detail::Summary
    view.handle_key(key(KeyCode::Enter)); // → Detail::EditApiKey
                                          // @step When the user types "sk-test-1"
    for c in "sk-test-1".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    // @step Then the draft equals "sk-test-1"
    // @step And the rendered Key line shows 9 masked characters ("•" × 9)
    match &view.mode {
        ProviderSettingsMode::Detail {
            sub: DetailSub::EditApiKey { draft },
            ..
        } => {
            assert_eq!(draft, "sk-test-1");
        }
        _ => panic!("expected EditApiKey"),
    }
}

/// Scenario: Backspace removes the last draft character
#[test]
fn backspace_removes_last_draft_character() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "abc"
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.handle_key(key(KeyCode::Enter));
    view.handle_key(key(KeyCode::Enter));
    for c in "abc".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    // @step When the user presses Backspace
    view.handle_key(key(KeyCode::Backspace));
    // @step Then the draft equals "ab"
    match &view.mode {
        ProviderSettingsMode::Detail {
            sub: DetailSub::EditApiKey { draft },
            ..
        } => {
            assert_eq!(draft, "ab");
        }
        _ => panic!("expected EditApiKey"),
    }
}

/// Scenario: Enter on EditApiKey with non-empty draft emits SaveProviderCredentials
///
/// RPC-162 — the post-save transition target changed from
/// `Detail::Summary { SavingCredentials }` to `List`. The save Action
/// itself is still emitted; the new RPC-162 feature file owns the
/// updated assertions. This test now only pins the Action payload to
/// preserve RPC-054 coverage of the "Enter→Save" path; the mode
/// assertion lives in
/// `provider_settings_edit_silent_cancel_rpc162.rs`.
#[test]
fn enter_on_edit_api_key_with_draft_emits_save() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "sk-test-1"
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.handle_key(key(KeyCode::Enter));
    view.handle_key(key(KeyCode::Enter));
    for c in "sk-test-1".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    // @step When the user presses Enter
    let out = view.handle_key(key(KeyCode::Enter));
    // @step Then the emitted ProviderSettingsEvent is Emit(Action::SaveProviderCredentials { provider_id: "anthropic", api_key: "sk-test-1" })
    match out {
        ProviderSettingsEvent::Emit(Action::SaveProviderCredentials {
            provider_id,
            api_key,
        }) => {
            assert_eq!(provider_id, "anthropic");
            assert_eq!(api_key, "sk-test-1");
        }
        _ => panic!("expected SaveProviderCredentials, got {out:?}"),
    }
}

// RPC-162 supersedes the legacy Enter-on-empty-draft inline-validation
// scenario. The new silent-cancel behavior is covered by
// `provider_settings_edit_silent_cancel_rpc162.rs::
// pressing_enter_on_an_empty_edit_api_key_draft_*`. The original test
// (which asserted `view.status.contains("API key cannot be empty")`)
// is deleted here.

// ────────────────────────────────────────────────────────────────────────
// ConfirmDialog flow for destructive 'd'
// ────────────────────────────────────────────────────────────────────────

/// Scenario: d on a configured row opens the ConfirmDialog
#[test]
fn d_on_configured_row_opens_confirm_dialog() {
    // @step Given the ProviderSettingsView is in List mode with "anthropic" focused
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 8)]);
    // @step And the anthropic row's configured field is true
    // @step When the user presses "d"
    let out = view.handle_key(key(KeyCode::Char('d')));
    // @step Then delete_confirm is Some(ConfirmDialog) with title "Delete credentials?", body "Delete credentials for anthropic?", primary "Delete", cancel "Cancel"
    let dialog = view.delete_confirm.as_ref().expect("confirm dialog");
    assert_eq!(dialog.title(), "Delete credentials?");
    assert_eq!(dialog.body(), "Delete credentials for anthropic?");
    assert_eq!(dialog.primary_label(), "Delete");
    assert_eq!(dialog.cancel_label(), "Cancel");
    // @step And NO ProviderSettingsEvent::Emit is dispatched
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    // @step And backend.delete_provider_credentials is NEVER called
    // (this view layer never calls backend; verified end-to-end in dispatch tests)
}

/// Scenario: d on an unconfigured row is a no-op
#[test]
fn d_on_unconfigured_row_is_noop() {
    // @step Given the ProviderSettingsView is in List mode with "anthropic" focused
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", false, 0)]);
    // @step And the anthropic row's configured field is false
    // @step When the user presses "d"
    let out = view.handle_key(key(KeyCode::Char('d')));
    // @step Then delete_confirm is None
    assert!(view.delete_confirm.is_none());
    // @step And no ProviderSettingsEvent::Emit is dispatched
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
}

/// Scenario: Enter on ConfirmDialog Primary emits ConfirmDeleteProviderCredentials
#[test]
fn enter_on_confirm_dialog_primary_emits_confirm_delete() {
    // @step Given the ProviderSettingsView's delete_confirm dialog is open for "anthropic" with Primary focused
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 8)]);
    view.handle_key(key(KeyCode::Char('d'))); // open dialog
    assert!(view.delete_confirm.is_some());
    // (default focus = Primary index 0 per ConfirmDialog::new)
    // @step When the user presses Enter
    let out = view.handle_key(key(KeyCode::Enter));
    // @step Then the emitted ProviderSettingsEvent is Emit(Action::ConfirmDeleteProviderCredentials("anthropic"))
    match out {
        ProviderSettingsEvent::Emit(Action::ConfirmDeleteProviderCredentials(id)) => {
            assert_eq!(id, "anthropic");
        }
        _ => panic!("expected ConfirmDeleteProviderCredentials, got {out:?}"),
    }
    // @step And delete_confirm is None
    assert!(view.delete_confirm.is_none());
    // @step And the view returns to List mode
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

/// Scenario: Esc on ConfirmDialog cancels without emitting
#[test]
fn esc_on_confirm_dialog_cancels_without_emit() {
    // @step Given the ProviderSettingsView's delete_confirm dialog is open for "anthropic"
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 8)]);
    view.handle_key(key(KeyCode::Char('d')));
    assert!(view.delete_confirm.is_some());
    // @step When the user presses Esc
    let out = view.handle_key(key(KeyCode::Esc));
    // @step Then delete_confirm is None
    assert!(view.delete_confirm.is_none());
    // @step And no ProviderSettingsEvent::Emit is dispatched
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    // @step And the view returns to List mode
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

// ────────────────────────────────────────────────────────────────────────
// Esc hierarchy
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Esc in List mode emits ProviderSettingsEvent::Close
#[test]
fn esc_in_list_mode_emits_close() {
    // @step Given the ProviderSettingsView is in List mode
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", false, 0)]);
    // @step And no ConfirmDialog is open
    assert!(view.delete_confirm.is_none());
    // @step When the user presses Esc
    let out = view.handle_key(key(KeyCode::Esc));
    // @step Then the emitted ProviderSettingsEvent is Close
    assert!(matches!(out, ProviderSettingsEvent::Close));
}

/// Scenario: Esc in Detail::Summary returns to List mode
#[test]
fn esc_in_detail_summary_returns_to_list() {
    // @step Given the ProviderSettingsView is in Detail::Summary for "openai" with selected_index = 5
    let providers: Vec<ProviderCredentialInfo> = (0..10)
        .map(|i| pinfo(&format!("p{i}"), "api_key", false, 0))
        .collect();
    let mut view = list_view_with(providers);
    // Move selection to index 5 then Enter to open Detail::Summary
    for _ in 0..5 {
        view.handle_key(key(KeyCode::Down));
    }
    assert_eq!(view.selected_index, 5);
    view.handle_key(key(KeyCode::Enter));
    assert!(matches!(view.mode, ProviderSettingsMode::Detail { .. }));
    // @step When the user presses Esc
    let out = view.handle_key(key(KeyCode::Esc));
    // @step Then the view's mode is List
    assert!(matches!(view.mode, ProviderSettingsMode::List));
    // @step And selected_index is still 5 (preserved)
    assert_eq!(view.selected_index, 5);
    // @step And no ProviderSettingsEvent::Close is emitted
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
}

// RPC-162 supersedes the legacy Esc-in-EditApiKey-returns-to-Summary
// scenario. The new direct-to-List behavior is covered by
// `provider_settings_edit_silent_cancel_rpc162.rs::
// pressing_esc_in_edit_api_key_transitions_directly_to_list_mode`.
// The original test is deleted here.

/// Scenario: Esc in Detail::OAuthNotice returns to List mode
#[test]
fn esc_in_detail_oauth_notice_returns_to_list() {
    // @step Given the ProviderSettingsView is in Detail::OAuthNotice for "codex"
    let mut view = list_view_with(vec![pinfo("codex", "oauth", false, 0)]);
    view.handle_key(key(KeyCode::Enter));
    assert!(matches!(
        view.mode,
        ProviderSettingsMode::Detail {
            sub: DetailSub::OAuthNotice,
            ..
        }
    ));
    // @step When the user presses Esc
    view.handle_key(key(KeyCode::Esc));
    // @step Then the view's mode is List
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

// ────────────────────────────────────────────────────────────────────────
// Title / footer rendering
// ────────────────────────────────────────────────────────────────────────

// Scenario "Title row shows configured count not total count" was removed
// in RPC-105: the header now reports `nav_items.len()` with the suffix
// "items" instead of the configured count. See
// spec/features/rpc105-provider-settings-header-nav-item-count.feature
// and tests/provider_settings_header_count_rpc105.rs.

/// Scenario: Footer hint in List mode (RPC-106 — TS-parity context-sensitive)
#[test]
fn footer_hint_list_mode() {
    // @step Given the ProviderSettingsView is in List mode
    let view = list_view_with(vec![pinfo("a", "api_key", false, 0)]);
    // @step When the view is rendered
    let hint = view.footer_hint();
    // RPC-106: this constructor uses set_providers (legacy), so nav_items
    // is empty and footer falls through to FOOTER_COMMON only. Per-row
    // hint coverage lives in provider_settings_footer_hints_rpc106.rs.
    // @step Then the footer row contains "/ filter"
    assert!(hint.contains("/ filter"), "hint: {hint:?}");
    // @step And the footer row contains "Tab: Switch to models"
    assert!(hint.contains("Tab: Switch to models"), "hint: {hint:?}");
    // @step And the footer row contains "Esc: close"
    assert!(hint.contains("Esc: close"), "hint: {hint:?}");
    // @step And the footer row uses U+00B7 MIDDLE DOT separators, not pipes
    assert!(!hint.contains('|'), "must not contain pipe: {hint:?}");
}

/// Scenario: Footer hint in Detail::Summary mode (RPC-106 bullet style — RPC-154 drops `t: test`)
#[test]
fn footer_hint_detail_summary_mode() {
    // @step Given the ProviderSettingsView is in Detail::Summary for "openai"
    let mut view = list_view_with(vec![pinfo("openai", "api_key", true, 4)]);
    view.handle_key(key(KeyCode::Enter));
    // @step When the view is rendered
    let hint = view.footer_hint();
    // @step Then the footer row does NOT contain "t: test" (RPC-154 removed the t keybind for TS parity)
    assert!(
        !hint.contains("t: test"),
        "RPC-154: Summary footer must NOT advertise `t: test`; hint was {hint:?}"
    );
    // @step And the footer row contains "r: refresh models"
    assert!(hint.contains("r: refresh models"));
    // @step And the footer row contains "Esc: back"
    assert!(hint.contains("Esc: back"));
}

/// Scenario: Footer hint in Detail::EditApiKey mode (RPC-106 bullet style)
#[test]
fn footer_hint_detail_edit_api_key_mode() {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic"
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.handle_key(key(KeyCode::Enter));
    view.handle_key(key(KeyCode::Enter));
    // @step When the view is rendered
    let hint = view.footer_hint();
    // @step Then the footer row contains "Enter: save"
    assert!(hint.contains("Enter: save"));
    // @step And the footer row contains "Esc: cancel"
    assert!(hint.contains("Esc: cancel"));
}

/// Scenario: Footer hint in Detail::OAuthNotice mode (RPC-106 bullet style)
#[test]
fn footer_hint_detail_oauth_notice_mode() {
    // @step Given the ProviderSettingsView is in Detail::OAuthNotice for "codex"
    let mut view = list_view_with(vec![pinfo("codex", "oauth", false, 0)]);
    view.handle_key(key(KeyCode::Enter));
    // @step When the view is rendered
    let hint = view.footer_hint();
    // @step Then the footer row contains "Esc: back"
    assert!(hint.contains("Esc: back"));
}

// ────────────────────────────────────────────────────────────────────────
// Filter mode (TS parity)
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Pressing "/" in List mode enters filter mode
#[test]
fn slash_in_list_mode_enters_filter_mode() {
    // @step Given the ProviderSettingsView is in List mode with providers ["anthropic", "openai", "codex"]
    let mut view = list_view_with(vec![
        pinfo("anthropic", "api_key", true, 1),
        pinfo("openai", "api_key", false, 0),
        pinfo("codex", "oauth", false, 0),
    ]);
    // @step And filter_mode is false
    assert!(!view.filter_mode);
    // @step And filter is ""
    assert_eq!(view.filter, "");
    // @step When the user presses "/"
    view.handle_key(key(KeyCode::Char('/')));
    // @step Then filter_mode is true
    assert!(view.filter_mode);
    // @step And filter is still ""
    assert_eq!(view.filter, "");
    // @step And no "/" character was inserted into any draft
    // (no Detail::EditApiKey active, so nothing to insert into)
}

/// Scenario: Typing characters in filter mode appends to filter string
#[test]
fn typing_in_filter_mode_appends_to_filter() {
    // @step Given the ProviderSettingsView is in List mode with filter_mode = true and filter = ""
    let mut view = list_view_with(vec![
        pinfo("anthropic", "api_key", true, 1),
        pinfo("openai", "api_key", false, 0),
        pinfo("codex", "oauth", false, 0),
    ]);
    view.handle_key(key(KeyCode::Char('/')));
    // @step When the user types "an"
    view.handle_key(key(KeyCode::Char('a')));
    view.handle_key(key(KeyCode::Char('n')));
    // @step Then filter equals "an"
    assert_eq!(view.filter, "an");
    // @step And the body row above the list shows "Filter: an"
    // (rendered output; checked by inspecting the field directly)
    // @step And the provider list shows only providers whose id or name contains "an" (case-insensitive)
    let visible = view.visible_provider_ids();
    assert!(visible.contains(&"anthropic".to_string()));
    assert!(!visible.contains(&"openai".to_string()));
    assert!(!visible.contains(&"codex".to_string()));
}

/// Scenario: Backspace in filter mode removes the last character
#[test]
fn backspace_in_filter_mode_removes_last_char() {
    // @step Given the ProviderSettingsView is in List mode with filter_mode = true and filter = "ant"
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.handle_key(key(KeyCode::Char('/')));
    for c in "ant".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    // @step When the user presses Backspace
    view.handle_key(key(KeyCode::Backspace));
    // @step Then filter equals "an"
    assert_eq!(view.filter, "an");
}

/// Scenario: Enter in filter mode exits filter mode but keeps the filter string
#[test]
fn enter_in_filter_mode_exits_but_keeps_filter() {
    // @step Given the ProviderSettingsView is in List mode with filter_mode = true and filter = "anth"
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.handle_key(key(KeyCode::Char('/')));
    for c in "anth".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    assert!(view.filter_mode);
    // @step When the user presses Enter
    view.handle_key(key(KeyCode::Enter));
    // @step Then filter_mode is false
    assert!(!view.filter_mode);
    // @step And filter equals "anth" (preserved)
    assert_eq!(view.filter, "anth");
    // @step And the visible providers are still filtered by "anth"
    let visible = view.visible_provider_ids();
    assert!(visible.contains(&"anthropic".to_string()));
}

/// Scenario: Esc in filter mode clears the filter string AND exits filter mode
#[test]
fn esc_in_filter_mode_clears_and_exits() {
    // @step Given the ProviderSettingsView is in List mode with filter_mode = true and filter = "xyz"
    let mut view = list_view_with(vec![
        pinfo("anthropic", "api_key", true, 1),
        pinfo("openai", "api_key", false, 0),
    ]);
    view.handle_key(key(KeyCode::Char('/')));
    for c in "xyz".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    // @step When the user presses Esc
    let out = view.handle_key(key(KeyCode::Esc));
    // @step Then filter_mode is false
    assert!(!view.filter_mode);
    // @step And filter equals ""
    assert_eq!(view.filter, "");
    // @step And the provider list is fully restored
    let visible = view.visible_provider_ids();
    assert_eq!(visible.len(), 2);
    // @step And no ProviderSettingsEvent::Close is emitted (Esc does NOT close the view in this case)
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
}

/// Scenario: Esc in List mode with a non-empty filter clears filter first (does not close view)
#[test]
fn esc_in_list_with_nonempty_filter_clears_filter_first() {
    // @step Given the ProviderSettingsView is in List mode with filter_mode = false and filter = "ant"
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    view.handle_key(key(KeyCode::Char('/')));
    for c in "ant".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    // Exit filter mode but keep the filter
    view.handle_key(key(KeyCode::Enter));
    assert!(!view.filter_mode);
    assert_eq!(view.filter, "ant");
    // @step When the user presses Esc
    let out = view.handle_key(key(KeyCode::Esc));
    // @step Then filter equals ""
    assert_eq!(view.filter, "");
    // @step And no ProviderSettingsEvent::Close is emitted
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    // @step And the view's mode is still List
    assert!(matches!(view.mode, ProviderSettingsMode::List));
}

/// Scenario: Esc in List mode with empty filter emits Close (second-Esc cascade)
#[test]
fn esc_in_list_with_empty_filter_emits_close() {
    // @step Given the ProviderSettingsView is in List mode with filter_mode = false and filter = ""
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    assert!(!view.filter_mode);
    assert_eq!(view.filter, "");
    // @step When the user presses Esc
    let out = view.handle_key(key(KeyCode::Esc));
    // @step Then the emitted ProviderSettingsEvent is Close
    assert!(matches!(out, ProviderSettingsEvent::Close));
}

/// Scenario: Filter substring is matched against both id and name (case-insensitive)
#[test]
fn filter_matches_id_or_name_case_insensitive() {
    // @step Given the ProviderSettingsView is in List mode with providers [{id: "github-copilot", name: "GitHub Copilot"}, {id: "anthropic", name: "Anthropic"}]
    let providers = vec![
        ProviderCredentialInfo {
            provider_id: "github-copilot".to_string(),
            display_name: "GitHub Copilot".to_string(),
            configured: false,
            credential_type: "oauth".to_string(),
            model_count: 0,
            masked_key: None,
            source: None,
        },
        ProviderCredentialInfo {
            provider_id: "anthropic".to_string(),
            display_name: "Anthropic".to_string(),
            configured: false,
            credential_type: "api_key".to_string(),
            model_count: 0,
            masked_key: None,
            source: None,
        },
    ];
    let mut view = list_view_with(providers);
    // @step And filter_mode = false and filter = "COPILOT"
    view.filter = "COPILOT".to_string();
    view.filter_mode = false;
    // @step Then the visible providers list contains "github-copilot"
    let visible = view.visible_provider_ids();
    assert!(visible.contains(&"github-copilot".to_string()));
    // @step And does NOT contain "anthropic"
    assert!(!visible.contains(&"anthropic".to_string()));
}

// ────────────────────────────────────────────────────────────────────────
// Source-shape — module declarations (legacy test kept for compatibility)
// ────────────────────────────────────────────────────────────────────────

/// Source-shape scenario: ProviderSettingsView module exists with the expected shape.
#[test]
fn provider_settings_view_module_compiles() {
    // @step Given the file rust/fspec-tui/src/views/provider_settings/mod.rs exists
    let _: ProviderSettingsView = ProviderSettingsView::default();
    let _ = ProviderSettingsMode::List;
    let _ = ProviderSettingsMode::Detail {
        provider_id: "x".to_string(),
        sub: DetailSub::Summary { last_status: None },
    };
    let _ = DetailSub::EditApiKey {
        draft: String::new(),
    };
    let _ = DetailSub::OAuthNotice;
    let _ = DetailStatus::Testing;
}

// ────────────────────────────────────────────────────────────────────────
// Detail::Summary rendering — TestOk / TestErr last_status
// ────────────────────────────────────────────────────────────────────────

/// Scenario: TestOk last_status renders as green "✓ ok (Xms)"
#[test]
fn test_ok_last_status_renders_green_check_with_latency() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    // @step Given the ProviderSettingsView is in Detail::Summary for "openai" with last_status = TestOk { latency_ms: 42 }
    let mut view = list_view_with(vec![pinfo("openai", "api_key", true, 4)]);
    view.mode = ProviderSettingsMode::Detail {
        provider_id: "openai".to_string(),
        sub: DetailSub::Summary {
            last_status: Some(DetailStatus::TestOk { latency_ms: 42 }),
        },
    };

    // @step When the view is rendered
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);

    // @step Then the body contains "✓ openai ok (42ms)" in green
    let mut found_green_ok = false;
    for y in 0..area.height {
        let mut line = String::new();
        let mut has_green_ok = false;
        for x in 0..area.width {
            let cell = &buf[(x, y)];
            line.push_str(cell.symbol());
            if cell.symbol() == "✓" && cell.style().fg == Some(Color::Green) {
                has_green_ok = true;
            }
        }
        if has_green_ok && line.contains("ok") && line.contains("42") {
            found_green_ok = true;
            break;
        }
    }
    assert!(
        found_green_ok,
        "expected a green '✓ ok (42ms)' line in the rendered buffer"
    );
}

/// Scenario: TestErr last_status renders as red "✗ <error>"
#[test]
fn test_err_last_status_renders_red_cross_with_error() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;

    // @step Given the ProviderSettingsView is in Detail::Summary for "openai" with last_status = TestErr { error: "unreachable: dns resolution failed" }
    let mut view = list_view_with(vec![pinfo("openai", "api_key", true, 4)]);
    view.mode = ProviderSettingsMode::Detail {
        provider_id: "openai".to_string(),
        sub: DetailSub::Summary {
            last_status: Some(DetailStatus::TestErr {
                error: "unreachable: dns resolution failed".to_string(),
            }),
        },
    };

    // @step When the view is rendered
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);

    // @step Then the body contains "✗ unreachable: dns resolution failed" in red
    let mut found_red_err = false;
    for y in 0..area.height {
        let mut line = String::new();
        let mut has_red_cross = false;
        for x in 0..area.width {
            let cell = &buf[(x, y)];
            line.push_str(cell.symbol());
            if cell.symbol() == "✗" && cell.style().fg == Some(Color::Red) {
                has_red_cross = true;
            }
        }
        if has_red_cross && line.contains("unreachable: dns resolution failed") {
            found_red_err = true;
            break;
        }
    }
    assert!(
        found_red_err,
        "expected a red '✗ unreachable: dns resolution failed' line in the rendered buffer"
    );
}
