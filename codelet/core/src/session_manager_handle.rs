//! Session manager handle abstraction (RPC-007).
//!
//! Defines the [`SessionManagerHandle`] trait that the dual-transport RPC
//! layer (codelet/rpc) consumes via dependency injection. The concrete
//! 8,649 LOC `SessionManager` implementation lives in `codelet/napi/src/
//! session_manager.rs` — codelet/core defines only the trait surface so
//! that codelet/rpc never imports codelet/napi (rpc → napi forbidden;
//! rpc → core permitted; napi → core permitted).
//!
//! ## NAPI shared contract invariant
//!
//! The trait surface here, plus the five new types in codelet/rpc-types
//! (SessionId, SessionInfo, SessionStatus, StreamChunk, LogRecord), are
//! the contract that all three frontends consume identically:
//!   * the JS frontend via codelet/napi's #[napi] re-exports,
//!   * the built-in ratatui frontend via EmbeddedTransport calling
//!     Arc<dyn SessionManagerHandle> directly,
//!   * the WebSocket frontend via tarpc-generated FspecServiceClient
//!     over bincode-encoded Envelope.
//!
//! ## Test stub
//!
//! [`StubSessionManagerHandle`] is a minimal in-memory implementation
//! used by integration tests so they can exercise the full RPC + push
//! channel surface without dragging in the real SessionManager and its
//! dependency tree (codelet-cli, codelet-git, codelet-tools,
//! codelet-providers, OAuth, persistence, ghost commits, etc.).

use codelet_rpc_types::{
    LogRecord, ModelInfo, ProviderInfo, SessionId, SessionInfo, SessionStatus, StreamChunk,
    ThinkingLevel,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tokio::sync::broadcast;

fn status_str(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Idle => "idle",
        SessionStatus::Running => "running",
        SessionStatus::Paused => "paused",
        SessionStatus::Compacting => "compacting",
        SessionStatus::Interrupted => "interrupted",
        SessionStatus::Cleared => "cleared",
    }
}

/// Trait implemented by the concrete `SessionManager` in codelet/napi
/// and by [`StubSessionManagerHandle`] in tests.
///
/// All methods are synchronous and non-blocking — the actual session
/// machinery (LLM streams, tool execution, compaction) is owned by
/// the implementation and runs on the host runtime.
pub trait SessionManagerHandle: Send + Sync + 'static {
    /// Return public metadata for every session currently tracked.
    fn list_sessions(&self) -> Vec<SessionInfo>;

    /// Create a new session with an optional role. Returns the
    /// freshly-minted [`SessionId`].
    fn create_session(&self, role: Option<String>) -> SessionId;

    /// Send user input to a session. Returns immediately — the actual
    /// streaming response arrives on the chunks broadcast subscribed
    /// via [`SessionManagerHandle::chunks_rx`].
    fn send_input(&self, session_id: &SessionId, text: String);

    /// Interrupt a running session. Returns immediately. A subsequent
    /// `StreamChunk::Interrupted` will arrive on the chunks broadcast.
    fn interrupt(&self, session_id: &SessionId);

    /// Return the current lifecycle state of a session.
    fn get_session_status(&self, session_id: &SessionId) -> SessionStatus;

    /// Subscribe to the per-process StreamChunk broadcast. Every send_input
    /// pushes its streaming output here as `(SessionId, StreamChunk)` tuples.
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)>;

    /// Subscribe to the per-process LogRecord broadcast. The host's
    /// tracing::Layer pushes structured events here.
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord>;

    /// Return a cloneable handle to the chunks broadcast sender so the
    /// host's tracing layer / NAPI ThreadsafeFunction co-listener can
    /// publish or co-subscribe directly.
    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)>;

    /// Return a cloneable handle to the logs broadcast sender so the
    /// host's tracing::Layer can push records onto the same broadcast
    /// that other listeners observe via `logs_rx`.
    fn logs_tx(&self) -> broadcast::Sender<LogRecord>;

    /// RPC-018: return the display + capability metadata for the model
    /// currently bound to `session_id`. Default implementation returns
    /// `ModelInfo::default()` (empty display name, all-false caps,
    /// context_window = 0) so handles that don't yet know how to
    /// resolve provider/model state — including `StubSessionManagerHandle`
    /// — compile without per-test wiring. The concrete codelet/napi
    /// `SessionManager` overrides this in RPC-022 once the ModelSelector
    /// modal dialog needs live data.
    fn get_model_info(&self, session_id: &SessionId) -> ModelInfo {
        let _ = session_id;
        ModelInfo::default()
    }

    /// RPC-018: return the per-session thinking/reasoning level.
    /// Mirrors `get_model_info` in shape — default returns
    /// `ThinkingLevel::Off`; the codelet/napi `SessionManager` overrides
    /// this in RPC-022 (ThinkingLevel modal dialog).
    fn get_thinking_level(&self, session_id: &SessionId) -> ThinkingLevel {
        let _ = session_id;
        ThinkingLevel::Off
    }

    /// RPC-022: return the available provider/model registry for the
    /// /model modal dialog. Default returns `Vec::new()` so handles
    /// that have not yet wired the model registry — including the
    /// `StubSessionManagerHandle` used by integration tests — compile
    /// without per-test wiring. The concrete codelet/napi
    /// `SessionManager` overrides this to read the cached
    /// `ModelRegistry` and map each provider/model into the
    /// transport-portable `ProviderInfo` / `ModelEntry` shape.
    fn list_providers(&self) -> Vec<ProviderInfo> {
        Vec::new()
    }

    /// RPC-022: set the model bound to a session. Default returns
    /// `Ok(())` (silent no-op) so handles that have not yet wired
    /// model selection — including the stub used by tests — compile
    /// without per-test wiring. The codelet/napi `SessionManager`
    /// overrides this to delegate to the existing
    /// `session_set_model`-style flow (model_string parsing +
    /// `ProviderManager::select_model`).
    fn set_model(
        &self,
        session_id: &SessionId,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let _ = (session_id, provider_id, model_id);
        Ok(())
    }

    /// RPC-022: set the base thinking/reasoning level for a session.
    /// Default returns `Ok(())` (silent no-op). The codelet/napi
    /// override forwards to the existing
    /// `session_set_base_thinking_level` flow.
    fn set_thinking_level(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        let _ = (session_id, level);
        Ok(())
    }

    /// RPC-027: set the PER-USER DEFAULT thinking/reasoning level.
    /// Unlike `set_thinking_level` (which is session-scoped), this
    /// persists the level so new sessions inherit it. Default returns
    /// `Ok(())` (silent no-op). The codelet/napi override forwards
    /// to the future `session_set_default_thinking_level` flow.
    fn set_thinking_level_default(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        let _ = (session_id, level);
        Ok(())
    }

    /// RPC-022: read the session's current role overlay text. Default
    /// returns `None` so handles that have not yet wired role state —
    /// including the stub — compile without per-test wiring. The
    /// codelet/napi override forwards to the existing
    /// `session_get_role` flow (which returns
    /// `Option<SupervisorRoleInfo>` on the JS surface).
    fn get_role(&self, session_id: &SessionId) -> Option<String> {
        let _ = session_id;
        None
    }

    /// RPC-022: set or clear the session's role overlay. Passing
    /// `None` clears. Default returns `Ok(())` (silent no-op). The
    /// codelet/napi override forwards to the existing
    /// `session_set_role` / `session.clear_role` flow.
    fn set_role(
        &self,
        session_id: &SessionId,
        role: Option<String>,
    ) -> Result<(), String> {
        let _ = (session_id, role);
        Ok(())
    }
}

// ============================================================================
// StubSessionManagerHandle — minimal in-memory implementation used by tests
// ============================================================================

/// Minimal in-memory implementation used by integration tests.
///
/// Holds an internal session table and emits a deterministic
/// `[StreamChunk::Text("hi back"), StreamChunk::Done]` sequence on
/// `send_input` regardless of input. Replaces the heavy
/// `SessionManager` for cross-transport tests so the tests don't
/// depend on the full provider/tool dependency tree.
pub struct StubSessionManagerHandle {
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    sessions: Arc<Mutex<Vec<SessionRecord>>>,
    next_id: AtomicU64,
    providers: Arc<Mutex<Vec<ProviderInfo>>>,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    id: SessionId,
    role: Option<String>,
    status: SessionStatus,
}

impl Default for StubSessionManagerHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl StubSessionManagerHandle {
    /// Construct a new stub backed by a deterministic ` [Text, Done]`
    /// emission policy.
    pub fn new() -> Self {
        Self::with_capacity(256, 1024)
    }

    /// Construct a stub matching what an external StubProvider would do —
    /// parameter is ignored; kept for API parity with the test harness.
    pub fn with_provider<P>(_provider: Arc<P>) -> Self {
        Self::new()
    }

    /// Construct a stub with custom broadcast capacities (mostly useful
    /// for stress tests that need bigger buffers).
    pub fn with_capacity(chunks_capacity: usize, logs_capacity: usize) -> Self {
        let (chunks_tx, _) = broadcast::channel(chunks_capacity);
        let (logs_tx, _) = broadcast::channel(logs_capacity);
        Self {
            chunks_tx,
            logs_tx,
            sessions: Arc::new(Mutex::new(Vec::new())),
            next_id: AtomicU64::new(1),
            providers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// RPC-022: pre-seed the provider/model registry returned by
    /// `list_providers`. Used by cross-transport parity tests that need
    /// the stub to return a non-empty registry without dragging in the
    /// real `SessionManager` + `ProviderManager` dependency tree.
    pub fn set_providers(&self, providers: Vec<ProviderInfo>) {
        if let Ok(mut guard) = self.providers.lock() {
            *guard = providers;
        }
    }

    /// Get a clonable handle to the chunks broadcast sender so the host
    /// can dual-fanout chunks (e.g. NAPI ThreadsafeFunction co-listener).
    pub fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        self.chunks_tx.clone()
    }

    /// Get a clonable handle to the logs broadcast sender so the host's
    /// tracing::Layer can push records.
    pub fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        self.logs_tx.clone()
    }

    fn set_status(&self, session_id: &SessionId, status: SessionStatus) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for record in sessions.iter_mut() {
                if record.id == *session_id {
                    record.status = status;
                    return;
                }
            }
        }
    }
}

impl SessionManagerHandle for StubSessionManagerHandle {
    fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = match self.sessions.lock() {
            Ok(sessions) => sessions,
            Err(_) => return Vec::new(),
        };
        sessions
            .iter()
            .map(|r| SessionInfo {
                id: r.id.value.clone(),
                name: r.id.value.clone(),
                status: status_str(r.status).to_string(),
                project: String::new(),
                message_count: 0,
                provider_id: None,
                model_id: None,
                is_isolated: false,
                worktree_path: None,
                role: r.role.clone(),
            })
            .collect()
    }

    fn create_session(&self, role: Option<String>) -> SessionId {
        let id = SessionId::new(format!(
            "stub-{}",
            self.next_id.fetch_add(1, Ordering::SeqCst)
        ));
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.push(SessionRecord {
                id: id.clone(),
                role,
                status: SessionStatus::Idle,
            });
        }
        id
    }

    fn send_input(&self, session_id: &SessionId, _text: String) {
        self.set_status(session_id, SessionStatus::Running);

        let chunks_tx = self.chunks_tx.clone();
        let sid = session_id.clone();
        let sessions = Arc::clone(&self.sessions);
        tokio::spawn(async move {
            // Deterministic stub-provider sequence: [Text("hi back"), Done].
            let _ = chunks_tx.send((sid.clone(), StreamChunk::text("hi back".to_string())));
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            let _ = chunks_tx.send((sid.clone(), StreamChunk::done()));

            // Flip state back to Idle once the stream has completed.
            if let Ok(mut sessions) = sessions.lock() {
                for record in sessions.iter_mut() {
                    if record.id == sid {
                        record.status = SessionStatus::Idle;
                        break;
                    }
                }
            }
        });
    }

    fn interrupt(&self, session_id: &SessionId) {
        self.set_status(session_id, SessionStatus::Interrupted);
        let _ = self
            .chunks_tx
            .send((session_id.clone(), StreamChunk::interrupted(Vec::new())));
    }

    fn get_session_status(&self, session_id: &SessionId) -> SessionStatus {
        let sessions = match self.sessions.lock() {
            Ok(sessions) => sessions,
            Err(_) => return SessionStatus::Idle,
        };
        sessions
            .iter()
            .find(|r| r.id == *session_id)
            .map(|r| r.status)
            .unwrap_or(SessionStatus::Idle)
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.chunks_tx.subscribe()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.logs_tx.subscribe()
    }

    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        self.chunks_tx.clone()
    }

    fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        self.logs_tx.clone()
    }

    fn list_providers(&self) -> Vec<ProviderInfo> {
        match self.providers.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => Vec::new(),
        }
    }
}
