//! PROV-107 — RPC-341 open-on-current-model cursor-seeding tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

/// Scenario: Cursor lands on the current model when it is loaded
#[test]
fn cursor_lands_on_current_model_when_loaded() {
    // @step Given my current model is "claude-sonnet"
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(Some("claude-sonnet".to_string()));

    // @step When the model selector loads the "openai" and "anthropic" providers
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);

    // @step Then the cursor is on the selectable row for "claude-sonnet"
    let row = &v.rows[v.selected_index];
    assert!(row.selectable);
    assert_eq!(row.model_id, "claude-sonnet");

    // @step And the cursor is not on the first model "gpt-4o"
    assert_ne!(row.model_id, "gpt-4o");
}

/// Scenario: No active selection when no current model is set (PROV-101)
#[test]
fn cursor_falls_back_when_no_current_model() {
    // @step Given no current model is set
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(None);

    // @step When the model selector loads the providers
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);

    // @step Then no row is auto-selected (PROV-101: no first-row fallback)
    assert!(
        !v.has_active_selection(),
        "no current model must leave nothing selected, not snap to index 0"
    );
}

/// Scenario: No active selection when the current model is not found (PROV-101)
#[test]
fn cursor_falls_back_when_current_model_not_found() {
    // @step Given my current model is "does-not-exist"
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(Some("does-not-exist".to_string()));

    // @step When the model selector loads the providers
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);

    // @step Then no row is auto-selected (PROV-101: no first-row fallback)
    assert!(
        !v.has_active_selection(),
        "an unmatched current model must leave nothing selected, not snap to index 0"
    );
}

/// PROV-117 root cause #1 (date-suffix / registry-normalization mismatch).
///
/// TS parity: `createModelSelection` persists the registry-NORMALIZED id
/// (`extractModelIdForRegistry(model.id)` → strips a trailing `-YYYYMMDD`),
/// while the selector ROWS carry the RAW dated id (`model.id`). On reopen the
/// `(current)` marker / cursor-seed compare the persisted normalized id against
/// the dated row id, normalizing BOTH sides
/// (`extractModelIdForRegistry(m.id) === normalizedModelId`,
/// modelInitializationService.ts:133-136). Rust currently compares the two ids
/// VERBATIM (state.rs:51, rows.rs:149, rows_render.rs:166), so a normalized
/// current id never matches a dated row id — the section never auto-expands and
/// the cursor never seeds. This test pins the TS-faithful behaviour and fails
/// RED on the current tree.
#[test]
fn cursor_seeds_on_dated_row_when_current_is_normalized() {
    // @step Given my persisted current model is the normalized family id "claude-sonnet-4"
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(Some("claude-sonnet-4".to_string()));

    // @step When the selector loads a provider whose only model carries a -YYYYMMDD date suffix
    v.set_providers(vec![provider("anthropic", &["claude-sonnet-4-20250514"])]);

    // @step Then the dated row is recognised as the current model and the cursor seeds on it
    assert!(
        v.has_active_selection(),
        "normalized current id must match the dated row (registry-normalized comparison)"
    );
    let row = &v.rows[v.selected_index];
    assert!(row.selectable);
    assert_eq!(row.model_id, "claude-sonnet-4-20250514");
}

/// Scenario: Seeded cursor on a below-the-fold model is scrolled into view
#[test]
fn seeded_cursor_below_fold_is_scrolled_into_view() {
    // @step Given my current model is in a long list below the viewport fold
    let ids: Vec<String> = (0..30).map(|i| format!("m{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(Some("m25".to_string()));

    // @step When the model selector loads the providers
    v.set_providers(vec![provider("openai", &refs)]);

    // @step Then the cursor is on the selectable row for my current model
    let row = &v.rows[v.selected_index];
    assert!(row.selectable);
    assert_eq!(row.model_id, "m25");

    // @step And the seeded row is scrolled into view
    assert!(v.selected_index >= v.scroll_offset);
    assert!(v.selected_index < v.scroll_offset + v.visible_rows);
}
