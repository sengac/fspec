//! BUG-648 support: a real daemon (SessionManager + SharedFspecService,
//! embedded and WebSocket), the session-state scripts the scenarios drive
//! on its BackgroundSession, and a digest of what a TUI shows for a session.

use std::net::SocketAddr;
use std::sync::Arc;

use codelet_core::persistence;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_fspec_tui::app::transcript_sync::TranscriptAction;
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{
    ContextFillInfo, NotificationSeverity, SessionId, SessionStatus, StreamChunk, TokenTracker,
    ToolCallInfo, ToolProgressInfo, ToolResultInfo,
};
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::SessionManager;
use codelet_test_helpers::{TestError, TestResult};

use crate::common::TaskOutcome;

pub const MODEL: &str = "anthropic/claude-opus-4-5";

/// Which transport a TUI uses to reach the daemon.
#[derive(Debug, Clone, Copy)]
pub enum Transport {
    Embedded,
    WebSocket,
}

pub const TRANSPORTS: [Transport; 2] = [Transport::Embedded, Transport::WebSocket];

/// The user directory, data directory and API key of one test, restored on drop.
pub struct Environment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Environment {
    pub fn isolate(root: &std::path::Path) -> Result<Self, TestError> {
        let saved = ["FSPEC_USER_DIR", "ANTHROPIC_API_KEY"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        std::env::set_var("FSPEC_USER_DIR", root);
        std::env::set_var("ANTHROPIC_API_KEY", "offline-bug648-test");
        codelet_common::set_data_directory(root.to_owned())?;
        persistence::reset_stores_for_tests();
        std::fs::create_dir_all(root.join("cache"))?;
        std::fs::write(
            root.join("cache/models.json"),
            include_str!("../../../sessions/tests/fixtures/prov101_models.json"),
        )?;
        Ok(Self(saved))
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        persistence::reset_stores_for_tests();
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// One daemon: its session manager, its service, its WebSocket server and
/// the session S the scenario drives.
pub struct Daemon {
    pub manager: Arc<SessionManager>,
    pub service: Arc<SharedFspecService>,
    pub address: SocketAddr,
    pub stats: codelet_rpc_server::ServerStats,
    pub server: tokio::task::JoinHandle<()>,
    pub project: tempfile::TempDir,
}

impl Daemon {
    pub async fn start() -> Result<Self, TestError> {
        let (project, _) = crate::common::temp_service()?;
        let manager = SessionManager::new();
        let handle: Arc<dyn SessionManagerHandle> = manager.clone();
        let watcher = Arc::new(codelet_core::work_units::WorkUnitsWatcher::new(
            project.path(),
        )?);
        let service = Arc::new(
            SharedFspecService::with_session_manager(watcher, handle)
                .with_cwd(project.path().to_path_buf()),
        );
        let (address, stats, server) =
            crate::common::start_ws_server_with_stats(service.clone()).await?;
        Ok(Self {
            manager,
            service,
            address,
            stats,
            server,
            project,
        })
    }

    /// A session over the daemon's start workspace.
    pub async fn create_session(&self, name: &str) -> Result<SessionId, TestError> {
        let manifest = persistence::create_session_with_provider(name, self.project.path(), MODEL)?;
        self.manager
            .create_session_from_manifest(&manifest, MODEL)
            .await?;
        Ok(SessionId::new(manifest.id.to_string()))
    }

    pub fn session(&self, id: &SessionId) -> Result<Arc<BackgroundSession>, TestError> {
        Ok(self.manager.get_session(&id.value)?)
    }

    pub async fn backend(&self, transport: Transport) -> Result<Arc<dyn FspecBackend>, TestError> {
        Ok(match transport {
            Transport::Embedded => Arc::new(EmbeddedFspecBackend::new(
                tokio::runtime::Handle::current(),
                self.service.clone(),
            )),
            Transport::WebSocket => Arc::new(
                WebSocketFspecBackend::connect(crate::common::ws_url(self.address)?).await?,
            ),
        })
    }

    pub fn stop(&self) {
        codelet_rpc_server::request_shutdown(&self.stats);
        self.server.abort();
    }
}

// ── Session-state scripts (driven on the daemon's BackgroundSession) ──────

pub fn tool_call(id: &str) -> StreamChunk {
    StreamChunk::tool_call(ToolCallInfo {
        id: id.into(),
        name: "Bash".into(),
        input: "{\"command\":\"./build.sh\"}".into(),
    })
}

pub fn tool_output(id: &str, text: &str) -> StreamChunk {
    StreamChunk::tool_progress(ToolProgressInfo {
        tool_call_id: id.into(),
        tool_name: "Bash".into(),
        output_chunk: text.into(),
        is_stderr: false,
    })
}

pub fn tool_result(id: &str, text: &str) -> StreamChunk {
    StreamChunk::tool_result(ToolResultInfo {
        tool_call_id: id.into(),
        content: text.into(),
        is_error: false,
    })
}

pub fn tokens(input: u32, output: u32) -> StreamChunk {
    StreamChunk::token_update(TokenTracker {
        input_tokens: input,
        output_tokens: output,
        ..TokenTracker::default()
    })
}

pub fn context_fill(percentage: u32) -> StreamChunk {
    StreamChunk::context_fill_update(ContextFillInfo {
        fill_percentage: percentage,
        effective_tokens: f64::from(percentage) * 1000.0,
        threshold: 90_000.0,
        context_window: 100_000.0,
    })
}

/// A marker the scenario waits for: every journaled chunk before it has
/// been applied once a TUI shows it.
pub fn sentinel(session: &BackgroundSession, marker: &str) {
    session.handle_output(StreamChunk::user_notification(
        marker.into(),
        NotificationSeverity::Info,
    ));
}

/// S is Running with a Bash tool call that has streamed 'line 1', has
/// reported 1200/300 tokens and a 40% context fill.
pub fn running_tool(session: &BackgroundSession) {
    session.set_status(SessionStatus::Running);
    session.handle_output(StreamChunk::text("Building it now.".into()));
    session.handle_output(tool_call("tool-1"));
    session.handle_output(tool_output("tool-1", "line 1\n"));
    session.handle_output(tokens(1200, 300));
    session.handle_output(context_fill(40));
}

pub fn question(text: &str) -> codelet_tools::request_user_input::HitlRequest {
    codelet_tools::request_user_input::HitlRequest {
        questions: vec![codelet_tools::request_user_input::HitlQuestion {
            id: "auth_library".into(),
            header: "Auth".into(),
            question: text.into(),
            options: None,
        }],
    }
}

/// S asks a question and is Paused (as the agent loop's HITL handler does).
pub fn ask(session: &BackgroundSession, text: &str) {
    session.set_status(SessionStatus::Running);
    session.set_hitl_request(Some(question(text)));
    session.set_status(SessionStatus::Paused);
}

pub fn approval(message: &str) -> codelet_tools::tool_pause::PauseState {
    codelet_tools::tool_pause::PauseState {
        kind: codelet_tools::tool_pause::PauseKind::Confirm,
        tool_name: "Bash".into(),
        message: message.into(),
        details: None,
        tool_call_id: Some("tool-2".into()),
    }
}

/// S waits on an approval (set_pause_state also sets Paused).
pub fn ask_approval(session: &BackgroundSession, message: &str) {
    session.set_status(SessionStatus::Running);
    session.set_pause_state(Some(approval(message)));
}

// ── Driving a TUI ────────────────────────────────────────────────────────

/// Run every pending task and dispatch every queued action.
pub async fn settle(app: &mut App) -> TestResult {
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

/// Drive `app` until `done` holds (subscriber tasks deliver live chunks
/// concurrently, so the loop waits on the state itself). Each round polls
/// the open tabs' transcripts, as the run loop's tick does.
pub async fn wait_until(app: &mut App, what: &str, done: impl Fn(&App) -> bool) -> TestResult {
    codelet_test_helpers::deadlock_guard(what, async {
        loop {
            app.dispatch(Action::Transcript(TranscriptAction::Poll));
            settle(app).await?;
            if done(app) {
                return Ok::<(), TestError>(());
            }
            codelet_test_helpers::yield_now().await;
        }
    })
    .await??;
    Ok(())
}

/// The full text of every scrollback entry of `session`'s tab.
pub fn transcript(app: &App, session: &SessionId) -> Vec<String> {
    let Some(context) = app.agent_view_store().session_context_for(session) else {
        return Vec::new();
    };
    context
        .scrollback
        .chunks()
        .iter()
        .filter_map(|chunk| context.scrollback.full_text_for_seq(chunk.seq))
        .collect()
}

pub fn shows(app: &App, session: &SessionId, marker: &str) -> bool {
    transcript(app, session)
        .iter()
        .any(|text| text.contains(marker))
}

/// Wait until `app`'s tab of `session` shows `marker`.
pub async fn wait_for_marker(app: &mut App, session: &SessionId, marker: &str) -> TestResult {
    let what = format!("the TUI shows {marker}");
    wait_until(app, &what, |app| shows(app, session, marker)).await
}

/// Everything a TUI shows of one session's live state.
#[derive(Debug, PartialEq)]
pub struct LiveView {
    pub status: Option<SessionStatus>,
    pub compaction: Option<String>,
    pub exec_stdin: Option<String>,
    pub approval: Option<(String, String)>,
    pub question: Option<(String, String)>,
    pub tokens: Option<String>,
    pub provider_waiting: Option<String>,
    pub focus: Option<String>,
    pub transcript: Vec<String>,
}

pub fn live_view(app: &App, session: &SessionId) -> LiveView {
    let store = app.agent_view_store();
    LiveView {
        status: store.session_status_for(session).copied(),
        compaction: store
            .compaction_progress_for(session)
            .map(|progress| format!("{progress:?}")),
        exec_stdin: store
            .exec_stdin_for(session)
            .map(|request| format!("{} {}", request.exec_session_id, request.command)),
        approval: store
            .pause_state_for(session)
            .zip(store.pause_route_for(session))
            .map(|(state, route)| (route.request_id.clone(), state.prompt.clone())),
        question: store.hitl_prompt_for(session).map(|prompt| {
            (
                prompt.route.request_id.clone(),
                prompt
                    .request
                    .questions
                    .iter()
                    .map(|question| question.question.clone())
                    .collect::<Vec<_>>()
                    .join(" | "),
            )
        }),
        tokens: store
            .token_state_for(session)
            .map(|state| format!("{state:?}")),
        provider_waiting: store
            .session_context_for(session)
            .and_then(|context| context.provider_waiting.clone()),
        focus: store
            .work_unit_context_for(session)
            .map(|context| context.id.clone()),
        transcript: transcript(app, session)
            .into_iter()
            .filter(|text| !text.contains("Reconnect"))
            .collect(),
    }
}

/// Render `frames` frames of `app` at 120x40 and return the last frame's rows.
pub fn render_rows(app: &mut App, frames: usize) -> Vec<String> {
    let area = ratatui::layout::Rect::new(0, 0, 120, 40);
    let mut buffer = ratatui::buffer::Buffer::empty(area);
    for _ in 0..frames {
        buffer = ratatui::buffer::Buffer::empty(area);
        app.render(area, &mut buffer);
    }
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect()
}

/// A live exec session (a long sleep) under an agent id no session owns.
pub async fn live_exec() -> Result<(codelet_tools::unified_exec::UnifiedExecTool, String), TestError>
{
    use rig::tool::Tool;
    codelet_tools::facade::declare_unrestricted_tool_access(
        "BUG-648 drives an exec session without a session manager",
    );
    let tool = codelet_tools::unified_exec::UnifiedExecTool::new(uuid::Uuid::new_v4());
    let result = tool
        .call(codelet_tools::unified_exec::UnifiedExecArgs(
            serde_json::json!({
                "action": "run",
                "command": "sh -c 'sleep 120'",
                "yield_time_ms": 50
            }),
        ))
        .await?;
    let id = result
        .session_id
        .ok_or("the exec command is still running")?;
    Ok((tool, id))
}

pub async fn close_exec(
    tool: &codelet_tools::unified_exec::UnifiedExecTool,
    id: &str,
) -> TestResult {
    use rig::tool::Tool;
    tool.call(codelet_tools::unified_exec::UnifiedExecArgs(
        serde_json::json!({
            "action": "close",
            "session_id": id
        }),
    ))
    .await?;
    Ok(())
}
