use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{ExecStdinRequest, SessionId, SessionState, StreamChunk};

use crate::common::MockBackend;

#[derive(Clone, Copy)]
pub enum Probe {
    Focus,
    Paused,
}

pub const PROBES: [Probe; 2] = [Probe::Focus, Probe::Paused];
pub type TestResult<Value = ()> = Result<Value, Box<dyn std::error::Error>>;

pub fn session() -> SessionId {
    SessionId::new("target")
}

pub fn request(command: &str) -> ExecStdinRequest {
    ExecStdinRequest {
        exec_session_id: command.to_owned(),
        command: command.to_owned(),
        quiet_seconds: 5,
        ts_ms: 1000,
    }
}

pub async fn fresh_app() -> TestResult<(App, Arc<MockBackend>)> {
    let backend = Arc::new(MockBackend::new());
    let transport: Arc<dyn FspecBackend> = backend.clone();
    let mut app = App::new(transport);
    app.dispatch(Action::SessionCreated(session()));
    app.dispatch(Action::SessionCreated(SessionId::new("other")));
    app.navigator_mut().active_view = ViewMode::Agent;
    drain(&mut app).await?;
    Ok((app, backend))
}

pub fn start_probe(app: &mut App, probe: Probe) -> TestResult {
    match probe {
        Probe::Focus => {
            if app.agent_view_store().current_session() == Some(&session()) {
                app.dispatch(Action::SessionNext);
            }
            app.dispatch(Action::SessionPrev);
        }
        Probe::Paused => {
            app.dispatch(Action::ChunkReceived(
                session(),
                StreamChunk::SessionStateChange {
                    state: SessionState::Paused,
                },
            ));
            let trigger = app
                .try_recv_action()
                .ok_or("Paused chunk must queue its probe trigger")?;
            assert!(matches!(trigger, Action::PauseChunkReceived(_)));
            app.dispatch(trigger);
        }
    }
    Ok(())
}

pub async fn take_completed(app: &mut App) -> TestResult<Vec<Action>> {
    while let Some(task) = app.next_pending_task() {
        tokio::time::timeout(Duration::from_secs(5), task).await??;
    }
    let mut actions = Vec::new();
    while let Some(action) = app.try_recv_action() {
        actions.push(action);
    }
    Ok(actions)
}

pub fn deliver(app: &mut App, actions: Vec<Action>) {
    for action in actions {
        app.dispatch(action);
    }
}

pub async fn drain(app: &mut App) -> TestResult {
    loop {
        let actions = take_completed(app).await?;
        if actions.is_empty() {
            return Ok(());
        }
        deliver(app, actions);
    }
}

pub async fn snapshot(
    app: &mut App,
    backend: &MockBackend,
    probe: Probe,
    value: Option<ExecStdinRequest>,
) -> TestResult<Vec<Action>> {
    let calls = backend.get_exec_stdin_request_calls();
    backend.script_exec_stdin_request(session(), value);
    start_probe(app, probe)?;
    let actions = take_completed(app).await?;
    assert!(backend.get_exec_stdin_request_calls() > calls);
    assert!(
        !actions.is_empty(),
        "probe must produce a deliverable action"
    );
    Ok(actions)
}

pub fn push(app: &mut App, value: Option<ExecStdinRequest>) {
    let chunk = value.map_or(StreamChunk::ExecStdinRequestCleared, |request| {
        StreamChunk::ExecStdinRequest { request }
    });
    app.dispatch(Action::ChunkReceived(session(), chunk));
}

pub fn visible_command(app: &App) -> Option<&str> {
    app.agent_view_store()
        .exec_stdin_for(&session())
        .map(|request| request.command.as_str())
}
