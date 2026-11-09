//! RPC-374 — Wire A key on board to open attachment picker and browser.
//!
//! Feature: spec/features/rust-board-open-attachment.feature
//!
//! Board-key + picker + open tests. The board's `A`/`a` arm always
//! consumes the key and emits `Action::OpenAttachmentPicker` only when the
//! selected work unit has attachments. The picker lists basenames; BUG-469:
//! selecting an attachment reads it from the daemon for the selected
//! workspace and opens it in the viewer (a recording launcher, so no real
//! browser launches in tests).

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
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
fn pressing_a_on_a_card_with_attachments_opens_the_picker() -> TestResult {
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
    let action = rx
        .try_recv()
        .or_fail("Action::OpenAttachmentPicker on bus")?;
    assert!(
        matches!(action, Action::OpenAttachmentPicker),
        "expected OpenAttachmentPicker, got {action:?}"
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
    Ok(())
}

#[test]
fn pressing_a_on_a_card_with_no_attachments_leaves_it_to_the_app_to_say_so() -> TestResult {
    // @step Given a card is selected that has no attachments
    let (view, mut rx) = fresh();
    let store = store_with(Vec::new());

    // @step When I press the uppercase A key
    let event = Event::Key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));
    let result = view.handle_event(&event, &store);

    // @step Then the open-attachment-picker action is emitted, and the App says the card has no attachments (TUI-129)
    let action = rx
        .try_recv()
        .or_fail("Action::OpenAttachmentPicker on bus")?;
    assert!(
        matches!(action, Action::OpenAttachmentPicker),
        "expected OpenAttachmentPicker, got {action:?}"
    );
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::WorkUnitsLoaded(vec![wu(
        "RPC-001",
        "implementing",
        Vec::new(),
    )]));
    app.board_store_mut().set_focused_column("implementing");
    app.board_store_mut()
        .set_selected_index_for("implementing", 0);
    app.dispatch(action);
    assert!(
        app.compositor().contains("session-storage-dialog"),
        "the App must say the card has no attachments; layers={:?}",
        app.compositor().layer_ids()
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
    Ok(())
}

#[test]
fn pressing_lowercase_a_behaves_the_same_as_uppercase_a() -> TestResult {
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
    let action = rx
        .try_recv()
        .or_fail("Action::OpenAttachmentPicker on bus")?;
    assert!(
        matches!(action, Action::OpenAttachmentPicker),
        "expected OpenAttachmentPicker, got {action:?}"
    );

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(None)));
    Ok(())
}

#[test]
fn the_picker_lists_the_selected_work_units_attachments() -> TestResult {
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
    Ok(())
}

/// The URLs a recording launcher was asked to open.
type Launched = Arc<std::sync::Mutex<Vec<String>>>;

/// BUG-469: an App over `mock` whose launcher records every URL.
fn recording_app(
    mock: &Arc<MockBackend>,
) -> Result<(App, Launched), codelet_test_helpers::TestError> {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    let launched = Arc::new(std::sync::Mutex::new(Vec::new()));
    let record = Arc::clone(&launched);
    app.set_board_attachment_launcher_for_test(Arc::new(move |url: &str| {
        record
            .lock()
            .map_err(|_| "launcher lock poisoned".to_string())?
            .push(url.to_owned());
        Ok(())
    }));
    Ok((app, launched))
}

async fn settle(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    for _ in 0..6 {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        // TEST-042: scheduler rounds, not a time window: on the
        // current-thread test runtime every ready task (a spawned backend
        // call, a subscriber forwarding a push) runs before this resumes.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
        }
    }
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn selecting_an_attachment_opens_it_from_the_selected_workspace_on_the_daemon() -> TestResult
{
    // @step Given the board shows workspace "beta"
    let mock = Arc::new(MockBackend::new());
    mock.script_card_attachment(Ok(b"# Beta design\n".to_vec()));
    let (mut app, launched) = recording_app(&mock)?;
    app.board_store_mut().set_workspaces(vec![
        codelet_rpc_types::WorkspaceEntry {
            path: "/work/alpha".into(),
            name: "alpha".into(),
            start: true,
        },
        codelet_rpc_types::WorkspaceEntry {
            path: "/work/beta".into(),
            name: "beta".into(),
            start: false,
        },
    ]);
    app.dispatch(Action::OpenWorkspaceDropdown);
    app.dispatch(Action::ChooseWorkspace(Some(1)));
    settle(&mut app).await?;
    // @step When I select the attachment "spec/attachments/BETA-001/a b.md"
    app.dispatch(Action::OpenAttachment(
        "spec/attachments/BETA-001/a b.md".into(),
    ));
    settle(&mut app).await?;
    // @step Then it is read from "beta" on the daemon
    assert_eq!(
        mock.card_attachment_requests(),
        vec![(
            "/work/beta".to_string(),
            "spec/attachments/BETA-001/a b.md".to_string()
        )]
    );
    // @step And the viewer opens its percent-encoded view URL
    let urls = launched.lock().or_fail("launcher lock")?.clone();
    assert_eq!(urls.len(), 1, "{urls:?}");
    assert!(
        urls[0].ends_with("/view/spec/attachments/BETA-001/a%20b.md"),
        "{urls:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn selecting_an_attachment_before_the_workspace_list_loads_shows_why_nothing_opened(
) -> TestResult {
    // @step Given the workspace list has not loaded
    let mock = Arc::new(MockBackend::new());
    let (mut app, launched) = recording_app(&mock)?;
    // @step When I select an attachment
    app.dispatch(Action::OpenAttachment(
        "spec/attachments/RPC-001/a b.md".into(),
    ));
    settle(&mut app).await?;
    // @step Then nothing is read from the daemon and no browser is launched
    assert!(mock.card_attachment_requests().is_empty());
    assert!(launched.lock().or_fail("launcher lock")?.is_empty());
    // @step And an error explains that no workspace is selected
    assert!(app.compositor().contains("session-storage-dialog"));
    Ok(())
}
