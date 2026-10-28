use std::{sync::Arc, time::Duration};

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitContext, WorkUnitFocus};

mod common;

type TestResult = Result<(), Box<dyn std::error::Error>>;
#[path = "bug230_focus_recovery/lifecycle.rs"]
mod lifecycle;

fn units() -> Vec<codelet_rpc_types::WorkUnitInfo> {
    vec![codelet_rpc_types::WorkUnitInfo {
        id: "AUTH-002".into(),
        title: "Current".into(),
        status: "implementing".into(),
        work_type: "story".into(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }]
}

async fn settle(app: &mut App) {
    loop {
        while let Some(task) = app.next_pending_task() {
            assert!(task.await.is_ok());
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn an_attachment_acknowledgement_does_not_rewrite_a_newer_remote_clear() -> TestResult {
    // @step Given an existing session focused on card "AUTH-001" and card "AUTH-002" on the board
    let backend = Arc::new(common::MockBackend::new());
    let session = SessionId::new("first");
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(session.clone()));
    app.dispatch(Action::WorkUnitsLoaded(units()));
    backend.clear_focus_after_binding();
    // @step When the user explicitly assigns card "AUTH-002" to that session
    app.dispatch(Action::AttachWorkUnitToSession("AUTH-002".into()));
    settle(&mut app).await;
    // @step And a delayed acknowledgement for the previous card cannot replace the newer focus
    assert!(backend
        .get_work_unit_focus(session.clone())
        .await?
        .context
        .is_none());
    assert!(app
        .agent_view_store()
        .work_unit_context_for(&session)
        .is_none());
    assert!(app.board_store().session_for("AUTH-002").is_none());
    assert_eq!(
        backend
            .get_work_unit_context(session)
            .await?
            .map(|context| context.id),
        Some("AUTH-002".into())
    );
    Ok(())
}

async fn info(
    backend: &common::MockBackend,
    session: SessionId,
) -> Result<codelet_rpc_types::SessionInfo, Box<dyn std::error::Error>> {
    backend.seed_session_snapshot(vec![session]);
    backend
        .session_snapshot()
        .await
        .into_iter()
        .next()
        .ok_or_else(|| "missing fixture session".into())
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn remote_creation_before_local_acknowledgement_preserves_board_entry() -> TestResult {
    // @step Given an existing session focused on card "AUTH-001" and card "AUTH-002" on the board
    let backend = Arc::new(common::MockBackend::new());
    let session = SessionId::new("new-board-session");
    backend.script_create_session(session.clone());
    let mut app = App::new(backend.clone());
    app.dispatch(Action::WorkUnitsLoaded(units()));
    // @step When the user enters card "AUTH-002"
    app.dispatch(Action::EnterWorkUnit("AUTH-002".into()));
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    app.dispatch(Action::SessionObserved(
        info(&backend, session.clone()).await?,
    ));
    settle(&mut app).await;
    // @step Then the backend and all card displays identify "AUTH-002"
    assert_eq!(
        backend
            .get_work_unit_focus(session.clone())
            .await?
            .context
            .map(|context| context.id),
        Some("AUTH-002".into())
    );
    assert_eq!(app.board_store().session_for("AUTH-002"), Some(&session));
    Ok(())
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn a_reopened_session_restores_an_unchanged_focus_revision() -> TestResult {
    // @step Given a newly connected client observing two existing sessions
    let backend = Arc::new(common::MockBackend::new());
    let session = SessionId::new("reopened");
    let mut snapshot = info(&backend, session.clone()).await?;
    snapshot.work_unit_focus = WorkUnitFocus {
        revision: 4,
        context: Some(WorkUnitContext {
            id: "AUTH-002".into(),
            title: "Current".into(),
            status: "implementing".into(),
        }),
        source: "restored".into(),
    };
    let mut app = App::new(backend);
    app.dispatch(Action::SessionObserved(snapshot.clone()));
    app.dispatch(Action::AgentExitChoice {
        choice: codelet_fspec_tui::components::exit_confirmation_dialog::ExitChoice::CloseSession,
    });
    // @step When it receives authoritative card focus for one session and cleared focus for the other
    app.dispatch(Action::SessionObserved(snapshot));
    // @step Then only the first session has a card suffix and a board badge
    assert_eq!(app.board_store().session_for("AUTH-002"), Some(&session));
    Ok(())
}

async fn revision(app: &mut App, session: &SessionId, expected: u64) -> TestResult {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            while let Some(action) = app.try_recv_action() {
                app.dispatch(action);
            }
            if app.agent_view_store().work_unit_focus_revision_for(session) == Some(expected) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    Ok(())
}

/// Feature: spec/features/session-work-unit-focus-projection.feature
#[tokio::test]
async fn lagged_subscriptions_and_reconnects_refetch_authoritative_focus() -> TestResult {
    // @step Given a newly connected client observing two existing sessions
    let backend = Arc::new(common::MockBackend::new());
    let session = SessionId::new("first");
    backend.seed_live_sessions(vec![session.clone()]);
    backend
        .set_work_unit_context(
            session.clone(),
            Some(WorkUnitContext {
                id: "AUTH-002".into(),
                title: "Current".into(),
                status: "implementing".into(),
            }),
        )
        .await?;
    let mut app = App::new(backend.clone());
    app.dispatch(Action::SessionCreated(session.clone()));
    let initial = backend
        .set_work_unit_focus(session.clone(), Some("AUTH-002".into()), "initial".into())
        .await?;
    app.bootstrap().await?;
    revision(&mut app, &session, initial.revision).await?;
    let fetches = backend.focus_fetches();

    // @step When it receives authoritative card focus for one session and cleared focus for the other
    let mut latest = WorkUnitFocus::default();
    for _ in 0..64 {
        latest = backend
            .set_work_unit_focus(session.clone(), None, "clear".into())
            .await?;
    }
    revision(&mut app, &session, latest.revision).await?;

    // @step Then only the first session has a card suffix and a board badge
    assert!(
        backend.focus_fetches() > fetches,
        "lag must fetch the authoritative snapshot"
    );
    assert!(app
        .agent_view_store()
        .work_unit_context_for(&session)
        .is_none());
    assert!(app.board_store().session_for("AUTH-002").is_none());
    let fetches = backend.focus_fetches();
    app.dispatch(Action::Reconnected);
    tokio::time::timeout(Duration::from_secs(3), async {
        while backend.focus_fetches() == fetches {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;

    // @step And an older initial snapshot cannot replace a newer live focus update
    app.dispatch(Action::WorkUnitFocusChanged(session.clone(), initial));
    assert!(app
        .agent_view_store()
        .work_unit_context_for(&session)
        .is_none());
    Ok(())
}
