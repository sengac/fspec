//! Feature: spec/features/send-input-refuses-sessions-the-daemon-does-not-hold.feature
//!
//! BUG-328: send_input / send_input_with_thinking return an error for a
//! session the daemon does not hold, on both transports against a real
//! `codelet_sessions::SessionManager`, and the TUI shows a refused send.

mod common;

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, OrFailErr, TestResult};
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use common::MockBackend;

async fn daemon_backends(
    workspace: &std::path::Path,
) -> Result<Vec<Arc<dyn FspecBackend>>, codelet_test_helpers::TestError> {
    std::fs::create_dir_all(workspace.join("spec")).or_fail("expected a value")?;
    std::fs::write(
        workspace.join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .or_fail("expected a value")?;
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).or_fail("expected a value")?);
    let handle: Arc<dyn SessionManagerHandle> = SessionManager::new();
    let service = Arc::new(
        SharedFspecService::with_session_manager(watcher, handle).with_cwd(workspace.to_path_buf()),
    );
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .or_fail("expected a value")?;
    let url = url::Url::parse(&format!("ws://{addr}/")).or_fail("expected a value")?;
    let websocket: Arc<dyn FspecBackend> = Arc::new(
        WebSocketFspecBackend::connect(url)
            .await
            .or_fail("expected a value")?,
    );
    Ok(vec![embedded, websocket])
}

// Scenario: send_input over a transport reports a session the daemon does not hold
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_send_input_reports_a_session_the_daemon_does_not_hold() -> TestResult {
    let workspace = tempfile::tempdir().or_fail("expected a value")?;
    // @step Given a daemon session manager that holds no session "missing-session"
    let backends = daemon_backends(workspace.path()).await?;
    let missing = SessionId::new("missing-session");

    // @step When a client calls send_input and send_input_with_thinking for "missing-session" over the embedded and websocket transports
    let mut errors = Vec::new();
    for backend in &backends {
        errors.push(backend.send_input(missing.clone(), "hello".into()).await);
        errors.push(
            backend
                .send_input_with_thinking(missing.clone(), "hello".into(), None)
                .await,
        );
    }

    // @step Then every call returns an error naming "missing-session" and telling the client to resume it
    assert_eq!(errors.len(), 4);
    for result in errors {
        let error = result.or_fail_err("input to a session the daemon does not hold")?;
        let text = error.to_string();
        assert!(text.contains("missing-session"), "{text}");
        assert!(text.contains("resume_session"), "{text}");
    }
    Ok(())
}

async fn drain(app: &mut App) -> codelet_test_helpers::TestResult {
    loop {
        while let Some(task) = app.next_pending_task() {
            task.await.completed_or_cancelled()?;
        }
        let Some(action) = app.try_recv_action() else {
            break;
        };
        app.dispatch(action);
    }
    Ok(())
}

fn scrollback_text(app: &App, session: &SessionId) -> String {
    app.agent_view_store()
        .session_context_for(session)
        .map(|context| context.scrollback.visible_window(1024))
        .unwrap_or_default()
        .iter()
        .flat_map(|chunk| {
            chunk.lines.iter().map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// Scenario: A refused send is shown in the session's scrollback
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_refused_send_is_shown_in_the_scrollback() -> TestResult {
    let session = SessionId::new("s-1");
    let mock = Arc::new(MockBackend::new());
    mock.fail_send_input(
        "Session not found: s-1 (resume it with resume_session before sending input)",
    );
    let mut app = App::new(mock.clone());
    // @step Given the TUI is showing a session whose input the daemon refuses
    app.dispatch(Action::SessionCreated(session.clone()));
    app.dispatch(Action::OpenAgentView(Some(session.clone())));
    drain(&mut app).await?;

    // @step When I submit "hello" in that session
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, "hello".into()));
    }
    drain(&mut app).await?;

    // @step Then the session's scrollback shows the send error
    assert_eq!(mock.send_input_calls(), 1);
    let scrollback = scrollback_text(&app, &session);
    assert!(
        scrollback.contains("Session not found: s-1"),
        "scrollback must show the refused send:\n{scrollback}"
    );
    Ok(())
}
