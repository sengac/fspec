//! PROV-107 — RPC-342 collapse-by-default expansion tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

/// Scenario: No current model set leaves every provider collapsed
#[test]
fn no_current_model_leaves_every_provider_collapsed() {
    // @step Given no current model is set
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(None);

    // @step When the model selector loads the "openai" and "anthropic" providers
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);

    // @step Then the "openai" provider is collapsed
    assert!(!v.is_expanded("openai"));
    // @step And the "anthropic" provider is collapsed
    assert!(!v.is_expanded("anthropic"));
    // @step And the title reads "Select Model (3 models)"
    assert_eq!(v.title_text(), "Select Model (3 models)");
}

/// Scenario: Only the current model's provider section is auto-expanded
#[test]
fn only_current_models_section_is_auto_expanded() {
    // @step Given my current model is "claude-sonnet"
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(Some("claude-sonnet".to_string()));

    // @step When the model selector loads the "openai" and "anthropic" providers
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);

    // @step Then the "anthropic" provider is expanded
    assert!(v.is_expanded("anthropic"));
    // @step And the "openai" provider is collapsed
    assert!(!v.is_expanded("openai"));
    // @step And the cursor is on the selectable row for "claude-sonnet"
    let row = &v.rows[v.selected_index];
    assert!(row.selectable);
    assert_eq!(row.model_id, "claude-sonnet");
}

/// Scenario: A current model in the first provider expands only that section
#[test]
fn current_model_in_first_provider_expands_only_that_section() {
    // @step Given my current model is "gpt-4o"
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(Some("gpt-4o".to_string()));

    // @step When the model selector loads the "openai" and "anthropic" providers
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);

    // @step Then the "openai" provider is expanded
    assert!(v.is_expanded("openai"));
    // @step And the "anthropic" provider is collapsed
    assert!(!v.is_expanded("anthropic"));
}

/// Scenario: Filtering reveals matches inside collapsed providers
#[test]
fn filtering_reveals_matches_inside_collapsed_providers() {
    // @step Given no current model is set
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(None);

    // @step And the model selector has loaded the "openai" and "anthropic" providers all collapsed
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);
    assert!(!v.is_expanded("openai"));

    // @step When I type the filter "o3"
    v.handle_key(key(KeyCode::Char('/')));
    v.handle_key(key(KeyCode::Char('o')));
    v.handle_key(key(KeyCode::Char('3')));

    // @step Then the model list shows the "o3-mini" model even though "openai" was collapsed
    assert!(v
        .rows
        .iter()
        .any(|r| r.selectable && r.model_id == "o3-mini"));
}

/// Scenario: Reloading providers re-applies the collapse default
#[test]
fn reloading_providers_reapplies_collapse_default() {
    // @step Given my current model is "gpt-4o"
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_current_model(Some("gpt-4o".to_string()));

    // @step And the model selector has loaded the providers with only "openai" expanded
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);
    assert!(v.is_expanded("openai"));
    assert!(!v.is_expanded("anthropic"));

    // @step When the providers are reloaded
    v.set_providers(vec![
        provider("openai", &["gpt-4o", "o3-mini"]),
        provider("anthropic", &["claude-sonnet"]),
    ]);

    // @step Then the "openai" provider is expanded
    assert!(v.is_expanded("openai"));
    // @step And the "anthropic" provider is collapsed
    assert!(!v.is_expanded("anthropic"));
}
