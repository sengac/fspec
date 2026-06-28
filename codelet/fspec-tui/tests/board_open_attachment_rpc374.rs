//! RPC-374 — Wire A key on board to open attachment picker and browser.
//!
//! Feature: spec/features/rust-board-open-attachment.feature
//!
//! Board-key + picker + URL-building tests. The board's `A`/`a` arm always
//! consumes the key and emits `Action::OpenAttachmentPicker` only when the
//! selected work unit has attachments. The picker lists basenames; selecting
//! an attachment opens it at the percent-encoded viewer URL built by the pure
//! `App::attachment_url` / `App::attachment_target` seam so no real browser
//! launches in tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    Action, App, AttachmentPickerDialog, BoardStore, BoardView, EventResult, FspecBackend, Theme,
};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::unbounded_channel;

mod common;

use common::MockBackend;

fn wu(id: &str, status: &str, attachments: Vec<String>) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments,
        last_state_change_at: None,
    }
}

fn fresh() -> (BoardView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

fn store_with(attachments: Vec<String>) -> BoardStore {
    let mut store = BoardStore::default();
    store.replace_work_units(vec![wu("RPC-001", "implementing", attachments)]);
    store.set_focused_column("implementing");
    store.set_selected_index_for("implementing", 0);
    store
}

#[test]
fn pressing_a_on_a_card_with_attachments_opens_the_picker() {
    // @step Given a card is selected that has two attachments
    let (view, mut rx) = fresh();
    let store = store_with(vec![
        "spec/attachments/RPC-001/design.md".to_string(),
        "spec/attachments/RPC-001/a b.md".to_string(),
    ]);

    // @step When I press the uppercase A key
    let event = Event::Key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));
    let result = view.handle_event(&event, &store);

    // @step Then the open-attachment-picker action is emitted
    let action = rx.try_recv().expect("Action::OpenAttachmentPicker on bus");
    assert!(
        matches!(action, Action::OpenAttachmentPicker),
        "expected OpenAttachmentPicker, got {action:?}"
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
}

#[test]
fn pressing_a_on_a_card_with_no_attachments_is_a_silent_no_op() {
    // @step Given a card is selected that has no attachments
    let (view, mut rx) = fresh();
    let store = store_with(Vec::new());

    // @step When I press the uppercase A key
    let event = Event::Key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));
    let result = view.handle_event(&event, &store);

    // @step Then no open-attachment-picker action is emitted
    assert!(rx.try_recv().is_err(), "expected no action on bus");

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
}

#[test]
fn pressing_lowercase_a_behaves_the_same_as_uppercase_a() {
    // @step Given a card is selected that has two attachments
    let (view, mut rx) = fresh();
    let store = store_with(vec![
        "spec/attachments/RPC-001/design.md".to_string(),
        "spec/attachments/RPC-001/a b.md".to_string(),
    ]);

    // @step When I press the lowercase a key
    let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    let result = view.handle_event(&event, &store);

    // @step Then the open-attachment-picker action is emitted
    let action = rx.try_recv().expect("Action::OpenAttachmentPicker on bus");
    assert!(
        matches!(action, Action::OpenAttachmentPicker),
        "expected OpenAttachmentPicker, got {action:?}"
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
}

#[test]
fn the_picker_lists_the_selected_work_units_attachments() {
    // @step Given a work unit with attachments "spec/attachments/RPC-001/design.md" and "spec/attachments/RPC-001/a b.md"
    let attachments = vec![
        "spec/attachments/RPC-001/design.md".to_string(),
        "spec/attachments/RPC-001/a b.md".to_string(),
    ];

    // @step When the attachment picker is built for that work unit
    let dialog = AttachmentPickerDialog::new(attachments);

    // @step Then the picker lists two entries
    let labels = dialog.row_labels();
    assert_eq!(labels.len(), 2, "expected two entries, got {labels:?}");

    // @step And the entries show the basenames "design.md" and "a b.md"
    assert_eq!(labels, vec!["design.md".to_string(), "a b.md".to_string()]);
}

#[test]
fn selecting_an_attachment_opens_it_at_the_encoded_viewer_url() {
    // @step Given the attachment viewer server is running on a known port
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.set_viewer_port_for_test(Some(53999));

    // @step When I select the attachment "spec/attachments/RPC-001/a b.md"
    let target = app.attachment_target("spec/attachments/RPC-001/a b.md");

    // @step Then the target is the percent-encoded view URL for that attachment on that port
    assert_eq!(
        target.as_deref(),
        Some("http://127.0.0.1:53999/view/spec/attachments/RPC-001/a%20b.md")
    );
    assert_eq!(
        App::attachment_url(53999, "spec/attachments/RPC-001/a b.md"),
        "http://127.0.0.1:53999/view/spec/attachments/RPC-001/a%20b.md"
    );
}

#[test]
fn selecting_an_attachment_is_a_safe_no_op_when_the_viewer_server_is_unavailable() {
    // @step Given the attachment viewer server is not running
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    app.set_viewer_port_for_test(None);

    // @step When I select an attachment
    let target = app.attachment_target("spec/attachments/RPC-001/a b.md");

    // @step Then there is no target and no browser is launched
    assert!(target.is_none(), "expected no target, got {target:?}");
    // Dispatching OpenAttachment with no port must not panic and must not
    // launch a browser (the open::that call is gated behind the Some branch
    // AND behind Handle::try_current(), which is None in this non-async test).
    app.dispatch(Action::OpenAttachment(
        "spec/attachments/RPC-001/a b.md".to_string(),
    ));
}
