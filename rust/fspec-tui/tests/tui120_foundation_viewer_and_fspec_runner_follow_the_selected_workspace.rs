//! TUI-120 — the foundation viewer and the agent-view fspec runner follow
//! the selected workspace.
//!
//! Feature: spec/features/foundation-viewer-and-fspec-runner-follow-the-selected-workspace.feature

mod common;

use crate::common::TaskOutcome;
use std::sync::{Arc, Mutex};

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{FspecRequest, SessionId, StreamChunk, WorkUnitInfo, WorkspaceEntry};
use common::MockBackend;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const BETA: &str = "/work/beta";

fn unit(id: &str) -> WorkUnitInfo {
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

/// An App over `mock` whose launcher records every URL.
fn app(mock: &Arc<MockBackend>) -> (App, Arc<Mutex<Vec<String>>>) {
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    let launched = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&launched);
    app.set_board_attachment_launcher_for_test(Arc::new(move |url: &str| {
        record
            .lock()
            .map_err(|_| "launcher lock poisoned".to_string())?
            .push(url.to_owned());
        Ok(())
    }));
    (app, launched)
}

fn show_beta(app: &mut App) {
    app.board_store_mut().set_workspaces(vec![
        WorkspaceEntry {
            path: "/work/alpha".into(),
            name: "alpha".into(),
            start: true,
        },
        WorkspaceEntry {
            path: BETA.into(),
            name: "beta".into(),
            start: false,
        },
    ]);
    app.dispatch(Action::OpenWorkspaceDropdown);
    app.dispatch(Action::ChooseWorkspace(Some(1)));
}

async fn settle(app: &mut App) -> TestResult {
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
async fn pressing_d_opens_the_selected_workspaces_foundation_from_the_daemon() -> TestResult {
    // @step Given the board shows workspace "beta"
    let mock = Arc::new(MockBackend::new());
    mock.script_card_attachment(Ok(b"# Beta foundation\n".to_vec()));
    let (mut app, launched) = app(&mock);
    show_beta(&mut app);
    settle(&mut app).await?;
    // @step When the user presses D
    app.dispatch(Action::OpenFoundation);
    settle(&mut app).await?;
    // @step Then "spec/FOUNDATION.md" of "beta" is read from the daemon
    assert_eq!(
        mock.card_attachment_requests(),
        vec![(BETA.to_string(), "spec/FOUNDATION.md".to_string())]
    );
    // @step And the viewer opens it
    let urls = launched
        .lock()
        .map_err(|_| "launcher lock poisoned")?
        .clone();
    assert_eq!(urls.len(), 1, "{urls:?}");
    assert!(urls[0].ends_with("/view/spec/FOUNDATION.md"), "{urls:?}");
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn pressing_d_before_the_workspace_list_loads_shows_why_nothing_opened() -> TestResult {
    // @step Given the workspace list has not loaded
    let mock = Arc::new(MockBackend::new());
    let (mut app, launched) = app(&mock);
    // @step When the user presses D
    app.dispatch(Action::OpenFoundation);
    settle(&mut app).await?;
    // @step Then nothing is read from the daemon
    assert!(mock.card_attachment_requests().is_empty());
    assert!(launched
        .lock()
        .map_err(|_| "launcher lock poisoned")?
        .is_empty());
    // @step And an error explains that no workspace is selected
    assert!(app.compositor().contains("session-storage-dialog"));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_agent_fspec_runner_reads_the_selected_workspaces_work_units() -> TestResult {
    // @step Given the board shows workspace "beta" holding work unit "BETA-001"
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![unit("ALPHA-001")]);
    mock.seed_workspace_work_units(BETA, vec![unit("BETA-001")]);
    let (mut app, _launched) = app(&mock);
    show_beta(&mut app);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    settle(&mut app).await?;
    // @step When a session asks the fspec runner to show work unit "BETA-001"
    app.dispatch(Action::ChunkReceived(
        SessionId::new("s-1"),
        StreamChunk::FspecCommandRequest {
            fspec_request: FspecRequest {
                command: "show-work-unit".to_string(),
                args_json: r#"{"id":"BETA-001"}"#.to_string(),
                project_root: ".".to_string(),
                tool_call_id: "t-1".to_string(),
            },
        },
    ));
    settle(&mut app).await?;
    // @step Then the runner answers with "BETA-001" from "beta"
    let result = mock.last_fspec_result().ok_or("no fspec result was sent")?;
    assert!(result.success, "{result:?}");
    assert!(result.data.contains("BETA-001"), "{}", result.data);
    Ok(())
}
