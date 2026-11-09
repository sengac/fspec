//! RPC-054 — ProviderSettingsView unit + view-routing tests.
//!
//! Feature: spec/features/rpc054-provider-settings-view.feature
//!
//! REVISION 2026-06-01: rewritten to drive the new full-screen
//! mode-view shape (ResumeSessionView pattern from RPC-026) — title +
//! separator + body + footer Layout, ConfirmDialog for destructive 'd',
//! the Detail::EditApiKey sub-view,
//! filter mode + Esc-cascade.
//!
//! These tests drive the ProviderSettingsView in isolation (no App /
//! backend) — keyboard input → emitted Action / mode transition.

use codelet_fspec_tui::components::Action;
use codelet_fspec_tui::views::provider_settings::nav_item::NavItemKind;
use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::{
    DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};
use codelet_rpc_types::ProviderCredentialInfo;
use codelet_test_helpers::{OrFail, TestResult};
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

/// BUG-578: open Detail::EditApiKey for an api_key `provider` the way the UI
/// does: Enter on the expanded provider's API-key row.
fn edit_api_key_view(
    provider: &str,
) -> Result<ProviderSettingsView, codelet_test_helpers::TestError> {
    let infos = vec![pinfo(provider, "api_key", true, 1)];
    let mut v = ProviderSettingsView::new();
    v.set_provider_display_infos(project_display_infos(&infos, &[]));
    v.set_providers(infos);
    v.toggle_expansion(provider);
    v.selected_index = v
        .nav_items
        .iter()
        .position(|n| matches!(n.kind, NavItemKind::ApiKey))
        .or_fail("the expanded provider has an API-key row")?;
    v.handle_key(key(KeyCode::Enter));
    Ok(v)
}

// ────────────────────────────────────────────────────────────────────────
// List mode — navigation, scrolling, no-providers placeholder
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Open view with no providers shows the centred placeholder
#[test]
fn empty_view_enter_and_d_are_noop_esc_closes() -> TestResult {
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
    Ok(())
}

/// Scenario: ↓ scrolls the window when the selection moves past the visible rows
#[test]
fn down_scrolls_window_past_visible_rows() -> TestResult {
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
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Detail::EditApiKey, opened by Enter on the provider's API-key row
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Typing characters in EditApiKey grows the draft
#[test]
fn typing_in_edit_api_key_grows_draft() -> TestResult {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with empty draft
    let mut view = edit_api_key_view("anthropic")?;
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
        _ => return Err("expected EditApiKey".into()),
    }
    Ok(())
}

/// Scenario: Backspace removes the last draft character
#[test]
fn backspace_removes_last_draft_character() -> TestResult {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "abc"
    let mut view = edit_api_key_view("anthropic")?;
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
        _ => return Err("expected EditApiKey".into()),
    }
    Ok(())
}

/// Scenario: Enter on EditApiKey with non-empty draft emits SaveProviderCredentials
///
/// RPC-162 — the post-save transition target is `List`. The save Action
/// itself is still emitted; the new RPC-162 feature file owns the
/// updated assertions. This test now only pins the Action payload to
/// preserve RPC-054 coverage of the "Enter→Save" path; the mode
/// assertion lives in
/// `provider_settings_edit_silent_cancel_rpc162.rs`.
#[test]
fn enter_on_edit_api_key_with_draft_emits_save() -> TestResult {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic" with draft "sk-test-1"
    let mut view = edit_api_key_view("anthropic")?;
    for c in "sk-test-1".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    // @step When the user presses Enter
    let out = view.handle_key(key(KeyCode::Enter));
    // @step Then the emitted ProviderSettingsEvent is Emit(Action::SaveProviderCredentials { provider_id: "anthropic", api_key: "sk-test-1" })
    match out {
        ProviderSettingsEvent::Emit(action) => {
            let Action::SaveProviderCredentials {
                provider_id,
                api_key,
            } = *action
            else {
                return Err(format!("expected SaveProviderCredentials, got {action:?}").into());
            };
            assert_eq!(provider_id, "anthropic");
            assert_eq!(api_key, "sk-test-1");
        }
        other => return Err(format!("expected SaveProviderCredentials, got {other:?}").into()),
    }
    Ok(())
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
fn d_on_configured_row_opens_confirm_dialog() -> TestResult {
    // @step Given the ProviderSettingsView is in List mode with "anthropic" focused
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 8)]);
    // @step And the anthropic row's configured field is true
    // @step When the user presses "d"
    let out = view.handle_key(key(KeyCode::Char('d')));
    // @step Then delete_confirm is Some(ConfirmDialog) with title "Delete credentials?", body "Delete credentials for anthropic?", primary "Delete", cancel "Cancel"
    let dialog = view.delete_confirm.as_ref().or_fail("confirm dialog")?;
    assert_eq!(dialog.title(), "Delete credentials?");
    assert_eq!(dialog.body(), "Delete credentials for anthropic?");
    assert_eq!(dialog.primary_label(), "Delete");
    assert_eq!(dialog.cancel_label(), "Cancel");
    // @step And NO ProviderSettingsEvent::Emit is dispatched
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    // @step And backend.delete_provider_credentials is NEVER called
    // (this view layer never calls backend; verified end-to-end in dispatch tests)
    Ok(())
}

/// Scenario: d on an unconfigured row is a no-op
#[test]
fn d_on_unconfigured_row_is_noop() -> TestResult {
    // @step Given the ProviderSettingsView is in List mode with "anthropic" focused
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", false, 0)]);
    // @step And the anthropic row's configured field is false
    // @step When the user presses "d"
    let out = view.handle_key(key(KeyCode::Char('d')));
    // @step Then delete_confirm is None
    assert!(view.delete_confirm.is_none());
    // @step And no ProviderSettingsEvent::Emit is dispatched
    assert!(matches!(out, ProviderSettingsEvent::Consumed));
    Ok(())
}

/// Scenario: Enter on ConfirmDialog Primary emits ConfirmDeleteProviderCredentials
#[test]
fn enter_on_confirm_dialog_primary_emits_confirm_delete() -> TestResult {
    // @step Given the ProviderSettingsView's delete_confirm dialog is open for "anthropic" with Primary focused
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 8)]);
    view.handle_key(key(KeyCode::Char('d'))); // open dialog
    assert!(view.delete_confirm.is_some());
    // (default focus = Primary index 0 per ConfirmDialog::new)
    // @step When the user presses Enter
    let out = view.handle_key(key(KeyCode::Enter));
    // @step Then the emitted ProviderSettingsEvent is Emit(Action::ConfirmDeleteProviderCredentials("anthropic"))
    match out {
        ProviderSettingsEvent::Emit(action) => {
            let Action::ConfirmDeleteProviderCredentials(id) = *action else {
                return Err(
                    format!("expected ConfirmDeleteProviderCredentials, got {action:?}").into(),
                );
            };
            assert_eq!(id, "anthropic");
        }
        other => {
            return Err(format!("expected ConfirmDeleteProviderCredentials, got {other:?}").into())
        }
    }
    // @step And delete_confirm is None
    assert!(view.delete_confirm.is_none());
    // @step And the view returns to List mode
    assert!(matches!(view.mode, ProviderSettingsMode::List));
    Ok(())
}

/// Scenario: Esc on ConfirmDialog cancels without emitting
#[test]
fn esc_on_confirm_dialog_cancels_without_emit() -> TestResult {
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
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Esc hierarchy
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Esc in List mode emits ProviderSettingsEvent::Close
#[test]
fn esc_in_list_mode_emits_close() -> TestResult {
    // @step Given the ProviderSettingsView is in List mode
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", false, 0)]);
    // @step And no ConfirmDialog is open
    assert!(view.delete_confirm.is_none());
    // @step When the user presses Esc
    let out = view.handle_key(key(KeyCode::Esc));
    // @step Then the emitted ProviderSettingsEvent is Close
    assert!(matches!(out, ProviderSettingsEvent::Close));
    Ok(())
}

// RPC-162 supersedes the legacy Esc-in-EditApiKey-returns-to-Summary
// scenario. The new direct-to-List behavior is covered by
// `provider_settings_edit_silent_cancel_rpc162.rs::
// pressing_esc_in_edit_api_key_transitions_directly_to_list_mode`.
// The original test is deleted here.

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
fn footer_hint_list_mode() -> TestResult {
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
    Ok(())
}

/// Scenario: Footer hint in Detail::EditApiKey mode (RPC-106 bullet style)
#[test]
fn footer_hint_detail_edit_api_key_mode() -> TestResult {
    // @step Given the ProviderSettingsView is in Detail::EditApiKey for "anthropic"
    let view = edit_api_key_view("anthropic")?;
    // @step When the view is rendered
    let hint = view.footer_hint();
    // @step Then the footer row contains "Enter: save"
    assert!(hint.contains("Enter: save"));
    // @step And the footer row contains "Esc: cancel"
    assert!(hint.contains("Esc: cancel"));
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Filter mode (TS parity)
// ────────────────────────────────────────────────────────────────────────

/// Scenario: Pressing "/" in List mode enters filter mode
#[test]
fn slash_in_list_mode_enters_filter_mode() -> TestResult {
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
    Ok(())
}

/// Scenario: Typing characters in filter mode appends to filter string
#[test]
fn typing_in_filter_mode_appends_to_filter() -> TestResult {
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
    Ok(())
}

/// Scenario: Backspace in filter mode removes the last character
#[test]
fn backspace_in_filter_mode_removes_last_char() -> TestResult {
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
    Ok(())
}

/// Scenario: Enter in filter mode exits filter mode but keeps the filter string
#[test]
fn enter_in_filter_mode_exits_but_keeps_filter() -> TestResult {
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
    Ok(())
}

/// Scenario: Esc in filter mode clears the filter string AND exits filter mode
#[test]
fn esc_in_filter_mode_clears_and_exits() -> TestResult {
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
    Ok(())
}

/// Scenario: Esc in List mode with a non-empty filter clears filter first (does not close view)
#[test]
fn esc_in_list_with_nonempty_filter_clears_filter_first() -> TestResult {
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
    Ok(())
}

/// Scenario: Esc in List mode with empty filter emits Close (second-Esc cascade)
#[test]
fn esc_in_list_with_empty_filter_emits_close() -> TestResult {
    // @step Given the ProviderSettingsView is in List mode with filter_mode = false and filter = ""
    let mut view = list_view_with(vec![pinfo("anthropic", "api_key", true, 1)]);
    assert!(!view.filter_mode);
    assert_eq!(view.filter, "");
    // @step When the user presses Esc
    let out = view.handle_key(key(KeyCode::Esc));
    // @step Then the emitted ProviderSettingsEvent is Close
    assert!(matches!(out, ProviderSettingsEvent::Close));
    Ok(())
}

/// Scenario: Filter substring is matched against both id and name (case-insensitive)
#[test]
fn filter_matches_id_or_name_case_insensitive() -> TestResult {
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
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Source-shape — module declarations (legacy test kept for compatibility)
// ────────────────────────────────────────────────────────────────────────

/// Source-shape scenario: ProviderSettingsView module exists with the expected shape.
#[test]
fn provider_settings_view_module_compiles() -> TestResult {
    // @step Given the file rust/fspec-tui/src/views/provider_settings/mod.rs exists
    let _: ProviderSettingsView = ProviderSettingsView::default();
    let _ = ProviderSettingsMode::List;
    let _ = ProviderSettingsMode::Detail {
        provider_id: "x".to_string(),
        sub: DetailSub::EditApiKey {
            draft: String::new(),
        },
    };
    Ok(())
}
