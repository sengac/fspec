//! RPC-373 — Wire D key on board to open FOUNDATION.md in browser.
//!
//! Feature: spec/features/rust-board-open-foundation.feature
//!
//! Board-key tests. The board's `D`/`d` arm only emits
//! `Action::OpenFoundation`; TUI-120 opens it from the selected workspace
//! (rpc tests in tui120_foundation_viewer_and_fspec_runner_follow_the_selected_workspace.rs).

use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::{Action, BoardStore, BoardView, EventResult, Theme};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::unbounded_channel;

fn fresh() -> (BoardView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

#[test]
fn pressing_uppercase_d_opens_the_foundation_document() -> TestResult {
    // @step Given the board view is focused
    let (view, mut rx) = fresh();
    let store = BoardStore::default();

    // @step When I press the uppercase D key
    let event = Event::Key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE));
    let result = view.handle_event(&event, &store);

    // @step Then the open-foundation action is emitted
    let action = rx.try_recv().or_fail("Action::OpenFoundation on bus")?;
    assert!(
        matches!(action, Action::OpenFoundation),
        "expected OpenFoundation, got {action:?}"
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
    Ok(())
}

#[test]
fn pressing_lowercase_d_opens_the_foundation_document() -> TestResult {
    // @step Given the board view is focused
    let (view, mut rx) = fresh();
    let store = BoardStore::default();

    // @step When I press the lowercase d key
    let event = Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    let result = view.handle_event(&event, &store);

    // @step Then the open-foundation action is emitted
    let action = rx.try_recv().or_fail("Action::OpenFoundation on bus")?;
    assert!(
        matches!(action, Action::OpenFoundation),
        "expected OpenFoundation, got {action:?}"
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
    Ok(())
}
