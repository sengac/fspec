//! BUG-431 — card attachments are read from the daemon.
//!
//! Feature: spec/features/card-attachments-come-from-the-daemon.feature

mod common;

use crate::common::TaskOutcome;
use std::sync::{Arc, Mutex};

use codelet_fspec_tui::{App, FspecBackend};
use common::MockBackend;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const REMOTE_WORKSPACE: &str = "/srv/bug431-remote-only/project";
const PATH: &str = "spec/attachments/A/notes.md";

/// An App over `mock` whose card workspace exists only on the daemon, with a
/// browser launcher that records every URL.
fn app(mock: &Arc<MockBackend>) -> (App, Arc<Mutex<Vec<String>>>) {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    let launched = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&launched);
    app.set_board_attachment_launcher_for_test(Arc::new(move |url: &str| {
        record
            .lock()
            .map_err(|_| "launcher lock poisoned".to_string())?
            .push(url.to_string());
        Ok(())
    }));
    (app, launched)
}

async fn settle(app: &mut App) -> Result<(), String> {
    while let Some(task) = app.next_pending_task() {
        task.await
            .completed_or_cancelled()
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_attachment_in_a_workspace_that_exists_only_on_the_daemon_opens() -> TestResult {
    // @step Given a card whose workspace exists only on the daemon
    assert!(!std::path::Path::new(REMOTE_WORKSPACE).exists());
    let mock = Arc::new(MockBackend::new());
    mock.script_card_attachment(Ok(b"# notes from the daemon\n".to_vec()));
    let (mut app, launched) = app(&mock);
    // @step When an attachment of that card is opened
    app.open_board_card_attachment_for_test(REMOTE_WORKSPACE.into(), PATH);
    settle(&mut app).await?;
    // @step Then the viewer serves the daemon's bytes for it
    assert_eq!(
        mock.card_attachment_requests(),
        vec![(REMOTE_WORKSPACE.to_string(), PATH.to_string())]
    );
    let url = launched
        .lock()
        .map_err(|_| "launched lock")?
        .first()
        .cloned()
        .ok_or("no browser launch")?;
    let body = reqwest::get(&url).await?.text().await?;
    assert!(body.contains("notes from the daemon"), "{body}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_daemons_refusal_is_shown_for_an_attachment_it_cannot_serve() -> TestResult {
    // @step Given a card whose workspace exists only on the daemon
    let mock = Arc::new(MockBackend::new());
    mock.script_card_attachment(Err(
        "attachment not found (missing file in the card workspace)".to_string(),
    ));
    let (mut app, launched) = app(&mock);
    // @step When an attachment the daemon cannot serve is opened
    app.open_board_card_attachment_for_test(REMOTE_WORKSPACE.into(), PATH);
    settle(&mut app).await?;
    // @step Then the attachment fails with the daemon's reason and no browser opens
    let failure = std::iter::from_fn(|| app.try_recv_action()).find_map(|action| match action {
        codelet_fspec_tui::Action::BoardCardAttachmentFailed { message, .. } => Some(message),
        _ => None,
    });
    assert!(
        failure
            .as_deref()
            .is_some_and(|message| message.contains("attachment not found")),
        "{failure:?}"
    );
    assert!(launched.lock().map_err(|_| "launched lock")?.is_empty());
    Ok(())
}
