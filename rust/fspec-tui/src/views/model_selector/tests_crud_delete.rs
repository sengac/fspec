//! PROV-107 — RPC-344 custom-model CRUD: delete/confirm + render tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;

/// Scenario: Deleting a custom model after confirming
#[test]
fn delete_confirm_yes_emits_delete() {
    // @step Given the model selector is showing a profile section with a custom model
    let mut v = expanded_profile_view();
    // @step And the cursor is on that custom model row
    v.handle_key(key(KeyCode::Down));

    // @step When I press "d"
    v.handle_key(key(KeyCode::Char('d')));
    // @step Then a delete confirmation shows the model display name and profile name
    match v.custom_model_mode() {
        CustomModelMode::DeleteConfirm {
            model_id,
            display_name,
            profile_name,
            ..
        } => {
            assert_eq!(model_id, "mycustom");
            assert_eq!(display_name, "mycustom");
            assert_eq!(profile_name, "my-profile");
        }
        other => panic!("expected DeleteConfirm, got {other:?}"),
    }

    // @step When I press "y"
    let out = v.handle_key(key(KeyCode::Char('y')));
    // @step Then the custom model is deleted
    match out {
        ModelSelectorEvent::Emit(Action::DeleteCustomModel {
            provider_id,
            profile_name,
            model_id,
        }) => {
            assert_eq!(provider_id, "openai");
            assert_eq!(profile_name, "my-profile");
            assert_eq!(model_id, "mycustom");
        }
        other => panic!("expected Emit(DeleteCustomModel), got {other:?}"),
    }
    // @step And I am returned to the browse list and the provider list is refreshed
    assert_eq!(v.custom_model_mode(), &CustomModelMode::Browse);
}

/// Scenario: Cancelling the delete confirmation keeps the custom model
#[test]
fn delete_confirm_no_cancels() {
    // @step Given the model selector is showing a profile section with a custom model
    let mut v = expanded_profile_view();
    // @step And the cursor is on that custom model row
    v.handle_key(key(KeyCode::Down));
    // @step When I press "d"
    v.handle_key(key(KeyCode::Char('d')));
    // @step Then a delete confirmation shows the model display name and profile name
    assert!(matches!(
        v.custom_model_mode(),
        CustomModelMode::DeleteConfirm { .. }
    ));

    // @step When I press "n"
    let out = v.handle_key(key(KeyCode::Char('n')));
    // @step Then no custom model is deleted
    assert!(!matches!(out, ModelSelectorEvent::Emit(_)));
    // @step And I am returned to the browse list
    assert_eq!(v.custom_model_mode(), &CustomModelMode::Browse);
}

/// Scenario: The open form intercepts keys that are browse shortcuts
#[test]
fn form_intercepts_browse_shortcuts() {
    // @step Given the Add Custom Model form is open with the Model ID field focused
    let mut v = profile_header_view();
    v.handle_key(key(KeyCode::Char('a')));
    assert_eq!(v.form().field_index, 0);

    // @step When I press "r"
    v.handle_key(key(KeyCode::Char('r')));
    // @step And I press "/"
    v.handle_key(key(KeyCode::Char('/')));

    // @step Then "r/" is typed into the Model ID field
    assert_eq!(v.form().id, "r/");
    // @step And neither a refresh nor a filter is triggered
    assert!(!v.is_refreshing());
    assert!(!v.filter_mode);
}

/// Scenario: Editing a custom model saves it in place under the same id
#[test]
fn edit_saves_in_place_under_same_id() {
    // @step Given the Edit Custom Model form is open for a custom model
    let mut v = expanded_profile_view();
    v.handle_key(key(KeyCode::Down));
    v.handle_key(key(KeyCode::Char('e')));
    assert!(matches!(
        v.custom_model_mode(),
        CustomModelMode::Edit { .. }
    ));

    // @step When I clear the Display Name field
    v.handle_key(key(KeyCode::Down)); // focus Display Name (index 1)
    for _ in 0.."mycustom".len() {
        v.handle_key(key(KeyCode::Backspace));
    }
    // @step And I press Enter
    let out = v.handle_key(key(KeyCode::Enter));

    // @step Then the custom model is saved in place under its original id with the updated display name
    match out {
        ModelSelectorEvent::Emit(Action::EditCustomModel {
            original_model_id,
            definition,
            ..
        }) => {
            assert_eq!(original_model_id, "mycustom");
            assert_eq!(definition.id, "mycustom");
            assert_eq!(definition.display_name, None);
        }
        other => panic!("expected Emit(EditCustomModel), got {other:?}"),
    }
    // @step And the form closes and the provider list is refreshed
    assert_eq!(v.custom_model_mode(), &CustomModelMode::Browse);
}

/// Render a view into a `width`x`height` TestBackend and return the buffer
/// text (one row per line) so footer/visible-text assertions are possible.
fn render_to_text(v: &mut ModelSelectorView, width: u16, height: u16) -> String {
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).expect("term");
    term.draw(|f| v.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Scenario: An open overlay shows only the form footer, not the browse footer
#[test]
fn open_overlay_shows_only_form_footer() {
    // @step Given the Add Custom Model form is open
    let mut v = profile_header_view();
    v.handle_key(key(KeyCode::Char('a')));
    assert!(matches!(v.custom_model_mode(), CustomModelMode::Add { .. }));

    // @step When the model selector is rendered
    let text = render_to_text(&mut v, 80, 24);

    // @step Then the footer shows the form hint "Enter save"
    assert!(
        text.contains("Enter save"),
        "form footer hint missing; got:\n{text}"
    );
    // @step And the browse hint "r Refresh" is not shown
    assert!(
        !text.contains("r Refresh"),
        "browse footer must not appear while the form is open; got:\n{text}"
    );
}

/// Scenario: Editing a custom model with no stored display name starts blank
#[test]
fn edit_with_no_stored_display_name_starts_blank() {
    // @step Given a custom model whose display label is identical to its id
    let mut v = expanded_profile_view();
    // expanded_profile_view's custom_model("mycustom") has display_name == id.

    // @step When I press "e" on that custom model row
    v.handle_key(key(KeyCode::Down)); // move from base model to the custom row
    v.handle_key(key(KeyCode::Char('e')));
    assert!(matches!(
        v.custom_model_mode(),
        CustomModelMode::Edit { .. }
    ));

    // @step Then the Edit form opens with the Display Name field blank
    assert_eq!(
        v.form().display_name,
        "",
        "display name must start blank when the label only echoes the id"
    );
    assert_eq!(v.form().id, "mycustom");
}

/// PROV-101 — Feature: spec/features/model-selector-no-auto-select.feature
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
        !matches!(out, ModelSelectorEvent::Emit(Action::ModelSelected(..))),
        "Enter must not silently select a model when nothing is highlighted, got {out:?}"
    );
}
