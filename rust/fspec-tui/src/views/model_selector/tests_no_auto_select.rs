//! PROV-101 — the model selector does not auto-select the first row when no
//! current model is set.
//!
//! Feature: spec/features/model-selector-no-auto-select.feature
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

/// Scenario: model-selector does not auto-select the first row when no current model
#[test]
fn no_current_model_means_no_active_selection_and_enter_is_noop() {
    // @step Given a model selector with no current model set
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(None);

    // @step When the model selector loads providers with selectable rows
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);

    // @step Then the model selector reports no active selection
    assert!(
        !v.has_active_selection(),
        "with no current model the selector must NOT auto-snap a selection to index 0"
    );

    // @step And pressing Enter emits no model-selected action
    let out = v.handle_key(key(KeyCode::Enter));
    assert!(
        !matches!(
            out,
            ModelSelectorEvent::Emit(ref a)
                if matches!(a.as_ref(), Action::ModelSelected(..))
        ),
        "Enter must not silently select a model when nothing is highlighted, got {out:?}"
    );
}
