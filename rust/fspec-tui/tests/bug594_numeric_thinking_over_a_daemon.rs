//! BUG-594: the TUI's WebSocket backend reads a numeric-effort model's
//! thinking options and sets a numeric level over a real daemon
//! connection (bincode over WebSocket).
//!
//! Feature: spec/features/numeric-thinking-values-cross-the-rpc-wire.feature

use std::sync::{Arc, Mutex};

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::transport::websocket::WebSocketFspecBackend;
use codelet_fspec_tui::FspecBackend;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::{
    LogRecord, NumericEffortKind, SessionId, SessionInfo, SessionStatus, StreamChunk,
    ThinkingChoice, ThinkingLevel, ThinkingOptions,
};
use codelet_test_helpers::TestResult;
use serde_json::Number;
use tokio::sync::broadcast;

/// A session manager whose one session's model offers a Decimal effort
/// from 0 to 1 and that records the thinking level it is given.
struct NumericEffortModel {
    inner: StubSessionManagerHandle,
    level: Mutex<ThinkingLevel>,
}

fn decimal_effort() -> ThinkingChoice {
    ThinkingChoice::NumericEffort {
        kind: NumericEffortKind::Decimal,
        min: Number::from(0),
        max: Number::from(1),
    }
}

impl SessionManagerHandle for NumericEffortModel {
    fn list_sessions(&self, project_path: &str) -> Result<Vec<SessionInfo>, String> {
        self.inner.list_sessions(project_path)
    }
    fn create_session_checked(&self, role: Option<String>) -> Result<SessionId, String> {
        self.inner.create_session_checked(role)
    }
    fn send_input(&self, session_id: &SessionId, text: String) -> Result<(), String> {
        self.inner.send_input(session_id, text)
    }
    fn interrupt(&self, session_id: &SessionId) -> Result<(), String> {
        self.inner.interrupt(session_id)
    }
    fn get_session_status(&self, session_id: &SessionId) -> Result<SessionStatus, String> {
        self.inner.get_session_status(session_id)
    }
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.inner.chunks_rx()
    }
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.inner.logs_rx()
    }
    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        self.inner.chunks_tx()
    }
    fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        self.inner.logs_tx()
    }
    fn get_thinking_options(&self, _session_id: &SessionId) -> Result<ThinkingOptions, String> {
        Ok(ThinkingOptions::Levels {
            model: "custom/numeric-effort".to_string(),
            choices: vec![ThinkingChoice::Off, decimal_effort()],
            takes_off: true,
        })
    }
    fn set_thinking_level(
        &self,
        _session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        *self.level.lock().map_err(|error| error.to_string())? = level;
        Ok(())
    }
    fn get_thinking_level(&self, _session_id: &SessionId) -> Result<ThinkingLevel, String> {
        Ok(self
            .level
            .lock()
            .map_err(|error| error.to_string())?
            .clone())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_tui_reads_and_sets_a_numeric_effort_over_a_daemon_connection() -> TestResult {
    // @step Given a daemon over WebSocket whose session's model offers a Decimal effort from 0 to 1
    let workspace = codelet_test_helpers::setup_temp_directory()?;
    std::fs::create_dir_all(workspace.path().join("spec"))?;
    std::fs::write(
        workspace.path().join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    let model = Arc::new(NumericEffortModel {
        inner: StubSessionManagerHandle::new(),
        level: Mutex::new(ThinkingLevel::Off),
    });
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::new(WorkUnitsWatcher::new(workspace.path())?),
        model.clone(),
    ));
    let (address, _stats, task) = bind_and_serve("127.0.0.1:0", service).await?;

    // @step And the TUI's WebSocket backend connected to it
    let backend =
        WebSocketFspecBackend::connect(url::Url::parse(&format!("ws://{address}"))?).await?;
    let session = SessionId::new("numeric-session");

    // @step When the TUI asks for the session's thinking options
    let options = backend.get_thinking_options(session.clone()).await?;

    // @step Then it receives Levels holding NumericEffort with kind Decimal, min 0 and max 1
    assert_eq!(options.choices(), &[ThinkingChoice::Off, decimal_effort()]);

    // @step When the TUI sets the session's thinking level to Numeric 0.5
    let half = Number::from_f64(0.5).ok_or("0.5 is not a JSON number")?;
    backend
        .set_thinking_level(session.clone(), ThinkingLevel::Numeric(half.clone()))
        .await?;

    // @step Then the session's thinking level is Numeric 0.5
    assert_eq!(
        model.get_thinking_level(&session)?,
        ThinkingLevel::Numeric(half.clone())
    );
    assert_eq!(
        backend.get_thinking_level(session).await?,
        ThinkingLevel::Numeric(half)
    );
    task.abort();
    Ok(())
}
