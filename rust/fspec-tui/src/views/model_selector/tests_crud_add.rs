//! PROV-107 — RPC-344 custom-model CRUD: add/edit-open + submit tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;
use codelet_rpc_types::ModelEntry;

/// Scenario: Pressing 'a' on a profile-section header opens the Add Custom Model form
#[test]
fn a_on_profile_header_opens_add_form() {
    // @step Given the model selector is showing a local-server profile section
    // @step And the cursor is on that profile-section header
    let mut v = profile_header_view();
    assert!(!v.rows[v.selected_index()].selectable, "cursor on header");

    // @step When I press "a"
    v.handle_key(key(KeyCode::Char('a')));

    // @step Then the Add Custom Model form opens
    match v.custom_model_mode() {
        CustomModelMode::Add {
            provider_id,
            profile_name,
        } => {
            assert_eq!(provider_id, "openai");
            assert_eq!(profile_name, "my-profile");
        }
        other => panic!("expected Add mode, got {other:?}"),
    }
    // @step And every field is empty
    let f = v.form();
    assert!(f.id.is_empty() && f.display_name.is_empty() && f.facade.is_none());
    // @step And the Model ID field is focused
    assert_eq!(f.field_index, 0);
}

/// Scenario: Pressing 'a' on a cloud provider header does nothing
#[test]
fn a_on_cloud_header_is_noop() {
    // @step Given the model selector is showing a cloud provider header with no profile
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![provider("anthropic", &["claude"])]);
    // @step And the cursor is on that cloud provider header
    assert!(!v.rows[v.selected_index()].selectable);

    // @step When I press "a"
    let out = v.handle_key(key(KeyCode::Char('a')));

    // @step Then no form opens
    assert_eq!(v.custom_model_mode(), &CustomModelMode::Browse);
    // @step And the model selector stays in browse mode
    assert!(matches!(out, ModelSelectorEvent::Consumed));
}

/// Scenario: Pressing 'e' on a custom model opens the Edit Custom Model form prefilled
#[test]
fn e_on_custom_model_opens_edit_prefilled() {
    // @step Given the model selector is showing a profile section with a custom model
    // Custom model carries a stored display name distinct from its id so the
    // prefill of a real display name is exercised (RPC-346 surfaces it on the row).
    let named_custom = ModelEntry {
        id: "mycustom".to_string(),
        display_name: "My Custom".to_string(),
        context_window: 128_000,
        supports_reasoning: false,
        supports_vision: false,
        is_custom: true,
    };
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![profile_provider_with(
        "openai",
        "my-profile",
        vec![model("base"), named_custom],
    )]);
    v.expanded = ["openai".to_string()].into_iter().collect();
    v.rebuild_rows();
    v.anchor_first_selectable();
    v.adjust_scroll();
    // @step And the cursor is on that custom model row
    v.handle_key(key(KeyCode::Down));
    let row = &v.rows[v.selected_index()];
    assert!(row.selectable && row.model_id == "mycustom");

    // @step When I press "e"
    v.handle_key(key(KeyCode::Char('e')));

    // @step Then the Edit Custom Model form opens
    match v.custom_model_mode() {
        CustomModelMode::Edit {
            provider_id,
            profile_name,
            original_model_id,
        } => {
            assert_eq!(provider_id, "openai");
            assert_eq!(profile_name, "my-profile");
            assert_eq!(original_model_id, "mycustom");
        }
        other => panic!("expected Edit mode, got {other:?}"),
    }
    // @step And the form is prefilled with the model's id, display name, context window, reasoning and vision
    let f = v.form();
    assert_eq!(f.id, "mycustom");
    assert_eq!(f.display_name, "My Custom");
    assert_eq!(f.context_window, "128000");
    assert_eq!(f.reasoning, Some(false));
    assert_eq!(f.has_vision, Some(false));
}

/// Scenario: Pressing 'e' or 'd' on a built-in model does nothing
#[test]
fn e_and_d_on_builtin_model_are_noop() {
    // @step Given the model selector is showing a profile section with a built-in non-custom model
    let mut v = expanded_profile_view();
    // @step And the cursor is on that built-in model row
    let row = &v.rows[v.selected_index()];
    assert!(row.selectable && row.model_id == "base");

    // @step When I press "e"
    v.handle_key(key(KeyCode::Char('e')));
    // @step Then no form opens
    // @step And the model selector stays in browse mode
    assert_eq!(v.custom_model_mode(), &CustomModelMode::Browse);

    // @step When I press "d"
    v.handle_key(key(KeyCode::Char('d')));
    // @step Then no delete confirmation opens
    // @step And the model selector stays in browse mode
    assert_eq!(v.custom_model_mode(), &CustomModelMode::Browse);
}

/// Scenario: Adding a custom model with a facade and reasoning enabled saves it
#[test]
fn add_flow_emits_add_custom_model() {
    // @step Given the Add Custom Model form is open for a profile section
    let mut v = profile_header_view();
    v.handle_key(key(KeyCode::Char('a')));

    // @step When I type a Model ID
    for c in "my-model".chars() {
        v.handle_key(key(KeyCode::Char(c)));
    }
    // @step And I move down to the Facade field and press the right arrow twice
    v.handle_key(key(KeyCode::Down));
    v.handle_key(key(KeyCode::Down));
    v.handle_key(key(KeyCode::Right));
    v.handle_key(key(KeyCode::Right));
    // @step And I move down to the Reasoning field and press the right arrow once
    for _ in 0..4 {
        v.handle_key(key(KeyCode::Down));
    }
    v.handle_key(key(KeyCode::Right));
    // @step And I press Enter
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then a custom model is saved with the typed id, the selected facade and reasoning enabled
    match out {
        ModelSelectorEvent::Emit(Action::AddCustomModel {
            provider_id,
            profile_name,
            definition,
        }) => {
            assert_eq!(provider_id, "openai");
            assert_eq!(profile_name, "my-profile");
            assert_eq!(definition.id, "my-model");
            assert_eq!(definition.facade.as_deref(), Some("codex"));
            assert_eq!(definition.reasoning, Some(true));
        }
        other => panic!("expected Emit(AddCustomModel), got {other:?}"),
    }
    // @step And the form closes and the provider list is refreshed
    assert_eq!(v.custom_model_mode(), &CustomModelMode::Browse);
}

/// Scenario: Saving the Add form with an empty Model ID is rejected
#[test]
fn add_empty_id_is_rejected() {
    // @step Given the Add Custom Model form is open for a profile section
    let mut v = profile_header_view();
    v.handle_key(key(KeyCode::Char('a')));
    // @step And the Model ID field is empty
    assert!(v.form().id.is_empty());

    // @step When I press Enter
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then no custom model is saved
    assert!(!matches!(out, ModelSelectorEvent::Emit(_)));
    // @step And the Add Custom Model form stays open
    assert!(matches!(v.custom_model_mode(), CustomModelMode::Add { .. }));
}

/// Scenario: A "80%" Compaction Trigger saves a percentage threshold
#[test]
fn compaction_percentage_saved() {
    // @step Given the Add Custom Model form is open for a profile section
    let mut v = profile_header_view();
    v.handle_key(key(KeyCode::Char('a')));
    // @step And I have typed a Model ID
    for c in "m1".chars() {
        v.handle_key(key(KeyCode::Char(c)));
    }
    // @step When I enter "80%" into the Compaction Trigger field
    for _ in 0..5 {
        v.handle_key(key(KeyCode::Down));
    }
    for c in "80%".chars() {
        v.handle_key(key(KeyCode::Char(c)));
    }
    // @step And I press Enter
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then the saved custom model carries a percentage compaction threshold of 80
    match out {
        ModelSelectorEvent::Emit(Action::AddCustomModel { definition, .. }) => {
            assert_eq!(
                definition.compaction_threshold_type.as_deref(),
                Some("percentage")
            );
            assert_eq!(definition.compaction_threshold_value, Some(80));
        }
        other => panic!("expected Emit(AddCustomModel), got {other:?}"),
    }
}

/// Scenario: A bare integer Compaction Trigger saves a tokens threshold
#[test]
fn compaction_tokens_saved() {
    // @step Given the Add Custom Model form is open for a profile section
    let mut v = profile_header_view();
    v.handle_key(key(KeyCode::Char('a')));
    // @step And I have typed a Model ID
    for c in "m1".chars() {
        v.handle_key(key(KeyCode::Char(c)));
    }
    // @step When I enter "200000" into the Compaction Trigger field
    for _ in 0..5 {
        v.handle_key(key(KeyCode::Down));
    }
    for c in "200000".chars() {
        v.handle_key(key(KeyCode::Char(c)));
    }
    // @step And I press Enter
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then the saved custom model carries a tokens compaction threshold of 200000
    match out {
        ModelSelectorEvent::Emit(Action::AddCustomModel { definition, .. }) => {
            assert_eq!(
                definition.compaction_threshold_type.as_deref(),
                Some("tokens")
            );
            assert_eq!(definition.compaction_threshold_value, Some(200_000));
        }
        other => panic!("expected Emit(AddCustomModel), got {other:?}"),
    }
}

/// Scenario: Pressing Esc in the Add form cancels without saving
#[test]
fn esc_in_add_form_cancels() {
    // @step Given the Add Custom Model form is open for a profile section
    let mut v = profile_header_view();
    v.handle_key(key(KeyCode::Char('a')));

    // @step When I press Esc
    let out = v.handle_key(key(KeyCode::Esc));

    // @step Then the form closes
    // @step And I am back in the browse list
    assert_eq!(v.custom_model_mode(), &CustomModelMode::Browse);
    // @step And no custom model is saved
    assert!(matches!(out, ModelSelectorEvent::Consumed));
}
