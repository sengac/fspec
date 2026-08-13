//! RPC-373 — Wire D key on board to open FOUNDATION.md in browser.
//!
//! Feature: spec/features/rust-board-open-foundation.feature
//!
//! Board-key + URL-building tests. The board's `D`/`d` arm only emits
//! `Action::OpenFoundation`; URL building lives in the pure
//! `App::foundation_url` / `App::foundation_target` seam so no real browser
//! launches in tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, BoardStore, BoardView, EventResult, FspecBackend, Theme};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::unbounded_channel;

mod common;

use common::MockBackend;

fn fresh() -> (BoardView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

#[test]
fn pressing_uppercase_d_opens_the_foundation_document() {
    // @step Given the board view is focused
    let (view, mut rx) = fresh();
    let store = BoardStore::default();

    // @step When I press the uppercase D key
    let event = Event::Key(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE));
    let result = view.handle_event(&event, &store);

    // @step Then the open-foundation action is emitted
    let action = rx.try_recv().expect("Action::OpenFoundation on bus");
    assert!(
        matches!(action, Action::OpenFoundation),
        "expected OpenFoundation, got {action:?}"
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
}

#[test]
fn pressing_lowercase_d_opens_the_foundation_document() {
    // @step Given the board view is focused
    let (view, mut rx) = fresh();
    let store = BoardStore::default();

    // @step When I press the lowercase d key
    let event = Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    let result = view.handle_event(&event, &store);

    // @step Then the open-foundation action is emitted
    let action = rx.try_recv().expect("Action::OpenFoundation on bus");
    assert!(
        matches!(action, Action::OpenFoundation),
        "expected OpenFoundation, got {action:?}"
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
}

#[test]
fn the_foundation_document_opens_at_the_viewer_url_when_the_server_is_running() {
    // @step Given the attachment viewer server is running on a known port
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.set_viewer_port_for_test(Some(53999));

    // @step When the open-foundation action resolves its target
    let target = app.foundation_target();

    // @step Then the target is the FOUNDATION.md view URL on that port
    assert_eq!(
        target.as_deref(),
        Some("http://127.0.0.1:53999/view/spec/FOUNDATION.md")
    );
    assert_eq!(
        App::foundation_url(53999),
        "http://127.0.0.1:53999/view/spec/FOUNDATION.md"
    );
}

#[test]
fn pressing_d_is_a_safe_no_op_when_the_viewer_server_is_unavailable() {
    // @step Given the attachment viewer server is not running
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.set_viewer_port_for_test(None);

    // @step When the open-foundation action resolves its target
    let target = app.foundation_target();

    // @step Then there is no target and no browser is launched
    assert!(target.is_none(), "expected no target, got {target:?}");
    // Dispatching OpenFoundation with no port must not panic and must not
    // launch a browser (the open::that call is gated behind the Some branch).
    app.dispatch(Action::OpenFoundation);
}
