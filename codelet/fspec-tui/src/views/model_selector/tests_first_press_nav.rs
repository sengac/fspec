//! Feature: spec/features/model-selector-first-press-navigation.feature
//!
//! PROV-124 — the /model selector must move the cursor on the FIRST arrow
//! press when opened with no matched current model, instead of swallowing the
//! first press to merely flip `has_selection`. `has_selection` gates Enter
//! only; movement is always a clamped no-wrap move (TS parity). These tests
//! also guard the PROV-101 Enter-no-op and the RPC-341 seed-on-current-model
//! behaviours so the fix does not regress them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

/// Build a fresh view with two providers, NO current model set, so
/// `set_providers` leaves every section collapsed and `has_selection == false`
/// with the cursor resting on row 0 (the first provider header).
fn no_current_model_view() -> ModelSelectorView {
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(None);
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);
    v
}

/// Scenario: First Down press moves the cursor one row when opened with no matched current model
#[test]
fn first_down_moves_cursor_one_row() {
    // @step Given the model selector is opened with no matched current model so every provider is collapsed and every row is a header
    let mut v = no_current_model_view();
    assert!(
        !v.has_active_selection(),
        "precondition: no active selection on a no-match open"
    );
    assert_eq!(v.selected_index(), 0, "precondition: cursor rests on row 0");
    assert!(
        !v.rows[v.selected_index()].selectable,
        "precondition: every row is a provider header"
    );

    // @step When I press Down once
    v.handle_key(key(KeyCode::Down));

    // @step Then the cursor moves from row 0 to row 1
    assert_eq!(
        v.selected_index(),
        1,
        "the first Down press must move the cursor from row 0 to row 1"
    );

    // @step And the selection is now active
    assert!(
        v.has_active_selection(),
        "the first explicit navigation must activate the selection"
    );
}

/// Scenario: First Up press at the top row is a clamped no-move but activates the selection
#[test]
fn first_up_at_top_is_clamped_no_move_but_activates() {
    // @step Given the model selector is opened with no matched current model so every provider is collapsed and every row is a header
    let mut v = no_current_model_view();
    assert!(
        !v.has_active_selection(),
        "precondition: no active selection on a no-match open"
    );
    assert_eq!(v.selected_index(), 0, "precondition: cursor rests on row 0");

    // @step When I press Up once
    v.handle_key(key(KeyCode::Up));

    // @step Then the cursor stays on row 0
    assert_eq!(
        v.selected_index(),
        0,
        "Up at the top row is a clamped no-move; the cursor stays on row 0"
    );

    // @step And the selection is now active
    assert!(
        v.has_active_selection(),
        "even a clamped no-move first navigation must activate the selection"
    );
}

/// Scenario: First PageDown press moves the selection by a viewport step
#[test]
fn first_pagedown_moves_a_viewport_step() {
    // @step Given the model selector is opened with no matched current model on a list taller than the viewport
    // tall_view(): one provider, 30 models. Rebuild with no current model so
    // the section is collapsed and has_selection stays false; render into a
    // short viewport so a PageDown step is smaller than the full list.
    let ids: Vec<String> = (0..30).map(|i| format!("m{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(None);
    v.set_providers(vec![provider("openai", &refs)]);
    // Expand the single section so there ARE rows below to page onto, but do
    // NOT navigate (has_selection stays false). Rebuild + render so
    // visible_rows reflects a real, short viewport.
    v.expanded = ["openai".to_string()].into_iter().collect();
    v.rebuild_rows();
    render_at(&mut v, 60, 12);
    assert!(
        !v.has_active_selection(),
        "precondition: no active selection before the first navigation"
    );
    assert_eq!(v.selected_index(), 0, "precondition: cursor rests on row 0");
    let step = v.visible_rows.max(1);
    assert!(
        v.rows.len() > step + 1,
        "precondition: the list is taller than one viewport step"
    );

    // @step When I press PageDown once
    v.handle_key(key(KeyCode::PageDown));

    // @step Then the cursor moves down by one viewport step
    assert_eq!(
        v.selected_index(),
        step,
        "the first PageDown press must move the selection down by one viewport step"
    );

    // @step And the selection is now active
    assert!(
        v.has_active_selection(),
        "the first PageDown must activate the selection"
    );
}

/// Scenario: Enter before any navigation is a no-op on a model row
///
/// PROV-101 regression guard. We construct the exact contested state: the
/// cursor rests on a SELECTABLE model row while `has_active_selection()` is
/// still false (a section is expanded so model rows exist, and the cursor is
/// placed on one WITHOUT an explicit navigation). Enter must be a consumed
/// no-op that emits no ModelSelected.
#[test]
fn enter_before_nav_is_noop_on_model_row() {
    // @step Given the model selector is opened with no matched current model and the cursor rests on a model row
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(None);
    v.set_providers(vec![provider("openai", &["gpt-4o", "o3-mini"])]);
    // Expand the section so model rows exist, then place the cursor on the
    // first selectable model row directly (no navigation → has_selection stays
    // false). This mirrors the "highlighted model row + Enter no-op" state.
    v.expanded = ["openai".to_string()].into_iter().collect();
    v.rebuild_rows();
    let model_row = v
        .rows
        .iter()
        .position(|r| r.selectable)
        .expect("an expanded section must expose a selectable model row");
    v.selected_index = model_row;
    assert!(
        v.rows[v.selected_index()].selectable,
        "precondition: cursor rests on a selectable model row"
    );

    // precondition: has_active_selection() is still false (no explicit navigation yet)
    assert!(
        !v.has_active_selection(),
        "precondition: has_active_selection() is still false (no explicit navigation yet)"
    );

    // @step When I press Enter before pressing any arrow key
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then the key is consumed and no model-selected action is emitted
    assert!(
        !matches!(out, ModelSelectorEvent::Emit(Action::ModelSelected(..))),
        "Enter with no active selection must NOT emit ModelSelected, got {out:?}"
    );
    assert!(
        matches!(out, ModelSelectorEvent::Consumed),
        "Enter with no active selection must be a consumed no-op, got {out:?}"
    );

    // @step And no selection is active
    assert!(
        !v.has_active_selection(),
        "a no-op Enter must not activate the selection"
    );
}

/// Scenario: Opening on a matched current model seeds the cursor and Enter selects immediately
///
/// RPC-341 regression guard.
#[test]
fn matched_current_model_seeds_cursor_and_enter_selects() {
    // @step Given my current model is "claude-sonnet"
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(Some("claude-sonnet".to_string()));

    // @step When the model selector loads the providers
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);

    // @step Then the cursor is seeded on the selectable row for "claude-sonnet" and the selection is active
    let row = &v.rows[v.selected_index()];
    assert!(row.selectable, "the seeded row must be selectable");
    assert_eq!(
        row.model_id, "claude-sonnet",
        "the cursor must be seeded on the current model's row"
    );
    assert!(
        v.has_active_selection(),
        "a matched current model must activate the selection on open"
    );

    // @step When I press Enter before pressing any arrow key
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then a model-selected action is emitted for "claude-sonnet"
    match out {
        ModelSelectorEvent::Emit(Action::ModelSelected(Some(sid), pkey, mid)) => {
            assert_eq!(sid.value, "s-1");
            assert_eq!(pkey, "anthropic");
            assert_eq!(mid, "claude-sonnet");
        }
        other => panic!("expected Emit(ModelSelected(Some(..), .., \"claude-sonnet\")), got {other:?}"),
    }
}
