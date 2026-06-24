//! PROV-117 — Enter on a section header toggles expansion (TS parity).
//!
//! Feature: spec/features/model-selector-enter-key-behavior.feature
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

/// Build a fresh collapse-by-default view with the cursor resting on the
/// FIRST provider header. No current model is set, so `set_providers`
/// leaves every section collapsed and `has_selection == false`; the row
/// projection therefore consists solely of provider headers and
/// `selected_index` (0) sits on the first header.
fn collapsed_view_on_header() -> (ModelSelectorView, String) {
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(None);
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);
    // selected_index defaults to 0; rows[0] is the first (non-selectable)
    // provider header. Capture its provider_key for the assertions.
    let row = &v.rows[v.selected_index()];
    assert!(
        !row.selectable,
        "cursor must rest on a non-selectable header"
    );
    let provider_key = row.provider_key.clone();
    (v, provider_key)
}

/// Scenario: Pressing Enter on a collapsed provider header expands the section
#[test]
fn enter_on_collapsed_header_expands_section() {
    // @step Given the /model view is open with a provider section that is collapsed and the cursor is on its header
    let (mut v, provider_key) = collapsed_view_on_header();
    assert!(!v.is_expanded(&provider_key), "section starts collapsed");
    let models_before = v.model_count();

    // @step When I press Enter
    v.handle_key(key(KeyCode::Enter));

    // @step Then the section expands and its model rows become visible
    assert!(
        v.is_expanded(&provider_key),
        "Enter on a collapsed header must expand the section"
    );
    assert!(
        v.model_count() > models_before,
        "expanding the section must reveal its model rows"
    );
}

/// Scenario: Pressing Enter again on an expanded provider header collapses the section
#[test]
fn enter_on_expanded_header_collapses_section() {
    // @step Given the /model view is open with a provider section that is expanded and the cursor is on its header
    let (mut v, provider_key) = collapsed_view_on_header();
    // Expand it first (Right), then re-focus the header so the cursor rests
    // on the now-expanded section's header.
    v.handle_key(key(KeyCode::Right));
    assert!(
        v.is_expanded(&provider_key),
        "precondition: section expanded"
    );
    // toggle_expansion re-anchors the cursor on the toggled provider's header.
    let row = &v.rows[v.selected_index()];
    assert!(
        !row.selectable && row.provider_key == provider_key,
        "cursor must rest on the expanded section's header"
    );
    let models_before = v.model_count();

    // @step When I press Enter
    v.handle_key(key(KeyCode::Enter));

    // @step Then the section collapses and its model rows are hidden
    assert!(
        !v.is_expanded(&provider_key),
        "Enter on an expanded header must collapse the section"
    );
    assert!(
        v.model_count() < models_before,
        "collapsing the section must hide its model rows"
    );
}

/// Scenario: Pressing Enter on a selectable model row selects the model and closes the view
#[test]
fn enter_on_model_row_emits_selection() {
    // @step Given the /model view is open for an active session and the cursor is on a selectable model row
    let mut v = loaded_view();
    v.handle_key(key(KeyCode::Home)); // anchor on first selectable model row
    assert!(
        v.rows[v.selected_index()].selectable,
        "cursor must rest on a selectable model row"
    );

    // @step When I press Enter
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then a model selection is emitted for the current session, provider and model
    match out {
        ModelSelectorEvent::Emit(Action::ModelSelected(Some(sid), pkey, mid)) => {
            assert_eq!(sid.value, "s-1");
            assert!(!pkey.is_empty());
            assert!(!mid.is_empty());
        }
        other => panic!("expected Emit(ModelSelected(Some(..))), got {other:?}"),
    }
    // @step And the model selector view closes
    // (close is driven by Navigator::apply_action on ModelSelected — the
    //  Emit above is the observable contract from this layer.)
}

/// Scenario: Pressing Enter on a model row selects the model even when there
/// is no active session (TS parity).
///
/// PROV-117 root cause #3: the TS implementation (ModelSelectorScreen.tsx
/// Enter handler) has NO session-existence guard — it always builds the
/// selection, fires `onSelectModel`, and closes the selector. Only the
/// downstream `modelSelectionService.selectModel` gates the *backend* model
/// write on `if (sessionId)`. The Rust Enter arm must therefore emit a
/// selection regardless of whether a session is present; closing + the
/// conditional backend write happen later in App::dispatch.
#[test]
fn enter_on_model_row_with_no_session_still_emits_selection() {
    // @step Given the /model view is open with NO active session and the cursor is on a selectable model row
    let mut v = loaded_view();
    v.set_session(None);
    v.handle_key(key(KeyCode::Home)); // anchor on first selectable model row
    assert!(
        v.rows[v.selected_index()].selectable,
        "cursor must rest on a selectable model row"
    );

    // @step When I press Enter
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then a model selection is still emitted (no session guard, matching TS)
    assert!(
        matches!(out, ModelSelectorEvent::Emit(Action::ModelSelected(..))),
        "expected Emit(ModelSelected) even with no session, got {out:?}"
    );
}
