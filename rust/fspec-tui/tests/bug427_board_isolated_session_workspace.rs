//! BUG-427 — the board creates isolated sessions in the selected workspace.
//!
//! Feature: spec/features/board-isolated-session-uses-selected-workspace.feature

mod common;

use crate::common::TaskOutcome;
use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{WorkUnitInfo, WorkspaceEntry};
use common::MockBackend;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

const ALPHA: &str = "/tmp/bug427-alpha";
const BETA: &str = "/tmp/bug427-beta";

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

fn entries() -> Vec<WorkspaceEntry> {
    vec![
        WorkspaceEntry {
            path: ALPHA.into(),
            name: "alpha".into(),
            start: true,
        },
        WorkspaceEntry {
            path: BETA.into(),
            name: "beta".into(),
            start: false,
        },
    ]
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

async fn settle(app: &mut App) -> Result<(), String> {
    for _ in 0..10 {
        while let Some(task) = app.next_pending_task() {
            task.await
                .completed_or_cancelled()
                .map_err(|error| error.to_string())?;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_isolated_agent_starts_in_the_selected_workspace() -> Result<(), String> {
    // @step Given the user has selected workspace "beta"
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![unit("ALPHA-001")]);
    mock.seed_workspaces(entries());
    mock.seed_workspace_work_units(BETA, vec![unit("BETA-001")]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.map_err(|error| error.to_string())?;
    settle(&mut app).await?;
    for code in [KeyCode::Char('w'), KeyCode::Down, KeyCode::Enter] {
        app.handle_event(&key(code));
        settle(&mut app).await?;
    }
    // @step When the user creates a new isolated agent from the board
    app.dispatch(Action::CreateSessionSubmitted { isolated: true });
    settle(&mut app).await?;
    // @step Then the isolated session is created in beta's path
    assert_eq!(mock.isolated_session_created_in().as_deref(), Some(BETA));
    Ok(())
}
