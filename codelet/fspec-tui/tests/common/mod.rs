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
use codelet_rpc_types::{LogRecord, SessionId, SessionInfo, StreamChunk, WorkUnitInfo};
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
    scripted_session: Mutex<Option<SessionId>>,
    last_send_input: Mutex<Option<(SessionId, String)>>,
    last_interrupt: Mutex<Option<SessionId>>,
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
            scripted_session: Mutex::new(None),
            last_send_input: Mutex::new(None),
            last_interrupt: Mutex::new(None),
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
