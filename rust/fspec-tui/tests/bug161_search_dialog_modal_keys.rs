//! BUG-161 — Board search dialog is a true modal: unhandled keys must not
//! leak through to the BoardView behind it.
//!
//! Feature: spec/features/board-search-dialog-modal-keyboard-blocking.feature
//!
//! ACDD TESTING phase: these tests assert the BUG-161 behaviour — while the
//! WorkUnitSearchDialog is open, every key it does not explicitly handle
//! (modifier-chorded keys, unmodified Left/Right, and any other unhandled
//! key) is CONSUMED as a no-op so the board behind the modal is frozen.
//! The dialog's own explicit arms (Esc, Tab, Backspace, printable chars,
//! Up/Down/PageUp/PageDown/Home/End, Enter) keep their current behaviour.
//!
//! RED note: the catch-all `_ => EventResult::ignored()` and the SHIFT/CTRL
//! modifier guard currently return `Ignored`, so modifier-chorded keys
//! (Shift+Right, Shift+?) and unmodified Left/Right fall through the
//! Compositor to the BoardView. The `modifier_chorded_keys_*`,
//! `the_help_key_*` and `unmodified_arrow_keys_*` tests are RED for those
//! keys until the BUG-161 implementation lands. The printable-char tests
//! (j/k/h/l, f/c/d/a/.) are regression guards — those keys are already
//! consumed by the dialog's `Char` arm.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::{
    Action, BoardStore, Component, Compositor, EventResult, WorkUnitSearchDialog,
    WORK_UNIT_SEARCH_DIALOG_ID,
};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::UnboundedReceiver;

fn wu(id: &str, title: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: title.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn char_key(c: char) -> Event {
    Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// A SHIFT-chorded key (the way crossterm encodes `?`, Shift+Right, ...).
fn shift_key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT))
}

/// A dialog wired to an action bus so tests can assert nothing was emitted.
fn dialog_with_bus(
    units: Vec<WorkUnitInfo>,
) -> (WorkUnitSearchDialog, UnboundedReceiver<Action>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (WorkUnitSearchDialog::new(units).with_action_tx(tx), rx)
}

/// A board store with AUTH-001 selected in the backlog column.
fn board_with_selection() -> BoardStore {
    let mut store = BoardStore::default();
    store.replace_work_units(vec![wu("AUTH-001", "User login")]);
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    store
}

/// Scenario: Board navigation keys are consumed while the dialog is open
#[test]
fn board_navigation_keys_are_consumed_while_the_dialog_is_open() {
    // @step Given the work-unit search dialog is open with zero matches
    let (mut dialog, _rx) = dialog_with_bus(vec![wu("AUTH-001", "User login")]);
    for c in "zzz".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }
    assert_eq!(dialog.matches(), Vec::<String>::new(), "query 'zzz' must yield zero matches");
    let store = board_with_selection();

    // @step When I press the "j" key
    for c in ['j', 'k', 'h', 'l'] {
        let result = dialog.handle_event(&char_key(c));

        // @step Then the dialog consumes the key
        assert!(
            result.is_consumed(),
            "key '{c}' must be CONSUMED by the dialog (modal contract)"
        );

        // @step And the board selection is unchanged
        assert_eq!(
            store.selected_work_unit().map(|u| u.id.as_str()),
            Some("AUTH-001"),
            "the board selection must stay frozen while the dialog is open"
        );
    }
}

/// Scenario: Board view-opening shortcuts are consumed while the dialog is open
#[test]
fn board_view_opening_shortcuts_are_consumed_while_the_dialog_is_open() {
    // @step Given the work-unit search dialog is open
    let (mut dialog, mut rx) = dialog_with_bus(vec![wu("AUTH-001", "User login")]);

    // @step When I press one of the keys "f", "c", "d", "a", "."
    for c in ['f', 'c', 'd', 'a', '.'] {
        let result = dialog.handle_event(&char_key(c));

        // @step Then the dialog consumes the key
        assert!(
            result.is_consumed(),
            "key '{c}' must be CONSUMED by the dialog (modal contract)"
        );

        // @step And no board action is emitted
        assert!(
            rx.try_recv().is_err(),
            "pressing '{c}' must not emit any Action on the bus"
        );
    }
}

/// Scenario: Modifier-chorded keys are consumed while the dialog is open
#[test]
fn modifier_chorded_keys_are_consumed_while_the_dialog_is_open() {
    // @step Given the work-unit search dialog is open
    let (mut dialog, mut rx) = dialog_with_bus(vec![wu("AUTH-001", "User login")]);

    // @step When I press Shift+Right
    let result = dialog.handle_event(&shift_key(KeyCode::Right));

    // @step Then the dialog consumes the key
    assert!(
        result.is_consumed(),
        "Shift+Right must be CONSUMED by the dialog (modifier guard)"
    );

    // @step And no agent view is opened
    assert!(
        rx.try_recv().is_err(),
        "Shift+Right must not emit OpenAgentView on the bus"
    );
}

/// Scenario: The help key is consumed while the dialog is open
#[test]
fn the_help_key_is_consumed_while_the_dialog_is_open() {
    // @step Given the work-unit search dialog is open
    let (mut dialog, mut rx) = dialog_with_bus(vec![wu("AUTH-001", "User login")]);

    // @step When I press the "?" key
    // crossterm encodes '?' as a SHIFT-chorded Char.
    let result = dialog.handle_event(&shift_key(KeyCode::Char('?')));

    // @step Then the dialog consumes the key
    assert!(
        result.is_consumed(),
        "'?' (Shift+Char) must be CONSUMED by the dialog (modifier guard)"
    );

    // @step And the help dialog does not open
    assert!(
        rx.try_recv().is_err(),
        "'?' must not reach the App-level HelpDialog handler"
    );
}

/// Scenario: Unmodified arrow keys are consumed while the dialog is open
/// (BUG-161: unmodified Left/Right reach the dialog's catch-all arm — they
/// are not printable chars — and must be consumed; currently they leak to
/// the board's FocusPrev/NextColumn.)
#[test]
fn unmodified_arrow_keys_left_right_are_consumed_while_the_dialog_is_open() {
    // @step Given the work-unit search dialog is open
    let (mut dialog, _rx) = dialog_with_bus(vec![wu("AUTH-001", "User login")]);
    let store = board_with_selection();

    // @step When I press Left and Right
    for code in [KeyCode::Left, KeyCode::Right] {
        let result = dialog.handle_event(&key(code));

        // @step Then the dialog consumes the key
        assert!(
            result.is_consumed(),
            "{code:?} must be CONSUMED by the dialog catch-all (modal contract)"
        );
    }

    // @step And the board column focus is unchanged
    assert_eq!(store.focused_column(), "backlog", "the board must stay frozen");
}

/// Scenario: Enter with zero matches is consumed and does not enter a work unit
#[test]
fn enter_with_zero_matches_is_consumed_and_does_not_enter_a_work_unit() {
    // @step Given the work-unit search dialog is open with zero matches
    let (mut dialog, mut rx) = dialog_with_bus(vec![wu("AUTH-001", "User login")]);
    for c in "zzz".chars() {
        let _ = dialog.handle_event(&char_key(c));
    }

    // @step When I press Enter
    let result = dialog.handle_event(&key(KeyCode::Enter));

    // @step Then the dialog consumes the key
    assert!(
        result.is_consumed(),
        "Enter with zero matches must be CONSUMED"
    );
    assert!(
        matches!(result, EventResult::Consumed(None)),
        "Enter with zero matches must be a no-op (no remove callback)"
    );

    // @step And no work unit is entered
    assert!(
        rx.try_recv().is_err(),
        "Enter with zero matches must not emit SelectWorkUnit on the bus"
    );
}

/// Scenario: The dialog's own keys still work while the board is frozen
#[test]
fn the_dialogs_own_keys_still_work_while_the_board_is_frozen() {
    // @step Given the work-unit search dialog is open in Id mode with a match list
    let (mut dialog, mut rx) = dialog_with_bus(vec![
        wu("AUTH-001", "Auth one"),
        wu("AUTH-002", "Auth two"),
    ]);
    assert_eq!(dialog.mode_label(), "id");
    assert_eq!(dialog.matches().len(), 2);

    // @step When I press Tab
    let tab_result = dialog.handle_event(&key(KeyCode::Tab));

    // @step Then the dialog shows the Title search mode
    assert!(tab_result.is_consumed());
    assert_eq!(dialog.mode_label(), "title");

    // @step When I type "auth" into the dialog
    for c in "auth".chars() {
        let r = dialog.handle_event(&char_key(c));
        assert!(r.is_consumed(), "printable chars must stay consumed");
    }
    // @step Then the dialog's query is "auth"
    // In Title mode both "Auth one"/"Auth two" match the query "auth".
    assert_eq!(
        dialog.matches(),
        vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
        "the query must have been edited by the typed characters"
    );

    // Backspace still edits the query: narrow to zero matches, then widen
    // back to two.
    let _ = dialog.handle_event(&char_key('x'));
    assert_eq!(dialog.matches(), Vec::<String>::new(), "'authx' matches nothing");
    let bs = dialog.handle_event(&key(KeyCode::Backspace));
    assert!(bs.is_consumed());
    assert_eq!(
        dialog.matches(),
        vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
        "backspace must pop one query char ('auth' matches both again)"
    );

    // Enter with a match emits SelectWorkUnit and closes the dialog.
    let enter = dialog.handle_event(&key(KeyCode::Enter));
    let EventResult::Consumed(Some(_callback)) = enter else {
        panic!("Enter with a match must carry the remove callback");
    };
    let action = rx.try_recv().expect("SelectWorkUnit on the bus");
    assert!(
        matches!(action, Action::SelectWorkUnit(ref id) if id == "AUTH-001"),
        "Enter must emit SelectWorkUnit(AUTH-001)"
    );

    // @step When I press Esc
    // @step Then the work-unit search dialog is closed
    let esc = dialog.handle_event(&key(KeyCode::Esc));
    let EventResult::Consumed(Some(esc_callback)) = esc else {
        panic!("Esc must carry the remove callback");
    };
    let mut compositor = Compositor::new();
    compositor.push(Box::new(dialog));
    assert!(compositor.contains(WORK_UNIT_SEARCH_DIALOG_ID));
    esc_callback(&mut compositor);
    assert!(
        !compositor.contains(WORK_UNIT_SEARCH_DIALOG_ID),
        "the remove callback must drop the dialog layer"
    );
}
