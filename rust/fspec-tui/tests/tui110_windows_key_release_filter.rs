//! TUI-110 — Windows key-doubling fix: central Press-only key filter.
//!
//! Feature: spec/features/windows-key-release-duplication.feature
//!
//! On Windows (cmd / Windows Terminal) crossterm reports BOTH
//! `KeyEventKind::Press` and `KeyEventKind::Release` for every key
//! (ratatui#347, crossterm#772). The App event path must process ONLY
//! `Press` events so each keystroke is registered exactly once.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::disconnect_dialog::DisconnectDialog;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

mod common;
use common::MockBackend;

/// Build a KeyEvent with an explicit `kind` (crossterm's `KeyEvent::new`
/// defaults to `Press`).
fn key_with_kind(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind,
        state: crossterm::event::KeyEventState::empty(),
    }
}

fn make_unit(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// Drain the App's action bus, dispatching every queued Action back into
/// the App (mirrors `AppTestHarness::drain_pending`). BoardView key
/// handlers emit Actions onto the bus; `App::dispatch` is what applies
/// them to the BoardStore.
fn drain_actions(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Fresh App with three backlog work units and the board focused.
/// Selection starts at index 0.
fn board_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.board_store_mut()
        .replace_work_units(vec![make_unit("AUTH-001"), make_unit("AUTH-002"), make_unit("AUTH-003")]);
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    (app, mock)
}

/// Scenario: A single Down-arrow press moves the board selection exactly one row
#[test]
fn single_down_arrow_press_moves_the_board_selection_exactly_one_row() {
    // @step Given the TUI is running in the board view with a work unit selected
    let (mut app, _mock) = board_app();
    // @step When a Down-arrow key event with kind Press arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Down, KeyEventKind::Press)));
    drain_actions(&mut app);
    // @step And a Down-arrow key event with kind Release arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Down, KeyEventKind::Release)));
    drain_actions(&mut app);
    // @step Then the board selection has moved down exactly one row
    assert_eq!(
        app.board_store().selected_index_for("backlog"),
        1,
        "Press must move 0→1 and the Release must be ignored (not 0→2)"
    );
}

/// Scenario: A key release event is dropped by the central app event loop
#[test]
fn a_key_release_event_is_dropped_by_the_central_app_event_loop() {
    // @step Given the TUI is running in the board view with a work unit selected
    let (mut app, _mock) = board_app();
    // @step When a Down-arrow key event with kind Release arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Down, KeyEventKind::Release)));
    drain_actions(&mut app);
    // @step Then the board selection has not moved
    assert_eq!(
        app.board_store().selected_index_for("backlog"),
        0,
        "a lone Release must not move the selection"
    );
}

/// Scenario: A key repeat event is dropped by the central app event loop
#[test]
fn a_key_repeat_event_is_dropped_by_the_central_app_event_loop() {
    // @step Given the TUI is running in the board view with a work unit selected
    let (mut app, _mock) = board_app();
    // @step When a Down-arrow key event with kind Repeat arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Down, KeyEventKind::Repeat)));
    drain_actions(&mut app);
    // @step Then the board selection has not moved
    assert_eq!(
        app.board_store().selected_index_for("backlog"),
        0,
        "a lone Repeat must not move the selection"
    );
}

/// Scenario: A single ? press opens the Help dialog exactly once
#[test]
fn a_single_question_press_opens_the_help_dialog_exactly_once() {
    // @step Given the TUI is running in the board view with no dialog open
    let (mut app, _mock) = board_app();
    assert!(!app.compositor().contains("help-dialog"));
    // @step When a ? key event with kind Press arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Char('?'), KeyEventKind::Press)));
    drain_actions(&mut app);
    // @step And a ? key event with kind Release arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Char('?'), KeyEventKind::Release)));
    drain_actions(&mut app);
    // @step Then the Help dialog is open
    assert!(
        app.compositor().contains("help-dialog"),
        "Press must open the Help dialog"
    );
    // @step And the Help dialog is the only dialog open
    assert_eq!(
        app.compositor().len(),
        1,
        "exactly one compositor layer (the Help dialog) must be open"
    );
}

/// Scenario: A ? release event does not open the Help dialog
#[test]
fn a_question_release_event_does_not_open_the_help_dialog() {
    // @step Given the TUI is running in the board view with no dialog open
    let (mut app, _mock) = board_app();
    // @step When a ? key event with kind Release arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Char('?'), KeyEventKind::Release)));
    drain_actions(&mut app);
    // @step Then no dialog is open
    assert!(
        app.compositor().is_empty(),
        "a lone ? Release must not open the Help dialog"
    );
}

/// Scenario: A ? repeat event does not open the Help dialog
#[test]
fn a_question_repeat_event_does_not_open_the_help_dialog() {
    // @step Given the TUI is running in the board view with no dialog open
    let (mut app, _mock) = board_app();
    // @step When a ? key event with kind Repeat arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Char('?'), KeyEventKind::Repeat)));
    drain_actions(&mut app);
    // @step Then no dialog is open
    assert!(
        app.compositor().is_empty(),
        "a lone ? Repeat must not open the Help dialog"
    );
}

/// Scenario: Press key events still flow to views and dialogs unchanged
#[test]
fn press_key_events_still_flow_to_views_and_dialogs_unchanged() {
    // @step Given the TUI is running in the board view with a work unit selected
    let (mut app, _mock) = board_app();
    // @step When a Down-arrow key event with kind Press arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Down, KeyEventKind::Press)));
    drain_actions(&mut app);
    // @step Then the board selection has moved down exactly one row
    assert_eq!(
        app.board_store().selected_index_for("backlog"),
        1,
        "a Press Down must still move the selection 0→1"
    );
}

/// Scenario: A repeat ? event does not trigger the app-level Help shortcut
#[test]
fn a_repeat_question_event_does_not_trigger_the_app_level_help_shortcut() {
    // @step Given the TUI is running in the board view with no dialog open
    let (mut app, _mock) = board_app();
    // @step When a ? key event with kind Repeat arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Char('?'), KeyEventKind::Repeat)));
    drain_actions(&mut app);
    // @step Then no dialog is open
    assert!(
        app.compositor().is_empty(),
        "a ? Repeat must not open the Help dialog"
    );
}

/// Scenario: A repeat q event does not quit the Disconnect dialog
#[test]
fn a_repeat_q_event_does_not_quit_the_disconnect_dialog() {
    // @step Given the Disconnect dialog is open
    let (mut app, _mock) = board_app();
    app.compositor_mut()
        .push(Box::new(DisconnectDialog::new()));
    assert!(app.compositor().contains("disconnect-dialog"));
    assert!(!app.should_quit());
    // @step When a q key event with kind Repeat arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Char('q'), KeyEventKind::Repeat)));
    drain_actions(&mut app);
    // @step Then the app has not quit
    assert!(
        !app.should_quit(),
        "a q Repeat must not quit the app"
    );
    // @step And the Disconnect dialog is still open
    assert!(
        app.compositor().contains("disconnect-dialog"),
        "the Disconnect dialog must remain open after a q Repeat"
    );
}

/// Supplementary: a Press q still quits the Disconnect dialog (no regression).
#[test]
fn a_press_q_event_still_quits_the_disconnect_dialog() {
    // @step Given the Disconnect dialog is open
    let (mut app, _mock) = board_app();
    app.compositor_mut()
        .push(Box::new(DisconnectDialog::new()));
    // @step When a q key event with kind Press arrives at the app event loop
    let _ = app.handle_event(&Event::Key(key_with_kind(KeyCode::Char('q'), KeyEventKind::Press)));
    drain_actions(&mut app);
    // @step Then the app has quit and the dialog is gone
    assert!(app.should_quit(), "a q Press must quit the app");
    assert!(
        !app.compositor().contains("disconnect-dialog"),
        "the Disconnect dialog must be removed on quit"
    );
}

/// Supplementary: Paste events are unaffected by the Press-only key filter.
#[test]
fn paste_events_are_unaffected_by_the_press_only_key_filter() {
    // @step Given the TUI is running in the board view with no dialog open
    let (mut app, _mock) = board_app();
    // @step When a Paste event arrives at the app event loop
    let result = app.handle_event(&Event::Paste("hello".to_string()));
    // @step Then the event is not treated as a key and no dialog opens
    assert!(!app.compositor().contains("help-dialog"));
    assert!(
        !matches!(result, codelet_fspec_tui::EventResult::Consumed(_)),
        "a Paste on the empty board is ignored by all stages"
    );
}
