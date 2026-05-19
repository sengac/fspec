//! Reusable fixtures for codelet-fspec-tui integration tests (RPC-008
//! architecture note Q-FIX-1).
//!
//! Fixture module — supports multiple feature files (no single Feature:
//! header). Used by integration tests for fspec-tui-embedded-backend,
//! fspec-tui-ws-backend, and fspec-tui-app-shell.
//!
//! These fixtures construct REAL services — real `WorkUnitsWatcher` over a
//! tempdir, real `SharedFspecService`, real `bind_and_serve` rpc-server
//! when needed — so integration tests exercise actual production code
//! paths rather than mocks. The only "mock" allowed is `MockBackend`
//! (added in a later test) since the FspecBackend trait surface is the
//! NEW code under test in this card and it has both real impls plus a
//! controlled in-memory mock for App-level tests.
//!
//! Per the dev-dependency policy locked in architecture note Q-DEV-CORE-1,
//! `codelet-core` is permitted in `[dev-dependencies]` so fixtures can
//! reach `codelet_core::work_units::WorkUnitsWatcher`. Production
//! `[dependencies]` of codelet/fspec-tui must NOT contain codelet-core —
//! `tests/source_shape.rs` enforces that asymmetry.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use tempfile::TempDir;
use tokio::task::JoinHandle;

/// Default fixture body written into `<tempdir>/spec/work-units.json` —
/// two `WorkUnitInfo` records, mirrored from
/// `codelet/rpc-embedded/tests/embedded_happy_path.rs` so cross-transport
/// assertions in RPC-008 remain comparable to RPC-005's baseline.
pub const SEED_WORK_UNITS_JSON: &str = r#"{"workUnits":{"AUTH-001":{"id":"AUTH-001","title":"User Login","type":"story","status":"done","description":"Sign in with email/password","estimate":5,"epic":"authentication"},"AUTH-002":{"id":"AUTH-002","title":"Password reset","type":"story","status":"implementing","estimate":3,"epic":"authentication"}}}"#;

/// Fixture (1) per Q-FIX-1.
///
/// Build a real tempdir-backed `WorkUnitsWatcher` hosting a real
/// `SharedFspecService`, seeded with the default fixture. The returned
/// [`TempDir`] MUST be kept alive for the lifetime of the test (dropping
/// it removes the underlying spec/work-units.json the watcher tracks).
pub fn temp_service() -> (TempDir, Arc<SharedFspecService>) {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("spec")).expect("mkdir spec/");
    fs::write(
        dir.path().join("spec").join("work-units.json"),
        SEED_WORK_UNITS_JSON,
    )
    .expect("write seed work-units.json");
    let watcher = Arc::new(WorkUnitsWatcher::new(dir.path()).expect("WorkUnitsWatcher::new"));
    let service = Arc::new(SharedFspecService::new(watcher));
    (dir, service)
}

/// Fixture (2) per Q-FIX-1.
///
/// Spawn a real `codelet_rpc_server::bind_and_serve` task bound to
/// `127.0.0.1:0` against the supplied shared service and return the
/// ephemeral [`SocketAddr`] plus the listener's [`JoinHandle`]. Callers
/// MUST keep the join handle alive (typically by binding to `_join`) for
/// the duration of the test — dropping it does not currently abort the
/// task because the join handle returned by `bind_and_serve` is not
/// `abort_on_drop`. Tests that need an explicit shutdown should call
/// `_join.abort()` at the end of the test body.
///
/// The discarded `ServerStats` middle field of the `bind_and_serve`
/// triple is reserved for behavioural assertions in future tests; this
/// fixture keeps the smoke tests focused on transport-agnostic parity.
pub async fn start_ws_server(service: Arc<SharedFspecService>) -> (SocketAddr, JoinHandle<()>) {
    let (addr, _stats, join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve must succeed against 127.0.0.1:0");
    (addr, join)
}

/// RPC-011 variant: also returns the `ServerStats` so the test can call
/// `request_shutdown(stats)` to simulate a daemon-side graceful drain
/// (the per-connection tasks send WS Close{going_away} which propagates
/// to the supervisor).
pub async fn start_ws_server_with_stats(
    service: Arc<SharedFspecService>,
) -> (SocketAddr, codelet_rpc_server::ServerStats, JoinHandle<()>) {
    bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve must succeed against 127.0.0.1:0")
}

/// Build a `ws://127.0.0.1:<port>/` URL for a given socket address —
/// helper for WS-connect tests so each call site doesn't repeat the
/// scheme + path concatenation.
pub fn ws_url(addr: SocketAddr) -> url::Url {
    url::Url::parse(&format!("ws://{addr}/"))
        .expect("ws://<addr>/ is always a valid URL")
}

/// Resolve a path relative to the codelet workspace root.
///
/// `CARGO_MANIFEST_DIR` resolves to `codelet/fspec-tui/` so the workspace
/// root is exactly one level up. Mirrors
/// `codelet/rpc-embedded/tests/source_helpers/mod.rs::workspace_root`
/// — duplicated locally because the existing helper isn't yet a
/// shared dev-dependency-friendly export.
pub fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .expect("fspec-tui crate must have a parent (the codelet workspace root)")
        .to_path_buf()
}

/// Read a file or panic — narrower error surface for source-shape tests
/// that bail on the first failed read.
pub fn read_to_string_or_panic(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("Failed to read {}: {}", path.display(), e);
    })
}

/// Recursively collect every `.rs` file beneath `root`. Mirrors
/// `codelet/rpc-embedded/tests/source_helpers/mod.rs::collect_rs_files`.
pub fn collect_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out
}

/// Strip both `//` line comments and `/* … */` block comments. Mirrors
/// `codelet/rpc-embedded/tests/source_helpers/mod.rs::strip_rust_comments`
/// so the source-shape regressions stay byte-equivalent across crates.
pub fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && next == Some(b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────
// Fixture (3) per Q-FIX-1: MockBackend (extended in RPC-009 with scripted
// create_session/send_input/interrupt + per-call counters + chunks_tx
// publisher per architecture note [10]).
// ─────────────────────────────────────────────────────────────────────────

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::FspecBackend;
use codelet_rpc_types::{
    CheckpointCounts, LogRecord, ModelInfo, ProviderInfo, SessionId, SessionInfo, StreamChunk,
    ThinkingLevel, WorkUnitInfo, WorkspaceInfo,
};
use tokio::sync::broadcast;

/// In-memory FspecBackend impl with seedable data + per-channel
/// broadcast::Sender handles tests use to push synthetic events. Used
/// by the App-level integration tests where a real WS server / real
/// service is overkill — the goal is to exercise the App's wiring
/// against `Arc<dyn FspecBackend>`, not the transport itself.
///
/// RPC-009 extensions (architecture note [10]):
///   - `script_create_session(SessionId)` — preload the SessionId the
///     next `create_session` call returns (replaces the RPC-008 `bail!`).
///   - per-call counters: `list_work_units_calls`, `create_session_calls`,
///     `send_input_calls`, `interrupt_calls`.
///   - `last_send_input` / `last_interrupt` — capture the most recent
///     argument tuple for assertion.
///   - `push_chunk(SessionId, StreamChunk)` — fire a chunk on the
///     broadcast so subscriber tests can drive scripted streams.
pub struct MockBackend {
    work_units: Mutex<Vec<WorkUnitInfo>>,
    sessions: Mutex<Vec<SessionInfo>>,
    work_units_tx: broadcast::Sender<Vec<WorkUnitInfo>>,
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    list_work_units_calls: AtomicUsize,
    create_session_calls: AtomicUsize,
    send_input_calls: AtomicUsize,
    interrupt_calls: AtomicUsize,
    checkpoint_counts_calls: AtomicUsize,
    /// RPC-017: per-call counters for the reorder methods + record of
    /// the most recently passed id so App-level dispatch tests can
    /// assert that `Action::ReorderUp`/`Down` routes to the focused-
    /// column selection.
    move_work_unit_up_calls: AtomicUsize,
    move_work_unit_down_calls: AtomicUsize,
    last_move_work_unit_up_id: Mutex<Option<String>>,
    last_move_work_unit_down_id: Mutex<Option<String>>,
    scripted_session: Mutex<Option<SessionId>>,
    last_send_input: Mutex<Option<(SessionId, String)>>,
    last_interrupt: Mutex<Option<SessionId>>,
    checkpoint_counts: Mutex<CheckpointCounts>,
    /// RPC-018: scripted ModelInfo returned by `get_model_info`.
    model_info: Mutex<ModelInfo>,
    /// RPC-018: scripted ThinkingLevel returned by `get_thinking_level`.
    thinking_level: Mutex<ThinkingLevel>,
    /// RPC-018: scripted WorkspaceInfo returned by `get_workspace_info`.
    workspace_info: Mutex<WorkspaceInfo>,
    /// RPC-018: when `Some`, `get_workspace_info` returns
    /// `Err(anyhow!(message))` so bootstrap-best-effort scenarios can
    /// exercise the failure branch.
    workspace_info_error: Mutex<Option<String>>,
    /// RPC-020: scripted file-search results returned by
    /// `search_files`. The mock filters by case-insensitive substring
    /// of `prefix` and caps at `limit`.
    file_search_results: Mutex<Vec<String>>,
    /// RPC-026: per-call counter for `persistence_delete_session`.
    delete_session_calls: AtomicUsize,
    /// RPC-026: capture of the last id passed to
    /// `persistence_delete_session`.
    last_deleted_session: Mutex<Option<SessionId>>,
    /// RPC-026: scripted history-search results returned by
    /// `persistence_search_history`. Indexed by query string.
    history_search_results: Mutex<Vec<codelet_rpc_types::HistoryMatch>>,
    /// RPC-026: per-call counter for `persistence_search_history`.
    search_history_calls: AtomicUsize,
    /// RPC-026: capture of the last query passed to
    /// `persistence_search_history`.
    last_history_query: Mutex<Option<String>>,
    /// RPC-022: scripted providers returned by `list_providers`.
    providers: Mutex<Vec<ProviderInfo>>,
    /// RPC-022: counters + captures for the five new RPC methods.
    list_providers_calls: AtomicUsize,
    set_session_model_calls: AtomicUsize,
    set_thinking_level_calls: AtomicUsize,
    get_session_role_calls: AtomicUsize,
    set_session_role_calls: AtomicUsize,
    last_set_session_model: Mutex<Option<(SessionId, String, String)>>,
    last_set_thinking_level: Mutex<Option<(SessionId, ThinkingLevel)>>,
    last_get_session_role: Mutex<Option<SessionId>>,
    last_set_session_role: Mutex<Option<(SessionId, Option<String>)>>,
    /// RPC-022: scripted per-session role overlay returned by
    /// `get_session_role` and overwritten by `set_session_role`. Mutex
    /// of a Vec because tests sometimes seed multiple sessions.
    session_roles: Mutex<Vec<(SessionId, Option<String>)>>,
    /// RPC-022: per-call counter for `persistence_add_history`.
    persistence_add_history_calls: AtomicUsize,
    /// RPC-022: capture of the last `(SessionId, text)` passed to
    /// `persistence_add_history`.
    last_persistence_add_history: Mutex<Option<(SessionId, String)>>,
}

impl Default for MockBackend {
    fn default() -> Self {
        let (work_units_tx, _) = broadcast::channel(64);
        let (chunks_tx, _) = broadcast::channel(64);
        let (logs_tx, _) = broadcast::channel(64);
        Self {
            work_units: Mutex::new(Vec::new()),
            sessions: Mutex::new(Vec::new()),
            work_units_tx,
            chunks_tx,
            logs_tx,
            list_work_units_calls: AtomicUsize::new(0),
            create_session_calls: AtomicUsize::new(0),
            send_input_calls: AtomicUsize::new(0),
            interrupt_calls: AtomicUsize::new(0),
            checkpoint_counts_calls: AtomicUsize::new(0),
            move_work_unit_up_calls: AtomicUsize::new(0),
            move_work_unit_down_calls: AtomicUsize::new(0),
            last_move_work_unit_up_id: Mutex::new(None),
            last_move_work_unit_down_id: Mutex::new(None),
            scripted_session: Mutex::new(None),
            last_send_input: Mutex::new(None),
            last_interrupt: Mutex::new(None),
            checkpoint_counts: Mutex::new(CheckpointCounts::default()),
            model_info: Mutex::new(ModelInfo::default()),
            thinking_level: Mutex::new(ThinkingLevel::Off),
            workspace_info: Mutex::new(WorkspaceInfo::default()),
            workspace_info_error: Mutex::new(None),
            file_search_results: Mutex::new(Vec::new()),
            delete_session_calls: AtomicUsize::new(0),
            last_deleted_session: Mutex::new(None),
            history_search_results: Mutex::new(Vec::new()),
            search_history_calls: AtomicUsize::new(0),
            last_history_query: Mutex::new(None),
            providers: Mutex::new(Vec::new()),
            list_providers_calls: AtomicUsize::new(0),
            set_session_model_calls: AtomicUsize::new(0),
            set_thinking_level_calls: AtomicUsize::new(0),
            get_session_role_calls: AtomicUsize::new(0),
            set_session_role_calls: AtomicUsize::new(0),
            last_set_session_model: Mutex::new(None),
            last_set_thinking_level: Mutex::new(None),
            last_get_session_role: Mutex::new(None),
            last_set_session_role: Mutex::new(None),
            session_roles: Mutex::new(Vec::new()),
            persistence_add_history_calls: AtomicUsize::new(0),
            last_persistence_add_history: Mutex::new(None),
        }
    }
}

impl MockBackend {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the in-memory work_units with the supplied list.
    pub fn seed_work_units(&self, units: Vec<WorkUnitInfo>) {
        *self.work_units.lock().expect("MockBackend mutex") = units;
    }

    /// Push a fresh work-units snapshot onto the broadcast channel.
    pub fn push_work_units(&self, units: Vec<WorkUnitInfo>) {
        let _ = self.work_units_tx.send(units);
    }

    /// Script the next `create_session` call to return this SessionId.
    pub fn script_create_session(&self, id: SessionId) {
        *self.scripted_session.lock().expect("MockBackend mutex") = Some(id);
    }

    /// Push a chunk onto the chunks broadcast (RPC-009 test helper).
    pub fn push_chunk(&self, id: SessionId, chunk: StreamChunk) {
        let _ = self.chunks_tx.send((id, chunk));
    }

    pub fn list_work_units_calls(&self) -> usize {
        self.list_work_units_calls.load(Ordering::SeqCst)
    }
    pub fn create_session_calls(&self) -> usize {
        self.create_session_calls.load(Ordering::SeqCst)
    }
    pub fn send_input_calls(&self) -> usize {
        self.send_input_calls.load(Ordering::SeqCst)
    }
    pub fn interrupt_calls(&self) -> usize {
        self.interrupt_calls.load(Ordering::SeqCst)
    }
    pub fn last_send_input(&self) -> Option<(SessionId, String)> {
        self.last_send_input.lock().expect("MockBackend mutex").clone()
    }
    pub fn last_interrupt(&self) -> Option<SessionId> {
        self.last_interrupt.lock().expect("MockBackend mutex").clone()
    }

    /// RPC-015: preload the CheckpointCounts the next `checkpoint_counts`
    /// call returns.
    pub fn set_checkpoint_counts(&self, counts: CheckpointCounts) {
        *self.checkpoint_counts.lock().expect("MockBackend mutex") = counts;
    }

    /// RPC-015: how many times `checkpoint_counts()` has been awaited.
    pub fn checkpoint_counts_calls(&self) -> usize {
        self.checkpoint_counts_calls.load(Ordering::SeqCst)
    }

    /// RPC-017: counter + capture for `move_work_unit_up`.
    pub fn move_work_unit_up_calls(&self) -> usize {
        self.move_work_unit_up_calls.load(Ordering::SeqCst)
    }

    /// RPC-017: counter + capture for `move_work_unit_down`.
    pub fn move_work_unit_down_calls(&self) -> usize {
        self.move_work_unit_down_calls.load(Ordering::SeqCst)
    }

    /// RPC-017: the last id passed to `move_work_unit_up`.
    pub fn last_move_work_unit_up_id(&self) -> Option<String> {
        self.last_move_work_unit_up_id
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-017: the last id passed to `move_work_unit_down`.
    pub fn last_move_work_unit_down_id(&self) -> Option<String> {
        self.last_move_work_unit_down_id
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-018: preload the ModelInfo the next `get_model_info` call returns.
    pub fn set_model_info(&self, info: ModelInfo) {
        *self.model_info.lock().expect("MockBackend mutex") = info;
    }

    /// RPC-018: preload the ThinkingLevel the next `get_thinking_level` call returns.
    pub fn set_thinking_level(&self, level: ThinkingLevel) {
        *self.thinking_level.lock().expect("MockBackend mutex") = level;
    }

    /// RPC-018: preload the WorkspaceInfo the next `get_workspace_info` call returns.
    pub fn set_workspace_info(&self, info: WorkspaceInfo) {
        *self.workspace_info.lock().expect("MockBackend mutex") = info;
    }

    /// RPC-018: force the next `get_workspace_info` call to fail with the
    /// supplied message — exercises the bootstrap best-effort branch.
    pub fn set_workspace_info_error(&self, message: String) {
        *self.workspace_info_error.lock().expect("MockBackend mutex") = Some(message);
    }

    /// RPC-020: preload the file-search result list. `search_files`
    /// filters this Vec case-insensitively against its `prefix` arg.
    pub fn set_file_search_results(&self, paths: Vec<String>) {
        *self.file_search_results.lock().expect("MockBackend mutex") = paths;
    }

    /// RPC-026: how many times `persistence_delete_session` was awaited.
    pub fn delete_session_calls(&self) -> usize {
        self.delete_session_calls.load(Ordering::SeqCst)
    }

    /// RPC-026: the last id passed to `persistence_delete_session`.
    pub fn last_deleted_session(&self) -> Option<SessionId> {
        self.last_deleted_session
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-026: preload the result list returned by
    /// `persistence_search_history`. The same Vec is returned for
    /// every query — tests script the desired outcome up front.
    pub fn set_history_search_results(
        &self,
        results: Vec<codelet_rpc_types::HistoryMatch>,
    ) {
        *self.history_search_results.lock().expect("MockBackend mutex") = results;
    }

    /// RPC-026: how many times `persistence_search_history` was awaited.
    pub fn search_history_calls(&self) -> usize {
        self.search_history_calls.load(Ordering::SeqCst)
    }

    /// RPC-026: the last query passed to `persistence_search_history`.
    pub fn last_history_query(&self) -> Option<String> {
        self.last_history_query
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-026: seed the in-memory `sessions` list returned by
    /// `list_sessions`. Tests use this to script the resume picker.
    pub fn seed_sessions(&self, sessions: Vec<SessionInfo>) {
        *self.sessions.lock().expect("MockBackend mutex") = sessions;
    }

    /// RPC-022: preload the provider registry returned by
    /// `list_providers`.
    pub fn seed_providers(&self, providers: Vec<ProviderInfo>) {
        *self.providers.lock().expect("MockBackend mutex") = providers;
    }

    /// RPC-022: how many times `list_providers` was awaited.
    pub fn list_providers_calls(&self) -> usize {
        self.list_providers_calls.load(Ordering::SeqCst)
    }

    /// RPC-022: how many times `set_session_model` was awaited.
    pub fn set_session_model_calls(&self) -> usize {
        self.set_session_model_calls.load(Ordering::SeqCst)
    }

    /// RPC-022: how many times `set_thinking_level` was awaited.
    pub fn set_thinking_level_calls(&self) -> usize {
        self.set_thinking_level_calls.load(Ordering::SeqCst)
    }

    /// RPC-022: how many times `get_session_role` was awaited.
    pub fn get_session_role_calls(&self) -> usize {
        self.get_session_role_calls.load(Ordering::SeqCst)
    }

    /// RPC-022: how many times `set_session_role` was awaited.
    pub fn set_session_role_calls(&self) -> usize {
        self.set_session_role_calls.load(Ordering::SeqCst)
    }

    /// RPC-022: the last `(SessionId, provider_id, model_id)` triple
    /// passed to `set_session_model`.
    pub fn last_set_session_model(&self) -> Option<(SessionId, String, String)> {
        self.last_set_session_model
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-022: the last `(SessionId, ThinkingLevel)` pair passed to
    /// `set_thinking_level`.
    pub fn last_set_thinking_level(&self) -> Option<(SessionId, ThinkingLevel)> {
        self.last_set_thinking_level
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-022: the last SessionId passed to `get_session_role`.
    pub fn last_get_session_role(&self) -> Option<SessionId> {
        self.last_get_session_role
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-022: the last `(SessionId, Option<String>)` passed to
    /// `set_session_role`.
    pub fn last_set_session_role(&self) -> Option<(SessionId, Option<String>)> {
        self.last_set_session_role
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-022: preload `(SessionId, role)` pairs so
    /// `get_session_role(id)` returns the seeded role overlay (None
    /// when the SessionId is not in the table).
    pub fn seed_session_role(&self, session: SessionId, role: Option<String>) {
        let mut roles = self.session_roles.lock().expect("MockBackend mutex");
        roles.retain(|(s, _)| s != &session);
        roles.push((session, role));
    }

    /// RPC-022: how many times `persistence_add_history` was awaited.
    pub fn persistence_add_history_calls(&self) -> usize {
        self.persistence_add_history_calls.load(Ordering::SeqCst)
    }

    /// RPC-022: the last `(SessionId, text)` pair passed to
    /// `persistence_add_history`.
    pub fn last_persistence_add_history(&self) -> Option<(SessionId, String)> {
        self.last_persistence_add_history
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }
}

#[async_trait]
impl FspecBackend for MockBackend {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        self.list_work_units_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.work_units.lock().expect("MockBackend mutex").clone())
    }

    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        Ok(self.sessions.lock().expect("MockBackend mutex").clone())
    }

    async fn create_session(&self, _role: Option<String>) -> Result<SessionId> {
        self.create_session_calls.fetch_add(1, Ordering::SeqCst);
        let scripted = self
            .scripted_session
            .lock()
            .expect("MockBackend mutex")
            .clone();
        Ok(scripted.unwrap_or_else(|| SessionId::new("s-mock-default")))
    }

    async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        self.send_input_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_send_input.lock().expect("MockBackend mutex") =
            Some((id, text));
        Ok(())
    }

    async fn interrupt(&self, id: SessionId) -> Result<()> {
        self.interrupt_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_interrupt.lock().expect("MockBackend mutex") = Some(id);
        Ok(())
    }

    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.work_units_tx.subscribe()
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.chunks_tx.subscribe()
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.logs_tx.subscribe()
    }

    async fn health(&self) -> Result<codelet_rpc_types::HealthInfo> {
        Ok(codelet_rpc_types::HealthInfo {
            uptime_secs: 0,
            connected_clients: 0,
            last_watcher_event_secs_ago: None,
            lag_chunks: 0,
            lag_logs: 0,
            lag_work_units: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    async fn checkpoint_counts(&self) -> Result<CheckpointCounts> {
        self.checkpoint_counts_calls.fetch_add(1, Ordering::SeqCst);
        Ok(*self.checkpoint_counts.lock().expect("MockBackend mutex"))
    }

    async fn move_work_unit_up(&self, id: String) -> Result<()> {
        self.move_work_unit_up_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_move_work_unit_up_id
            .lock()
            .expect("MockBackend mutex") = Some(id);
        Ok(())
    }

    async fn move_work_unit_down(&self, id: String) -> Result<()> {
        self.move_work_unit_down_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_move_work_unit_down_id
            .lock()
            .expect("MockBackend mutex") = Some(id);
        Ok(())
    }

    async fn get_model_info(&self, _session_id: SessionId) -> Result<ModelInfo> {
        Ok(self.model_info.lock().expect("MockBackend mutex").clone())
    }

    async fn get_thinking_level(&self, _session_id: SessionId) -> Result<ThinkingLevel> {
        Ok(*self.thinking_level.lock().expect("MockBackend mutex"))
    }

    async fn get_workspace_info(&self) -> Result<WorkspaceInfo> {
        if let Some(msg) = self
            .workspace_info_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(self.workspace_info.lock().expect("MockBackend mutex").clone())
    }

    async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>> {
        // RPC-020: MockBackend returns scripted matches (or an empty
        // Vec) so tests can drive the file search popup without
        // touching the real filesystem.
        let all = self.file_search_results.lock().expect("MockBackend mutex").clone();
        let filtered: Vec<String> = all
            .into_iter()
            .filter(|p| p.to_lowercase().contains(&prefix.to_lowercase()))
            .take(limit as usize)
            .collect();
        Ok(filtered)
    }

    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
        self.persistence_add_history_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_persistence_add_history
            .lock()
            .expect("MockBackend mutex") = Some((session, text));
        Ok(())
    }

    async fn persistence_get_history(
        &self,
        _session: SessionId,
        _limit: u32,
    ) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn persistence_search_history(
        &self,
        query: String,
    ) -> Result<Vec<codelet_rpc_types::HistoryMatch>> {
        self.search_history_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_history_query.lock().expect("MockBackend mutex") = Some(query);
        Ok(self.history_search_results.lock().expect("MockBackend mutex").clone())
    }

    async fn persistence_delete_session(&self, id: SessionId) -> Result<()> {
        self.delete_session_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_deleted_session.lock().expect("MockBackend mutex") = Some(id.clone());
        let mut sessions = self.sessions.lock().expect("MockBackend mutex");
        sessions.retain(|s| s.id != id.value);
        Ok(())
    }

    async fn list_providers(&self) -> Result<Vec<ProviderInfo>> {
        self.list_providers_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.providers.lock().expect("MockBackend mutex").clone())
    }

    async fn set_session_model(
        &self,
        session_id: SessionId,
        provider_id: String,
        model_id: String,
    ) -> Result<()> {
        self.set_session_model_calls.fetch_add(1, Ordering::SeqCst);
        *self
            .last_set_session_model
            .lock()
            .expect("MockBackend mutex") = Some((session_id, provider_id, model_id));
        Ok(())
    }

    async fn set_thinking_level(
        &self,
        session_id: SessionId,
        level: ThinkingLevel,
    ) -> Result<()> {
        self.set_thinking_level_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_set_thinking_level
            .lock()
            .expect("MockBackend mutex") = Some((session_id, level));
        Ok(())
    }

    async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>> {
        self.get_session_role_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_get_session_role.lock().expect("MockBackend mutex") =
            Some(session_id.clone());
        let roles = self.session_roles.lock().expect("MockBackend mutex");
        let role = roles
            .iter()
            .find(|(s, _)| s == &session_id)
            .and_then(|(_, r)| r.clone());
        Ok(role)
    }

    async fn set_session_role(
        &self,
        session_id: SessionId,
        role: Option<String>,
    ) -> Result<()> {
        self.set_session_role_calls.fetch_add(1, Ordering::SeqCst);
        *self
            .last_set_session_role
            .lock()
            .expect("MockBackend mutex") = Some((session_id.clone(), role.clone()));
        // Mirror the SessionManager production behaviour: overwrite the
        // overlay when set, drop it when cleared.
        let mut roles = self.session_roles.lock().expect("MockBackend mutex");
        roles.retain(|(s, _)| s != &session_id);
        roles.push((session_id, role));
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Fixture (4) per Q-FIX-1: test_app — App + 80x24 TestBackend pair
// ─────────────────────────────────────────────────────────────────────────

use codelet_fspec_tui::App;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// Construct an [`App`] alongside an 80x24 [`Terminal<TestBackend>`]
/// suitable for App-level integration tests.
pub fn test_app(backend: Arc<dyn FspecBackend>) -> (App, Terminal<TestBackend>) {
    let app = App::new(backend);
    let terminal_backend = TestBackend::new(80, 24);
    let terminal = Terminal::new(terminal_backend).expect("Terminal::new(TestBackend)");
    (app, terminal)
}

// ─────────────────────────────────────────────────────────────────────────
// Fixture (5) per Q-FIX-1: render_one_frame
// ─────────────────────────────────────────────────────────────────────────

use ratatui::buffer::Buffer;

/// Drive a single render cycle of `app` against `terminal` and return
/// a clone of the resulting [`Buffer`] for snapshotting.
pub fn render_one_frame(terminal: &mut Terminal<TestBackend>, app: &mut App) -> Buffer {
    terminal
        .draw(|frame| {
            app.render(frame.area(), frame.buffer_mut());
        })
        .expect("Terminal::draw");
    terminal.backend().buffer().clone()
}

/// Convert a [`Buffer`] into a Vec<String> of row text — one entry per
/// row, suitable for `insta::assert_yaml_snapshot!`.
pub fn buffer_to_rows(buf: &Buffer) -> Vec<String> {
    let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::with_capacity(buf.area.width as usize);
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}
