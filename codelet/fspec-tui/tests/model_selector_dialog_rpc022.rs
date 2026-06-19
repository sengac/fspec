//! RPC-022 — ModelSelectorDialog component unit tests.
//!
//! Feature: spec/features/rpc022-model-selector-dialog.feature
//!
//! Drives the Priority::Foreground modal dialog through its public
//! Component surface: `priority()`, `render()`, `handle_event()`,
//! `selected_index()`, `provider_count()`, `row_count()`, and the
//! test-only `take_pending_action()` accessor.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_fspec_tui::{
    Action, Compositor, EventResult, ModelSelectorDialog, Priority, MODEL_SELECTOR_DIALOG_ID,
};
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn render_to_string(dialog: &mut ModelSelectorDialog, w: u16, h: u16) -> String {
    use codelet_fspec_tui::Component;
    let backend = TestBackend::new(w, h);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        Component::render(dialog, frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf: Buffer = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn provider(key: &str, display: &str, models: Vec<ModelEntry>) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: display.to_string(),
        models,
        profile_name: None,
        is_unreachable: false,
    }
}

fn model(id: &str, display: &str, ctx: u32, reasoning: bool, vision: bool) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: display.to_string(),
        context_window: ctx,
        supports_reasoning: reasoning,
        supports_vision: vision,
        is_custom: false,
    }
}

/// Scenario: ModelSelectorDialog renders at Priority::Foreground
#[test]
fn model_selector_dialog_renders_at_priority_foreground() {
    use codelet_fspec_tui::Component;
    // @step Given a fresh ModelSelectorDialog with id "model-selector-dialog"
    let dialog = ModelSelectorDialog::new(SessionId::new("s-1"), Vec::new());
    assert_eq!(dialog.id(), MODEL_SELECTOR_DIALOG_ID);
    // @step When its priority() method is invoked
    let prio = dialog.priority();
    // @step Then the result is Priority::Foreground
    assert_eq!(prio, Priority::Foreground);
    // @step And Priority::Foreground has discriminant 900
    assert_eq!(Priority::Foreground as u32, 900);
    // @step And Priority::Foreground sorts strictly between Priority::High (800) and Priority::Critical (1000)
    assert!(Priority::High < Priority::Foreground);
    assert!(Priority::Foreground < Priority::Critical);
}

/// Scenario: ModelSelectorDialog renders via the tui-popup adapter pattern
#[test]
fn model_selector_dialog_renders_via_tui_popup_adapter_pattern() {
    // @step Given a ModelSelectorDialog seeded with two providers (anthropic with [opus-4.6], openai with [gpt-5.1-codex])
    let providers = vec![
        provider(
            "anthropic",
            "anthropic",
            vec![model("opus-4.6", "opus-4.6", 200_000, false, false)],
        ),
        provider(
            "openai",
            "openai",
            vec![model(
                "gpt-5.1-codex",
                "gpt-5.1-codex",
                200_000,
                true,
                false,
            )],
        ),
    ];
    let mut dialog = ModelSelectorDialog::new(SessionId::new("s-1"), providers);
    // @step When the dialog is rendered onto a 100x30 TestBackend
    let painted = render_to_string(&mut dialog, 100, 30);
    // @step Then the rendered buffer contains the substring "Select Model"
    assert!(painted.contains("Select Model"));
    // @step And the rendered buffer contains the substring "anthropic"
    assert!(painted.contains("anthropic"));
    // @step And the rendered buffer contains the substring "openai"
    assert!(painted.contains("openai"));
    // @step And the production source uses the shared dialog_theme renderer
    // (RPC-027 replaced the tui_popup::Popup adapter with dialog_theme::render_dialog;
    // the original RPC-022 contract was a SizedWidgetRef adapter, which has
    // been superseded.)
    let src = common::read_to_string_or_panic(
        &common::workspace_root()
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("model_selector_dialog.rs"),
    );
    assert!(
        src.contains("render_dialog"),
        "production source must use dialog_theme::render_dialog"
    );
    let rows_src = common::read_to_string_or_panic(
        &common::workspace_root()
            .join("fspec-tui")
            .join("src")
            .join("components")
            .join("model_selector_dialog_rows.rs"),
    );
    // @step And the production source does NOT define a hand-rolled centered_rect helper
    assert!(!src.contains("fn centered_rect"));
    assert!(!rows_src.contains("fn centered_rect"));
}

/// Scenario: Arrow keys navigate the flat provider+model list with wrap-around
#[test]
fn arrow_keys_navigate_flat_list_with_wrap_around() {
    use codelet_fspec_tui::Component;
    // @step Given a ModelSelectorDialog seeded with anthropic[opus-4.6] and openai[gpt-5.1-codex]
    let providers = vec![
        provider(
            "anthropic",
            "anthropic",
            vec![model("opus-4.6", "opus-4.6", 200_000, false, false)],
        ),
        provider(
            "openai",
            "openai",
            vec![model(
                "gpt-5.1-codex",
                "gpt-5.1-codex",
                200_000,
                true,
                false,
            )],
        ),
    ];
    let mut dialog = ModelSelectorDialog::new(SessionId::new("s-1"), providers);
    // @step And the dialog is initialised with selected_index = 0 (anthropic header)
    // The dialog's `new` snaps selected_index to the first SELECTABLE row
    // (skipping provider headers). The initial selectable row is the
    // opus-4.6 model row at index 1.
    let initial = dialog.selected_index();
    assert_eq!(
        initial, 1,
        "initial selectable row should be anthropic[opus-4.6]"
    );
    // @step When the user presses Down four times
    for _ in 0..4 {
        let _ = dialog.handle_event(&key(KeyCode::Down));
    }
    // @step Then selected_index wraps back to 0 after exhausting the visible list
    // The flat list is [header(0), opus-4.6(1), header(2), gpt-5.1-codex(3)].
    // Only rows 1 and 3 are selectable. Down x4 from row 1 walks:
    //   1 → 3 → 1 → 3 → 1 (wrap-around through selectable rows).
    let final_idx = dialog.selected_index();
    assert_eq!(
        final_idx, initial,
        "Down x4 must wrap back to the initial selectable row (got {final_idx})"
    );
}

/// Scenario: Enter on a model row emits Action::ModelSelected
#[test]
fn enter_on_a_model_row_emits_action_model_selected() {
    use codelet_fspec_tui::Component;
    // @step Given a ModelSelectorDialog seeded with anthropic[opus-4.6] and openai[gpt-5.1-codex]
    let providers = vec![
        provider(
            "anthropic",
            "anthropic",
            vec![model("opus-4.6", "opus-4.6", 200_000, false, false)],
        ),
        provider(
            "openai",
            "openai",
            vec![model(
                "gpt-5.1-codex",
                "gpt-5.1-codex",
                200_000,
                true,
                false,
            )],
        ),
    ];
    // @step And the dialog was constructed against SessionId::new("s-1")
    let mut dialog = ModelSelectorDialog::new(SessionId::new("s-1"), providers);
    // @step And selected_index points at the openai[gpt-5.1-codex] row
    // Walk forward (Down) from opus-4.6 (idx 1) to gpt-5.1-codex (idx 3).
    let _ = dialog.handle_event(&key(KeyCode::Down));
    assert_eq!(dialog.selected_index(), 3);
    // @step When the user presses Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));
    // @step Then handle_event returns EventResult::Consumed with a callback
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    // @step And the callback emits Action::ModelSelected(SessionId::new("s-1"), "openai", "gpt-5.1-codex")
    let action = dialog
        .take_pending_action()
        .expect("pending action must be set");
    match action {
        Action::ModelSelected(sid, provider, model_id) => {
            assert_eq!(sid, SessionId::new("s-1"));
            assert_eq!(provider, "openai");
            assert_eq!(model_id, "gpt-5.1-codex");
        }
        other => panic!("expected ModelSelected, got {other:?}"),
    }
    // @step And the callback removes the dialog from the Compositor via its id
    let mut compositor = Compositor::new();
    compositor.push(Box::new(ModelSelectorDialog::new(
        SessionId::new("s-1"),
        Vec::new(),
    )));
    callback(&mut compositor);
    assert!(
        !compositor.contains(MODEL_SELECTOR_DIALOG_ID),
        "callback must remove the dialog from the Compositor"
    );
}

/// Scenario: Esc dismisses the ModelSelectorDialog without side effects
#[test]
fn esc_dismisses_the_model_selector_dialog_without_side_effects() {
    use codelet_fspec_tui::Component;
    // @step Given a ModelSelectorDialog seeded with anthropic[opus-4.6]
    let providers = vec![provider(
        "anthropic",
        "anthropic",
        vec![model("opus-4.6", "opus-4.6", 200_000, false, false)],
    )];
    let mut dialog = ModelSelectorDialog::new(SessionId::new("s-1"), providers);
    // @step When the user presses Esc
    let result = dialog.handle_event(&key(KeyCode::Esc));
    // @step Then handle_event returns EventResult::Consumed with a callback
    let callback = match result {
        EventResult::Consumed(Some(cb)) => cb,
        _ => panic!("expected Consumed(Some(callback))"),
    };
    // @step And the callback removes the dialog from the Compositor via its id
    let mut compositor = Compositor::new();
    compositor.push(Box::new(ModelSelectorDialog::new(
        SessionId::new("s-1"),
        Vec::new(),
    )));
    callback(&mut compositor);
    assert!(!compositor.contains(MODEL_SELECTOR_DIALOG_ID));
    // @step And no Action::ModelSelected is emitted
    assert!(
        dialog.take_pending_action().is_none(),
        "Esc must not emit any pending action"
    );
}

/// Scenario: ModelSelectorDialog with zero providers shows a 'No providers available' hint
#[test]
fn dialog_with_zero_providers_shows_no_providers_available_hint() {
    use codelet_fspec_tui::Component;
    // @step Given a ModelSelectorDialog seeded with Vec::<ProviderInfo>::new()
    let mut dialog = ModelSelectorDialog::new(SessionId::new("s-1"), Vec::new());
    // @step When the dialog is rendered onto a 80x24 TestBackend
    let painted = render_to_string(&mut dialog, 80, 24);
    // @step Then the rendered buffer contains the substring "No providers available"
    assert!(painted.contains("No providers available"));
    // @step And pressing Enter on the empty list emits NO Action::ModelSelected
    let _ = dialog.handle_event(&key(KeyCode::Enter));
    assert!(
        dialog.take_pending_action().is_none(),
        "Enter on empty list must not emit ModelSelected"
    );
}

/// Scenario: ModelSelectorDialog footer documents the out-of-scope custom model creation
#[test]
fn footer_documents_out_of_scope_custom_model_creation() {
    // @step Given a ModelSelectorDialog with at least one provider
    let providers = vec![provider(
        "anthropic",
        "anthropic",
        vec![model("opus-4.6", "opus-4.6", 200_000, false, false)],
    )];
    let mut dialog = ModelSelectorDialog::new(SessionId::new("s-1"), providers);
    // @step When the dialog is rendered onto a 100x30 TestBackend
    let painted = render_to_string(&mut dialog, 100, 30);
    // @step Then the rendered buffer contains the substring "Custom models: not yet supported"
    assert!(painted.contains("Custom models: not yet supported"));
}

/// Scenario: Each model row paints capability badges [R] [V] [Nk]
#[test]
fn each_model_row_paints_capability_badges() {
    // @step Given a ModelSelectorDialog seeded with anthropic[opus-4.6] where opus-4.6 supports reasoning AND vision AND has context_window 200000
    let providers = vec![provider(
        "anthropic",
        "anthropic",
        vec![model("opus-4.6", "opus-4.6", 200_000, true, true)],
    )];
    let mut dialog = ModelSelectorDialog::new(SessionId::new("s-1"), providers);
    // @step When the dialog is rendered
    let painted = render_to_string(&mut dialog, 100, 30);
    let opus_row = painted
        .lines()
        .find(|l| l.contains("opus-4.6"))
        .expect("opus-4.6 row not found");
    // @step Then the row for opus-4.6 contains the substring "[R]"
    assert!(opus_row.contains("[R]"));
    // @step And the row for opus-4.6 contains the substring "[V]"
    assert!(opus_row.contains("[V]"));
    // @step And the row for opus-4.6 contains the substring "[200k]"
    assert!(opus_row.contains("[200k]"));
}

/// Scenario: model_selector_dialog.rs stays under 300 lines
#[test]
fn model_selector_dialog_rs_stays_under_300_lines() {
    // @step Given the file codelet/fspec-tui/src/components/model_selector_dialog.rs after RPC-022 lands
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("components")
        .join("model_selector_dialog.rs");
    // @step When a test counts the line-count of the file
    let lines = common::read_to_string_or_panic(&path).lines().count();
    // @step Then the file has fewer than 300 lines
    assert!(lines < 300, "model_selector_dialog.rs has {lines} lines");
}
