//! PROV-104 — loading-vs-empty state parity (Rules [8],[9],[10]).
//!
//! Feature: spec/features/model-selector-keyboard-navigation-e2e.feature
//!
//! These two scenarios target the renderer bug where the not-yet-loaded
//! ("loading") state and the loaded-but-no-models ("empty") state are
//! INDISTINGUISHABLE: today `render_body` paints the single
//! `EMPTY_PLACEHOLDER = "No providers available"` whenever `rows.is_empty()`,
//! regardless of whether providers have finished loading. The view must
//! instead surface a DISTINCT visible loading indicator while
//! `providers_loaded() == false`, and an explicit no-models empty state once
//! loading has completed with no selectable models.
//!
//! Deterministic render-level tests (rather than the timing-racy real-binary
//! e2e path) reproduce these states exactly: a freshly-`new()` view has
//! `loaded == false` (the loading state), and `set_providers(vec![])` yields
//! `loaded == true` with no selectable rows (the empty state).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// Render the whole view into a `w`x`h` TestBackend and return the buffer
/// as one `String` per row. Populates `v.visible_rows` as a side effect.
fn render_lines(v: &mut ModelSelectorView, w: u16, h: u16) -> Vec<String> {
    let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).expect("term");
    term.draw(|f| v.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut lines = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut s = String::new();
        for x in 0..buf.area.width {
            s.push_str(buf[(x, y)].symbol());
        }
        lines.push(s);
    }
    lines
}

/// Scenario: Opening /model before providers load shows a loading state not a blank inert list
#[test]
fn opening_model_before_load_shows_loading_indicator_not_blank_list() {
    // @step Given the fspec binary is launched and a Work Agent is open
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));

    // @step When I submit "/model" and the provider list has not yet finished loading
    // The freshly-constructed view has not yet folded a `list_providers()`
    // result, so it is in the pre-load state.
    assert!(
        !v.providers_loaded(),
        "fresh view must be in the not-yet-loaded state"
    );
    let lines = render_lines(&mut v, 60, 14);
    let body = lines.join("\n");

    // @step Then the view shows a visible loading indicator rather than a blank list
    let lower = body.to_lowercase();
    assert!(
        lower.contains("loading"),
        "pre-load body must paint a visible loading indicator, got:\n{body}"
    );
    assert!(
        !body.contains("No providers available"),
        "the loading state must be DISTINCT from the empty placeholder, got:\n{body}"
    );
}

/// Scenario: Opening /model with no models shows an explicit empty state
#[test]
fn opening_model_with_no_models_shows_explicit_empty_state() {
    // @step Given the fspec binary is launched with FSPEC_USER_DIR pointing at a temp config with no profiles and no provider credentials
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));

    // @step When the provider list has finished loading with no selectable models
    v.set_providers(vec![]);
    assert!(
        v.providers_loaded(),
        "view must be marked loaded after set_providers"
    );
    assert_eq!(v.model_count(), 0, "no selectable models in the projection");
    let lines = render_lines(&mut v, 60, 14);
    let body = lines.join("\n");

    // @step Then the view shows an explicit no-models empty state instead of appearing to ignore arrow keys
    let lower = body.to_lowercase();
    assert!(
        lower.contains("no models"),
        "loaded-but-empty body must paint an explicit no-models empty state, got:\n{body}"
    );
    assert!(
        !lower.contains("loading"),
        "the empty state must be DISTINCT from the loading state, got:\n{body}"
    );

    // @step And I open a Work Agent and submit "/model"
    // (The view was opened above via construction + set_providers; this
    // step is the user action that surfaced the explicit empty state.)
    assert!(
        v.providers_loaded() && v.model_count() == 0,
        "the /model view is open, loaded, and shows no selectable models"
    );
}
