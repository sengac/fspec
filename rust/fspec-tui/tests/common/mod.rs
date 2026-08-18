//! Reusable fixtures for codelet-fspec-tui integration tests (RPC-008
//! architecture note Q-FIX-1).
//!
//! Fixture module — supports multiple feature files (no single Feature:
//! header). Used by integration tests for fspec-tui-embedded-backend,
//! fspec-tui-ws-backend, and fspec-tui-app-shell.
//!
//! RPC-065: also exposes the `harness` sub-module which provides the
//! reusable `AppTestHarness` consumed by `behaviour_parity_rpc065.rs`.
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
//! `[dependencies]` of rust/fspec-tui must NOT contain codelet-core —
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
/// `rust/rpc-embedded/tests/embedded_happy_path.rs` so cross-transport
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
    url::Url::parse(&format!("ws://{addr}/")).expect("ws://<addr>/ is always a valid URL")
}

/// Resolve a path relative to the codelet workspace root.
///
/// `CARGO_MANIFEST_DIR` resolves to `rust/fspec-tui/` so the workspace
/// root is exactly one level up. Mirrors
/// `rust/rpc-embedded/tests/source_helpers/mod.rs::workspace_root`
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
/// `rust/rpc-embedded/tests/source_helpers/mod.rs::collect_rs_files`.
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
/// `rust/rpc-embedded/tests/source_helpers/mod.rs::strip_rust_comments`
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

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::FspecBackend;
use codelet_rpc_types::{
    ApprovalChoice, BlocklistRuleInfo, CheckpointCounts, CompactionResult, FspecResult,
    HitlRequest, HitlResponse, IncomingMessageInput, IsolatedSessionInfo, LogRecord, ModelEntry,
    ModelInfo, PauseState, ProviderCredentialInfo, ProviderCredentialInput, ProviderInfo,
    SessionId, SessionInfo, SessionStatus, StreamChunk, TestConnectionResult, ThinkingLevel,
    WorkUnitContext, WorkUnitInfo, WorkspaceInfo,
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
    /// RPC-415: wrapped in `Mutex<Option>` (mirroring `chunks_tx`) so the
    /// reconnect-resubscribe tests can drop the Sender via
    /// `disconnect_all()` (subscribers observe `RecvError::Closed`) and
    /// then install a FRESH Sender via `reconnect_all()` — modelling the
    /// transport supervisor swapping in a brand-new RPC client whose
    /// broadcast senders are distinct from the old (dropped) client's.
    work_units_tx: Mutex<Option<broadcast::Sender<Vec<WorkUnitInfo>>>>,
    /// RPC-045: wrapped in `Mutex<Option>` so tests can deliberately
    /// drop the Sender via `close_chunks_tx()` to simulate a
    /// SessionManager shutdown and assert the subscriber loop exits
    /// without panicking.
    chunks_tx: Mutex<Option<broadcast::Sender<(SessionId, StreamChunk)>>>,
    /// RPC-415: `Mutex<Option>` for the same disconnect/reconnect swap as
    /// `work_units_tx`.
    logs_tx: Mutex<Option<broadcast::Sender<LogRecord>>>,
    /// RPC-045: push-driven (SessionId, SessionStatus) broadcast Sender.
    /// Tests use `push_status_change` to drive synthetic transitions
    /// without going through a real SessionManager.
    ///
    /// RPC-415: `Mutex<Option>` for the disconnect/reconnect swap.
    status_changes_tx: Mutex<Option<broadcast::Sender<(SessionId, SessionStatus)>>>,
    /// RPC-385: push-driven session-created broadcast Sender. Tests use
    /// `push_session_created` to drive synthetic session-creation events
    /// (e.g. an AgentManager-spawned subordinate) without a real
    /// SessionManager. The capacity is intentionally small (matching the
    /// other broadcast channels) so the lag-recovery scenario can overflow
    /// it and force `RecvError::Lagged`.
    ///
    /// RPC-415: `Mutex<Option>` for the disconnect/reconnect swap.
    session_created_tx: Mutex<Option<broadcast::Sender<SessionInfo>>>,
    /// TUI-109: push-driven checkpoint-enumeration progress broadcast
    /// Sender. Tests use `push_checkpoints_progress` to drive synthetic
    /// `CheckpointsProgress` frames without a real RPC server.
    checkpoints_progress_tx: Mutex<
        Option<broadcast::Sender<codelet_rpc_types::CheckpointsProgress>>,
    >,
    list_work_units_calls: AtomicUsize,
    create_session_calls: AtomicUsize,
    send_input_calls: AtomicUsize,
    interrupt_calls: AtomicUsize,
    checkpoint_counts_calls: AtomicUsize,
    /// RPC-365: per-call counters + last-path capture for the restore
    /// transport methods so App-dispatch tests can assert the wiring.
    restore_checkpoint_file_calls: AtomicUsize,
    restore_checkpoint_all_calls: AtomicUsize,
    last_restore_file: Mutex<Option<(String, String, String)>>,
    /// RPC-366: per-call counters + last-key capture for the delete
    /// transport methods so App-dispatch tests can assert the wiring.
    delete_checkpoint_calls: AtomicUsize,
    delete_all_checkpoints_calls: AtomicUsize,
    last_delete_checkpoint: Mutex<Option<(String, String)>>,
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
    /// RPC-098: counter incremented every time `destroy_session` is called
    /// via the FspecBackend trait. Lets ESC exit-confirmation tests assert
    /// that Close Session reaches the backend and Detach/Cancel do NOT.
    destroy_session_calls: AtomicUsize,
    /// RPC-098: most recently destroyed SessionId — paired counterpart to
    /// `destroy_session_calls`.
    last_destroyed_session: Mutex<Option<SessionId>>,
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
    /// RPC-427: capture of the last project_path passed to
    /// `list_sessions`. Tests use `list_sessions_project()` to
    /// assert that the TUI passes the current working directory.
    last_list_sessions_project: Mutex<Option<String>>,
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
    /// PROV-118: per-call counter + capture for `set_default_model`.
    set_default_model_calls: AtomicUsize,
    last_set_default_model: Mutex<Option<String>>,
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
    /// RPC-045: per-call counter for `send_fspec_result`.
    send_fspec_result_calls: AtomicUsize,
    /// RPC-045: capture of the last `FspecResult` passed to
    /// `send_fspec_result`. The matching `SessionId` is co-stored so
    /// tests assert both pieces of the round-trip.
    last_fspec_result: Mutex<Option<(SessionId, FspecResult)>>,
    /// RPC-046: per-call counter for `clear_history`.
    clear_history_calls: AtomicUsize,
    /// RPC-046: capture of the last SessionId passed to
    /// `clear_history`.
    last_clear_history_session: Mutex<Option<SessionId>>,
    /// RPC-046: when `Some`, `clear_history` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the error-notice branch.
    clear_history_error: Mutex<Option<String>>,
    /// RPC-047: per-call counter for `compact_session`.
    compact_session_calls: AtomicUsize,
    /// RPC-047: capture of the last SessionId passed to
    /// `compact_session`.
    last_compact_session: Mutex<Option<SessionId>>,
    /// RPC-047: scripted result returned by `compact_session`. Defaults
    /// to a 1.0 compression-ratio Ok response so scenarios that don't
    /// care about the body still get a well-formed reply.
    compact_session_result: Mutex<Result<CompactionResult, String>>,
    /// RPC-049: per-call counter for `resume_session`.
    resume_session_calls: AtomicUsize,
    /// RPC-049: capture of the last SessionId passed to `resume_session`.
    last_resume_session: Mutex<Option<SessionId>>,
    /// RPC-049: when `Some`, `resume_session` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the error-notice branch.
    resume_session_error: Mutex<Option<String>>,
    /// RPC-049: scripted replay-buffer returned by `get_buffered_output`.
    buffered_output: Mutex<Vec<StreamChunk>>,
    /// RPC-049: per-call counter for `get_buffered_output`.
    get_buffered_output_calls: AtomicUsize,
    /// RPC-049: capture of the last (id, limit) pair passed to
    /// `get_buffered_output`.
    last_get_buffered_output: Mutex<Option<(SessionId, u32)>>,
    /// RPC-050: per-call counter for `set_work_unit_context`.
    set_work_unit_context_calls: AtomicUsize,
    /// RPC-050: per-call counter for `get_work_unit_context`.
    get_work_unit_context_calls: AtomicUsize,
    /// RPC-050: capture of the last `(SessionId, Option<WorkUnitContext>)`
    /// passed to `set_work_unit_context`.
    last_set_work_unit_context: Mutex<Option<(SessionId, Option<WorkUnitContext>)>>,
    /// RPC-050: in-memory work-unit context store so `get_work_unit_context`
    /// reads back what `set_work_unit_context` wrote. Mirrors the
    /// production StubSessionManagerHandle's `work_unit_ctx` map.
    work_unit_contexts: Mutex<HashMap<SessionId, WorkUnitContext>>,
    /// RPC-050: when `Some`, `set_work_unit_context` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the error-notice branch without touching the in-memory store.
    set_work_unit_context_error: Mutex<Option<String>>,
    /// RPC-051: per-session scripted return for
    /// `persistence_get_history`. Tests use `script_history` to seed
    /// the snapshot the App's Shift+↑ first-press fetch task receives.
    scripted_history: Mutex<HashMap<SessionId, Vec<String>>>,
    /// RPC-052: per-call counter for `set_pending_input`.
    set_pending_input_calls: AtomicUsize,
    /// RPC-052: per-call counter for `get_pending_input`.
    get_pending_input_calls: AtomicUsize,
    /// RPC-052: capture of every `(SessionId, Option<String>)` passed to
    /// `set_pending_input` in call order so the debounce-coalescing
    /// scenarios can assert how many distinct writes landed AND the
    /// final value the backend was left holding.
    pending_input_writes: Mutex<Vec<(SessionId, Option<String>)>>,
    /// RPC-052: in-memory pending-input store so `get_pending_input`
    /// reads back what `set_pending_input` wrote AND so test fixtures
    /// can seed a value via `script_pending_input`.
    pending_input_store: Mutex<HashMap<SessionId, Option<String>>>,
    /// RPC-052: when `Some`, `get_pending_input` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the hydration error branch.
    get_pending_input_error: Mutex<Option<String>>,
    /// RPC-052: when `Some`, `set_pending_input` returns
    /// `Err(anyhow!(message))` so failure-path scenarios can exercise
    /// the silent-drop branch.
    set_pending_input_error: Mutex<Option<String>>,
    // ── RPC-053 pause / HITL surface ─────────────────────────────────
    /// RPC-053: per-session scripted pause-state return from
    /// `get_pause_state`. None when no pause is active for the session.
    pause_state_store: Mutex<HashMap<SessionId, Option<PauseState>>>,
    /// RPC-053: per-session scripted hitl-request return from
    /// `get_hitl_request`. None when no HITL request is pending.
    hitl_request_store: Mutex<HashMap<SessionId, Option<HitlRequest>>>,
    /// RPC-053: per-call counters for the six pause / HITL methods.
    get_pause_state_calls: AtomicUsize,
    get_hitl_request_calls: AtomicUsize,
    pause_resume_calls: AtomicUsize,
    pause_confirm_calls: AtomicUsize,
    pause_triple_calls: AtomicUsize,
    send_hitl_response_calls: AtomicUsize,
    /// RPC-053: capture of every `(SessionId, accept)` passed to
    /// `pause_confirm` in call order.
    pause_confirm_calls_log: Mutex<Vec<(SessionId, bool)>>,
    /// RPC-053: capture of every `(SessionId, ApprovalChoice)` passed
    /// to `pause_triple` in call order.
    pause_triple_calls_log: Mutex<Vec<(SessionId, ApprovalChoice)>>,
    /// RPC-053: capture of every SessionId passed to `pause_resume`.
    pause_resume_calls_log: Mutex<Vec<SessionId>>,
    /// RPC-053: capture of every `(SessionId, HitlResponse)` passed to
    /// `send_hitl_response` in call order.
    send_hitl_response_calls_log: Mutex<Vec<(SessionId, HitlResponse)>>,
    /// RPC-053: scripted error slots — when `Some`, the matching
    /// method returns `Err(anyhow!(message))`.
    get_pause_state_error: Mutex<Option<String>>,
    get_hitl_request_error: Mutex<Option<String>>,
    pause_resume_error: Mutex<Option<String>>,
    pause_confirm_error: Mutex<Option<String>>,
    pause_triple_error: Mutex<Option<String>>,
    send_hitl_response_error: Mutex<Option<String>>,
    // ── RPC-054 provider-credentials surface ─────────────────────────
    /// RPC-054: ordered list of provider credential infos returned by
    /// `list_provider_credentials`. Tests use `seed_provider_credentials`
    /// to set the initial list AND `set_post_save_provider_credentials`
    /// / `set_post_delete_provider_credentials` / `set_post_refresh_provider_credentials`
    /// to script the next list returned after the App's follow-up refresh.
    provider_credentials: Mutex<Vec<ProviderCredentialInfo>>,
    /// RPC-054: next-list-after-save override. When `Some`, the very next
    /// `list_provider_credentials` call after `set_provider_credentials`
    /// returns this value instead of the seeded list, then clears the
    /// override.
    provider_credentials_after_save: Mutex<Option<Vec<ProviderCredentialInfo>>>,
    /// RPC-054: next-list-after-delete override.
    provider_credentials_after_delete: Mutex<Option<Vec<ProviderCredentialInfo>>>,
    /// RPC-054: next-list-after-refresh override.
    provider_credentials_after_refresh: Mutex<Option<Vec<ProviderCredentialInfo>>>,
    /// RPC-054: when set, the next `list_provider_credentials` call
    /// returns this override (one-shot).
    next_list_override: Mutex<Option<Vec<ProviderCredentialInfo>>>,
    /// RPC-054: per-call counters for the credential surface.
    list_provider_credentials_calls: AtomicUsize,
    get_provider_credential_calls: AtomicUsize,
    set_provider_credentials_calls: AtomicUsize,
    delete_provider_credentials_calls: AtomicUsize,
    test_provider_connection_calls: AtomicUsize,
    refresh_models_cache_calls: AtomicUsize,
    /// RPC-054: capture of the last (provider_id, input) tuple passed to
    /// `set_provider_credentials`.
    last_set_provider_credentials: Mutex<Option<(String, ProviderCredentialInput)>>,
    /// RPC-054: capture of the last provider_id passed to
    /// `delete_provider_credentials`.
    last_delete_provider_credentials: Mutex<Option<String>>,
    /// RPC-054: capture of the last provider_id passed to
    /// `test_provider_connection`.
    last_test_provider_connection: Mutex<Option<String>>,
    /// RPC-054: capture of the last provider_id passed to
    /// `refresh_models_cache`.
    last_refresh_models_cache: Mutex<Option<String>>,
    /// RPC-054: per-provider scripted result for
    /// `test_provider_connection`. Defaults to `Ok(success: true, latency 0)`.
    test_connection_results: Mutex<HashMap<String, TestConnectionResult>>,
    /// RPC-054: per-provider scripted model list returned by
    /// `refresh_models_cache`.
    refresh_models_results: Mutex<HashMap<String, Vec<ModelEntry>>>,
    /// RPC-054: when `Some`, `set_provider_credentials` returns
    /// `Err(anyhow!(message))` so the silent-error scenario can exercise
    /// the tracing::warn! branch.
    set_provider_credentials_error: Mutex<Option<String>>,
    /// RPC-054: when `Some`, `list_provider_credentials` returns
    /// `Err(anyhow!(message))`.
    list_provider_credentials_error: Mutex<Option<String>>,
    /// RPC-054: when `Some`, `delete_provider_credentials` returns
    /// `Err(anyhow!(message))`.
    delete_provider_credentials_error: Mutex<Option<String>>,
    /// RPC-054: when `Some`, `test_provider_connection` returns
    /// `Err(anyhow!(message))`.
    test_provider_connection_error: Mutex<Option<String>>,
    /// RPC-054: when `Some`, `refresh_models_cache` returns
    /// `Err(anyhow!(message))`.
    refresh_models_cache_error: Mutex<Option<String>>,
    // ── PROV-109 profile write surface ───────────────────────────────
    /// PROV-109: per-call counters for the profile write surface.
    save_profile_calls: AtomicUsize,
    delete_profile_calls: AtomicUsize,
    /// PROV-109: capture of the last `(provider_id, profile_name, definition)`
    /// tuple passed to `save_profile`.
    last_save_profile: Mutex<Option<(String, String, codelet_rpc_types::ProfileDefinition)>>,
    /// PROV-109: capture of the last `(provider_id, profile_name)` pair passed
    /// to `delete_profile`.
    last_delete_profile: Mutex<Option<(String, String)>>,
    /// PROV-109: when `Some`, `save_profile` returns `Err(anyhow!(message))`.
    save_profile_error: Mutex<Option<String>>,
    /// PROV-109: when `Some`, `delete_profile` returns `Err(anyhow!(message))`.
    delete_profile_error: Mutex<Option<String>>,
    /// PROV-112: per-call counter for `oauth_clear_tokens`.
    oauth_clear_tokens_calls: AtomicUsize,
    /// PROV-112: capture of every `provider_id` passed to `oauth_clear_tokens`,
    /// in call order (so per-provider routing can be asserted).
    oauth_clear_tokens_providers: Mutex<Vec<String>>,
    /// PROV-112: when `Some`, `oauth_clear_tokens` returns `Err(anyhow!(msg))`.
    oauth_clear_tokens_error: Mutex<Option<String>>,
    /// PROV-112: per-call counter for `oauth_get_tokens`.
    oauth_get_tokens_calls: AtomicUsize,
    /// PROV-112: seeded `provider_id → has_tokens` answers for
    /// `oauth_get_tokens` (defaults to `false` when unset).
    oauth_get_tokens_results: Mutex<HashMap<String, bool>>,
    // ── PROV-113 OAuth login surface ─────────────────────────────────
    /// PROV-113: per-call counter + provider capture for `oauth_browser_login`.
    oauth_browser_login_calls: AtomicUsize,
    oauth_browser_login_providers: Mutex<Vec<String>>,
    /// PROV-113: when `Some`, `oauth_browser_login` returns `Err(anyhow!(msg))`.
    oauth_browser_login_error: Mutex<Option<String>>,
    /// PROV-113: per-call counter for `oauth_headless_start`.
    oauth_headless_start_calls: AtomicUsize,
    /// PROV-113: scripted `(authorize_url, pkce_verifier)` for
    /// `oauth_headless_start`.
    oauth_headless_start_result: Mutex<(String, String)>,
    /// PROV-113: per-call counter + `(provider, code, verifier)` capture for
    /// `oauth_headless_complete`.
    oauth_headless_complete_calls: AtomicUsize,
    oauth_headless_complete_args: Mutex<Vec<(String, String, String)>>,
    oauth_headless_complete_error: Mutex<Option<String>>,
    /// PROV-113: per-call counter + provider capture for `oauth_device_start`.
    oauth_device_start_calls: AtomicUsize,
    oauth_device_start_providers: Mutex<Vec<String>>,
    /// PROV-113: scripted `(user_code, verification_url, device_auth_id,
    /// interval)` for `oauth_device_start`.
    oauth_device_start_result: Mutex<(String, String, String, u64)>,
    oauth_device_start_error: Mutex<Option<String>>,
    /// PROV-113: per-call counter for `oauth_device_poll`.
    oauth_device_poll_calls: AtomicUsize,
    oauth_device_poll_error: Mutex<Option<String>>,
    /// PROV-114: per-call counter for `oauth_copilot_device_start`.
    oauth_copilot_device_start_calls: AtomicUsize,
    /// PROV-114: capture of every `enterprise_host` passed to
    /// `oauth_copilot_device_start`, in call order (asserts the normalized
    /// host is forwarded; `None` for GitHub.com).
    oauth_copilot_device_start_hosts: Mutex<Vec<Option<String>>>,
    /// PROV-114: scripted `(user_code, verification_url, device_auth_id,
    /// interval)` returned by `oauth_copilot_device_start`.
    oauth_copilot_device_start_result: Mutex<(String, String, String, u64)>,
    /// PROV-114: when `Some`, `oauth_copilot_device_start` returns
    /// `Err(anyhow!(msg))`.
    oauth_copilot_device_start_error: Mutex<Option<String>>,
    // ── RPC-055 debug-capture surface ────────────────────────────────
    /// RPC-055: per-call counter for `toggle_debug`.
    toggle_debug_calls: AtomicUsize,
    /// RPC-055: capture of the last `(SessionId, debug_dir)` pair passed
    /// to `toggle_debug`.
    last_toggle_debug: Mutex<Option<(SessionId, String)>>,
    /// RPC-055: scripted result returned by `toggle_debug`. Default is
    /// `Ok(String::new())` so scenarios that don't care about the body
    /// still get a well-formed Ok reply.
    toggle_debug_result: Mutex<Result<String, String>>,
    /// RPC-055: per-call counter for `set_debug_directory`.
    set_debug_directory_calls: AtomicUsize,
    /// RPC-055: capture of the last path passed to `set_debug_directory`.
    last_set_debug_directory: Mutex<Option<String>>,
    /// RPC-055: when `Some`, `set_debug_directory` returns
    /// `Err(anyhow!(message))`.
    set_debug_directory_error: Mutex<Option<String>>,
    // ── RPC-430 debug hydration / propagation tracking ────────────────
    /// RPC-430: per-call counter for `get_debug_enabled`.
    get_debug_enabled_calls: AtomicUsize,
    /// RPC-430: scripted result returned by `get_debug_enabled`.
    get_debug_enabled_result: Mutex<Result<bool, String>>,
    /// RPC-430: per-call counter for `set_debug_enabled`.
    set_debug_enabled_calls: AtomicUsize,
    /// RPC-430: capture of the last `(SessionId, bool)` passed to `set_debug_enabled`.
    last_set_debug_enabled: Mutex<Option<(SessionId, bool)>>,
    // ── RPC-056 blocklist surface ────────────────────────────────────
    /// RPC-056: in-memory rule list returned by `blocklist_list`.
    blocklist_rules: Mutex<Vec<BlocklistRuleInfo>>,
    /// RPC-056: per-call counter for `blocklist_list`.
    blocklist_list_calls: AtomicUsize,
    /// RPC-056: when `Some`, `blocklist_list` returns
    /// `Err(anyhow!(message))`.
    blocklist_list_error: Mutex<Option<String>>,
    // ── RPC-057 /merge-worktree surface ──────────────────────────────
    /// RPC-057: seeded `MergeOutcome` returned by `merge_session_worktree`.
    merge_outcome: Mutex<codelet_rpc_types::MergeOutcome>,
    /// RPC-057: per-call counter for `merge_session_worktree`.
    merge_session_worktree_calls: AtomicUsize,
    /// RPC-057: when `Some`, `merge_session_worktree` returns
    /// `Err(anyhow!(message))`.
    merge_session_worktree_error: Mutex<Option<String>>,
    /// RPC-057: per-call counter for `discard_session_worktree`.
    discard_session_worktree_calls: AtomicUsize,
    /// RPC-057: when `Some`, `discard_session_worktree` returns
    /// `Err(anyhow!(message))`.
    discard_session_worktree_error: Mutex<Option<String>>,
    /// RPC-057: per-call counter for `prune_orphaned_worktrees`.
    prune_orphaned_worktrees_calls: AtomicUsize,
    /// RPC-057: seeded pruned session ids list.
    pruned_sessions: Mutex<Vec<String>>,
    /// RPC-057: per-call counter for `list_session_worktrees`.
    list_session_worktrees_calls: AtomicUsize,
    /// RPC-057: seeded `SessionWorktreeInfo` list.
    session_worktrees: Mutex<Vec<codelet_rpc_types::SessionWorktreeInfo>>,
    /// RPC-057: per-call counter for `inspect_session_changes`.
    inspect_session_changes_calls: AtomicUsize,
    /// RPC-057: seeded `SessionChangesSummary`.
    session_changes_summary: Mutex<codelet_rpc_types::SessionChangesSummary>,
    // ── RPC-058 /schedule surface ────────────────────────────────────
    /// RPC-058: seeded `Result<ScheduledJob, String>` returned by
    /// `schedule_add`. Defaults to `Ok(ScheduledJob::default())`.
    schedule_add_result: Mutex<std::result::Result<codelet_rpc_types::ScheduledJob, String>>,
    /// RPC-058: seeded `Result<Vec<ScheduledJob>, String>` returned by
    /// `schedule_list`.
    schedule_list_result: Mutex<std::result::Result<Vec<codelet_rpc_types::ScheduledJob>, String>>,
    /// RPC-058: seeded `Result<ScheduledJob, String>` returned by
    /// `schedule_pause`.
    schedule_pause_result: Mutex<std::result::Result<codelet_rpc_types::ScheduledJob, String>>,
    /// RPC-058: seeded `Result<ScheduledJob, String>` returned by
    /// `schedule_resume`.
    schedule_resume_result: Mutex<std::result::Result<codelet_rpc_types::ScheduledJob, String>>,
    /// RPC-058: seeded `Result<(), String>` returned by `schedule_remove`.
    schedule_remove_result: Mutex<std::result::Result<(), String>>,
    /// RPC-058: per-call counters.
    schedule_add_calls: AtomicUsize,
    schedule_list_calls: AtomicUsize,
    schedule_pause_calls: AtomicUsize,
    schedule_resume_calls: AtomicUsize,
    schedule_remove_calls: AtomicUsize,

    // ── RPC-059 /loop surface ────────────────────────────────────────
    /// RPC-059: seeded `Result<RegisteredLoop, String>` returned by
    /// `loop_add`. Defaults to `Ok(RegisteredLoop::default())`.
    loop_add_result: Mutex<std::result::Result<codelet_rpc_types::RegisteredLoop, String>>,
    /// RPC-059: seeded `Result<bool, String>` returned by `loop_cancel`.
    loop_cancel_result: Mutex<std::result::Result<bool, String>>,
    /// RPC-059: seeded `Result<Vec<RegisteredLoop>, String>` returned by
    /// `loop_list`.
    loop_list_result: Mutex<std::result::Result<Vec<codelet_rpc_types::RegisteredLoop>, String>>,
    /// RPC-059: per-call counters.
    loop_add_calls: AtomicUsize,
    loop_cancel_calls: AtomicUsize,
    loop_list_calls: AtomicUsize,
    /// RPC-060: seeded `Result<IsolatedSessionInfo, String>` returned by
    /// `create_isolated_session`.
    create_isolated_session_result: Mutex<std::result::Result<IsolatedSessionInfo, String>>,
    /// RPC-060: per-call counter for `create_isolated_session`.
    create_isolated_session_calls: AtomicUsize,
    /// RPC-061: scripted per-session supervisors list returned by
    /// `get_supervisors`. Indexed by SessionId.
    supervisors_results: Mutex<HashMap<SessionId, Vec<SessionId>>>,
    /// RPC-061: scripted `Result<(), String>` returned by `add_supervisor`.
    /// Default `Ok(())`.
    add_supervisor_result: Mutex<std::result::Result<(), String>>,
    /// RPC-061: scripted `Result<(), String>` returned by
    /// `receive_incoming_message`. Default `Ok(())`.
    receive_incoming_message_result: Mutex<std::result::Result<(), String>>,
    /// RPC-061: per-call counters for the five new methods.
    add_supervisor_calls: AtomicUsize,
    remove_supervisor_calls: AtomicUsize,
    get_supervisors_calls: AtomicUsize,
    get_subordinate_calls: AtomicUsize,
    get_subordinates_calls: AtomicUsize,
    receive_incoming_message_calls: AtomicUsize,
    /// RPC-061: capture of the last payload passed to
    /// `receive_incoming_message`.
    last_received_incoming_message: Mutex<Option<(SessionId, IncomingMessageInput)>>,
}

impl Default for MockBackend {
    fn default() -> Self {
        let (work_units_tx, _) = broadcast::channel(64);
        let (chunks_tx, _) = broadcast::channel(64);
        let (logs_tx, _) = broadcast::channel(64);
        let (status_changes_tx, _) = broadcast::channel(64);
        let (session_created_tx, _) = broadcast::channel(64);
        let (checkpoints_progress_tx, _) = broadcast::channel(64);
        Self {
            work_units: Mutex::new(Vec::new()),
            sessions: Mutex::new(Vec::new()),
            work_units_tx: Mutex::new(Some(work_units_tx)),
            chunks_tx: Mutex::new(Some(chunks_tx)),
            logs_tx: Mutex::new(Some(logs_tx)),
            status_changes_tx: Mutex::new(Some(status_changes_tx)),
            session_created_tx: Mutex::new(Some(session_created_tx)),
            checkpoints_progress_tx: Mutex::new(Some(checkpoints_progress_tx)),
            list_work_units_calls: AtomicUsize::new(0),
            create_session_calls: AtomicUsize::new(0),
            send_input_calls: AtomicUsize::new(0),
            interrupt_calls: AtomicUsize::new(0),
            checkpoint_counts_calls: AtomicUsize::new(0),
            restore_checkpoint_file_calls: AtomicUsize::new(0),
            restore_checkpoint_all_calls: AtomicUsize::new(0),
            last_restore_file: Mutex::new(None),
            delete_checkpoint_calls: AtomicUsize::new(0),
            delete_all_checkpoints_calls: AtomicUsize::new(0),
            last_delete_checkpoint: Mutex::new(None),
            move_work_unit_up_calls: AtomicUsize::new(0),
            move_work_unit_down_calls: AtomicUsize::new(0),
            last_move_work_unit_up_id: Mutex::new(None),
            last_move_work_unit_down_id: Mutex::new(None),
            scripted_session: Mutex::new(None),
            last_send_input: Mutex::new(None),
            last_interrupt: Mutex::new(None),
            destroy_session_calls: AtomicUsize::new(0),
            last_destroyed_session: Mutex::new(None),
            checkpoint_counts: Mutex::new(CheckpointCounts::default()),
            model_info: Mutex::new(ModelInfo::default()),
            thinking_level: Mutex::new(ThinkingLevel::Off),
            workspace_info: Mutex::new(WorkspaceInfo::default()),
            workspace_info_error: Mutex::new(None),
            file_search_results: Mutex::new(Vec::new()),
            delete_session_calls: AtomicUsize::new(0),
            last_deleted_session: Mutex::new(None),
            last_list_sessions_project: Mutex::new(None),
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
            set_default_model_calls: AtomicUsize::new(0),
            last_set_default_model: Mutex::new(None),
            last_set_thinking_level: Mutex::new(None),
            last_get_session_role: Mutex::new(None),
            last_set_session_role: Mutex::new(None),
            session_roles: Mutex::new(Vec::new()),
            persistence_add_history_calls: AtomicUsize::new(0),
            last_persistence_add_history: Mutex::new(None),
            send_fspec_result_calls: AtomicUsize::new(0),
            last_fspec_result: Mutex::new(None),
            clear_history_calls: AtomicUsize::new(0),
            last_clear_history_session: Mutex::new(None),
            clear_history_error: Mutex::new(None),
            compact_session_calls: AtomicUsize::new(0),
            last_compact_session: Mutex::new(None),
            compact_session_result: Mutex::new(Ok(CompactionResult {
                original_tokens: 0,
                compacted_tokens: 0,
                // RPC-420: compression_ratio is the PERCENT of tokens
                // removed [0,100]; 0.0 is the "nothing removed" sentinel.
                compression_ratio: 0.0,
                turns_summarized: 0,
                turns_kept: 0,
            })),
            resume_session_calls: AtomicUsize::new(0),
            last_resume_session: Mutex::new(None),
            resume_session_error: Mutex::new(None),
            buffered_output: Mutex::new(Vec::new()),
            get_buffered_output_calls: AtomicUsize::new(0),
            last_get_buffered_output: Mutex::new(None),
            set_work_unit_context_calls: AtomicUsize::new(0),
            get_work_unit_context_calls: AtomicUsize::new(0),
            last_set_work_unit_context: Mutex::new(None),
            work_unit_contexts: Mutex::new(HashMap::new()),
            set_work_unit_context_error: Mutex::new(None),
            scripted_history: Mutex::new(HashMap::new()),
            set_pending_input_calls: AtomicUsize::new(0),
            get_pending_input_calls: AtomicUsize::new(0),
            pending_input_writes: Mutex::new(Vec::new()),
            pending_input_store: Mutex::new(HashMap::new()),
            get_pending_input_error: Mutex::new(None),
            set_pending_input_error: Mutex::new(None),
            pause_state_store: Mutex::new(HashMap::new()),
            hitl_request_store: Mutex::new(HashMap::new()),
            get_pause_state_calls: AtomicUsize::new(0),
            get_hitl_request_calls: AtomicUsize::new(0),
            pause_resume_calls: AtomicUsize::new(0),
            pause_confirm_calls: AtomicUsize::new(0),
            pause_triple_calls: AtomicUsize::new(0),
            send_hitl_response_calls: AtomicUsize::new(0),
            pause_confirm_calls_log: Mutex::new(Vec::new()),
            pause_triple_calls_log: Mutex::new(Vec::new()),
            pause_resume_calls_log: Mutex::new(Vec::new()),
            send_hitl_response_calls_log: Mutex::new(Vec::new()),
            get_pause_state_error: Mutex::new(None),
            get_hitl_request_error: Mutex::new(None),
            pause_resume_error: Mutex::new(None),
            pause_confirm_error: Mutex::new(None),
            pause_triple_error: Mutex::new(None),
            send_hitl_response_error: Mutex::new(None),
            // ── RPC-054 ──────────────────────────────────────────────
            provider_credentials: Mutex::new(Vec::new()),
            provider_credentials_after_save: Mutex::new(None),
            provider_credentials_after_delete: Mutex::new(None),
            provider_credentials_after_refresh: Mutex::new(None),
            next_list_override: Mutex::new(None),
            list_provider_credentials_calls: AtomicUsize::new(0),
            get_provider_credential_calls: AtomicUsize::new(0),
            set_provider_credentials_calls: AtomicUsize::new(0),
            delete_provider_credentials_calls: AtomicUsize::new(0),
            test_provider_connection_calls: AtomicUsize::new(0),
            refresh_models_cache_calls: AtomicUsize::new(0),
            last_set_provider_credentials: Mutex::new(None),
            last_delete_provider_credentials: Mutex::new(None),
            last_test_provider_connection: Mutex::new(None),
            last_refresh_models_cache: Mutex::new(None),
            test_connection_results: Mutex::new(HashMap::new()),
            refresh_models_results: Mutex::new(HashMap::new()),
            set_provider_credentials_error: Mutex::new(None),
            list_provider_credentials_error: Mutex::new(None),
            delete_provider_credentials_error: Mutex::new(None),
            test_provider_connection_error: Mutex::new(None),
            refresh_models_cache_error: Mutex::new(None),
            // ── PROV-109 ─────────────────────────────────────────────
            save_profile_calls: AtomicUsize::new(0),
            delete_profile_calls: AtomicUsize::new(0),
            last_save_profile: Mutex::new(None),
            last_delete_profile: Mutex::new(None),
            save_profile_error: Mutex::new(None),
            delete_profile_error: Mutex::new(None),
            oauth_clear_tokens_calls: AtomicUsize::new(0),
            oauth_clear_tokens_providers: Mutex::new(Vec::new()),
            oauth_clear_tokens_error: Mutex::new(None),
            oauth_get_tokens_calls: AtomicUsize::new(0),
            oauth_get_tokens_results: Mutex::new(HashMap::new()),
            // ── PROV-113 ─────────────────────────────────────────────
            oauth_browser_login_calls: AtomicUsize::new(0),
            oauth_browser_login_providers: Mutex::new(Vec::new()),
            oauth_browser_login_error: Mutex::new(None),
            oauth_headless_start_calls: AtomicUsize::new(0),
            oauth_headless_start_result: Mutex::new((
                "https://claude.ai/oauth/authorize?code=1".to_string(),
                "v".repeat(43),
            )),
            oauth_headless_complete_calls: AtomicUsize::new(0),
            oauth_headless_complete_args: Mutex::new(Vec::new()),
            oauth_headless_complete_error: Mutex::new(None),
            oauth_device_start_calls: AtomicUsize::new(0),
            oauth_device_start_providers: Mutex::new(Vec::new()),
            oauth_device_start_result: Mutex::new((
                "ABCD-1234".to_string(),
                "https://verify.example/device".to_string(),
                "device-auth-1".to_string(),
                5,
            )),
            oauth_device_start_error: Mutex::new(None),
            oauth_device_poll_calls: AtomicUsize::new(0),
            oauth_device_poll_error: Mutex::new(None),
            oauth_copilot_device_start_calls: AtomicUsize::new(0),
            oauth_copilot_device_start_hosts: Mutex::new(Vec::new()),
            oauth_copilot_device_start_result: Mutex::new((
                "COPILOT-CODE".to_string(),
                "https://github.com/login/device".to_string(),
                "copilot-device-1".to_string(),
                1,
            )),
            oauth_copilot_device_start_error: Mutex::new(None),
            // ── RPC-055 ──────────────────────────────────────────────
            toggle_debug_calls: AtomicUsize::new(0),
            last_toggle_debug: Mutex::new(None),
            toggle_debug_result: Mutex::new(Ok(String::new())),
            set_debug_directory_calls: AtomicUsize::new(0),
            last_set_debug_directory: Mutex::new(None),
            set_debug_directory_error: Mutex::new(None),
            // ── RPC-430 ────────────────────────────────────────────────
            get_debug_enabled_calls: AtomicUsize::new(0),
            get_debug_enabled_result: Mutex::new(Ok(false)),
            set_debug_enabled_calls: AtomicUsize::new(0),
            last_set_debug_enabled: Mutex::new(None),
            // ── RPC-056 ──────────────────────────────────────────────
            blocklist_rules: Mutex::new(Vec::new()),
            blocklist_list_calls: AtomicUsize::new(0),
            blocklist_list_error: Mutex::new(None),
            merge_outcome: Mutex::new(codelet_rpc_types::MergeOutcome {
                status: codelet_rpc_types::MergeStatus::NoChanges,
                conflicts: Vec::new(),
                merge_commit: None,
            }),
            merge_session_worktree_calls: AtomicUsize::new(0),
            merge_session_worktree_error: Mutex::new(None),
            discard_session_worktree_calls: AtomicUsize::new(0),
            discard_session_worktree_error: Mutex::new(None),
            prune_orphaned_worktrees_calls: AtomicUsize::new(0),
            pruned_sessions: Mutex::new(Vec::new()),
            list_session_worktrees_calls: AtomicUsize::new(0),
            session_worktrees: Mutex::new(Vec::new()),
            inspect_session_changes_calls: AtomicUsize::new(0),
            session_changes_summary: Mutex::new(codelet_rpc_types::SessionChangesSummary {
                files_changed: 0,
                insertions: 0,
                deletions: 0,
                commits: Vec::new(),
            }),
            schedule_add_result: Mutex::new(Ok(codelet_rpc_types::ScheduledJob::default())),
            schedule_list_result: Mutex::new(Ok(Vec::new())),
            schedule_pause_result: Mutex::new(Ok(codelet_rpc_types::ScheduledJob::default())),
            schedule_resume_result: Mutex::new(Ok(codelet_rpc_types::ScheduledJob::default())),
            schedule_remove_result: Mutex::new(Ok(())),
            schedule_add_calls: AtomicUsize::new(0),
            schedule_list_calls: AtomicUsize::new(0),
            schedule_pause_calls: AtomicUsize::new(0),
            schedule_resume_calls: AtomicUsize::new(0),
            schedule_remove_calls: AtomicUsize::new(0),
            loop_add_result: Mutex::new(Ok(codelet_rpc_types::RegisteredLoop::default())),
            loop_cancel_result: Mutex::new(Ok(true)),
            loop_list_result: Mutex::new(Ok(Vec::new())),
            loop_add_calls: AtomicUsize::new(0),
            loop_cancel_calls: AtomicUsize::new(0),
            loop_list_calls: AtomicUsize::new(0),
            create_isolated_session_result: Mutex::new(Ok(IsolatedSessionInfo {
                session_id: SessionId::new("mock-isolated"),
                worktree_path: String::new(),
                base_commit: String::new(),
            })),
            create_isolated_session_calls: AtomicUsize::new(0),
            supervisors_results: Mutex::new(HashMap::new()),
            add_supervisor_result: Mutex::new(Ok(())),
            receive_incoming_message_result: Mutex::new(Ok(())),
            add_supervisor_calls: AtomicUsize::new(0),
            remove_supervisor_calls: AtomicUsize::new(0),
            get_supervisors_calls: AtomicUsize::new(0),
            get_subordinate_calls: AtomicUsize::new(0),
            get_subordinates_calls: AtomicUsize::new(0),
            receive_incoming_message_calls: AtomicUsize::new(0),
            last_received_incoming_message: Mutex::new(None),
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
        if let Some(tx) = self
            .work_units_tx
            .lock()
            .expect("MockBackend mutex")
            .as_ref()
        {
            let _ = tx.send(units);
        }
    }

    /// Script the next `create_session` call to return this SessionId.
    pub fn script_create_session(&self, id: SessionId) {
        *self.scripted_session.lock().expect("MockBackend mutex") = Some(id);
    }

    /// RPC-427: Return the last project_path passed to `list_sessions()`.
    pub fn list_sessions_project(&self) -> Option<String> {
        self.last_list_sessions_project
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// Push a chunk onto the chunks broadcast (RPC-009 test helper).
    pub fn push_chunk(&self, id: SessionId, chunk: StreamChunk) {
        if let Some(tx) = self.chunks_tx.lock().expect("MockBackend mutex").as_ref() {
            let _ = tx.send((id, chunk));
        }
    }

    /// RPC-045: drop the chunks_tx Sender to simulate a SessionManager
    /// shutdown. Subsequent `chunks_rx().recv()` calls observe
    /// `RecvError::Closed` and the subscriber loop exits cleanly.
    pub fn close_chunks_tx(&self) {
        *self.chunks_tx.lock().expect("MockBackend mutex") = None;
    }

    /// RPC-045: push a (SessionId, SessionStatus) broadcast frame so
    /// status-subscriber tests can drive scripted transitions without
    /// touching a real SessionManager.
    pub fn push_status_change(&self, id: SessionId, status: SessionStatus) {
        if let Some(tx) = self
            .status_changes_tx
            .lock()
            .expect("MockBackend mutex")
            .as_ref()
        {
            let _ = tx.send((id, status));
        }
    }

    /// RPC-385: push a session-created broadcast frame so the new
    /// session-created subscriber task can be exercised (idempotent tab
    /// append + lag recovery) without a real SessionManager spawn.
    pub fn push_session_created(&self, id: SessionId) {
        let info = SessionInfo {
            id: id.value,
            name: String::new(),
            status: "idle".to_string(),
            project: String::new(),
            message_count: 0,
            provider_id: None,
            model_id: None,
            is_isolated: false,
            worktree_path: None,
            role: None,
            updated_at_ms: None,
        };
        if let Some(tx) = self
            .session_created_tx
            .lock()
            .expect("MockBackend mutex")
            .as_ref()
        {
            let _ = tx.send(info);
        }
    }

    /// TUI-109: push a checkpoint-enumeration progress frame so the
    /// checkpoints-progress subscriber test can drive synthetic frames
    /// without a real RPC server.
    pub fn push_checkpoints_progress(
        &self,
        progress: codelet_rpc_types::CheckpointsProgress,
    ) {
        if let Some(tx) = self
            .checkpoints_progress_tx
            .lock()
            .expect("MockBackend mutex")
            .as_ref()
        {
            let _ = tx.send(progress);
        }
    }

    /// RPC-415: drop ALL five broadcast Senders to simulate the transport
    /// supervisor dropping the old RPC client on a WS disconnect. Every
    /// live subscriber `Receiver` then observes `RecvError::Closed` on its
    /// next `recv().await`, so all five App subscriber loops exit.
    pub fn disconnect_all(&self) {
        *self.work_units_tx.lock().expect("MockBackend mutex") = None;
        *self.chunks_tx.lock().expect("MockBackend mutex") = None;
        *self.logs_tx.lock().expect("MockBackend mutex") = None;
        *self.status_changes_tx.lock().expect("MockBackend mutex") = None;
        *self.session_created_tx.lock().expect("MockBackend mutex") = None;
        *self
            .checkpoints_progress_tx
            .lock()
            .expect("MockBackend mutex") = None;
    }

    /// RPC-415: install a FRESH Sender for every broadcast stream,
    /// modelling the transport supervisor swapping in a brand-new RPC
    /// client after a successful reconnect. The new Senders are distinct
    /// from any dropped by `disconnect_all()`, so a subscriber that
    /// re-subscribes now is bound to the NEW client's receivers. Any
    /// receiver still holding an OLD Sender's `Receiver` will never see
    /// events pushed after this call — which is exactly how we prove the
    /// respawn rebinds to the current client.
    pub fn reconnect_all(&self) {
        let (work_units_tx, _) = broadcast::channel(64);
        let (chunks_tx, _) = broadcast::channel(64);
        let (logs_tx, _) = broadcast::channel(64);
        let (status_changes_tx, _) = broadcast::channel(64);
        let (session_created_tx, _) = broadcast::channel(64);
        let (checkpoints_progress_tx, _) = broadcast::channel(64);
        *self.work_units_tx.lock().expect("MockBackend mutex") = Some(work_units_tx);
        *self.chunks_tx.lock().expect("MockBackend mutex") = Some(chunks_tx);
        *self.logs_tx.lock().expect("MockBackend mutex") = Some(logs_tx);
        *self.status_changes_tx.lock().expect("MockBackend mutex") = Some(status_changes_tx);
        *self.session_created_tx.lock().expect("MockBackend mutex") = Some(session_created_tx);
        *self
            .checkpoints_progress_tx
            .lock()
            .expect("MockBackend mutex") = Some(checkpoints_progress_tx);
    }

    /// RPC-045: per-call counter for `send_fspec_result`.
    pub fn send_fspec_result_calls(&self) -> usize {
        self.send_fspec_result_calls.load(Ordering::SeqCst)
    }

    /// RPC-045: the most recent `FspecResult` captured by
    /// `send_fspec_result`. `None` until the runner round-trips.
    pub fn last_fspec_result(&self) -> Option<FspecResult> {
        self.last_fspec_result
            .lock()
            .expect("MockBackend mutex")
            .as_ref()
            .map(|(_, r)| r.clone())
    }

    /// RPC-045: the matching `(SessionId, FspecResult)` tuple captured
    /// by `send_fspec_result`. Useful when a test seeds multiple
    /// sessions and needs to assert which one the result was routed to.
    pub fn last_fspec_result_with_session(&self) -> Option<(SessionId, FspecResult)> {
        self.last_fspec_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
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
        self.last_send_input
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }
    pub fn last_interrupt(&self) -> Option<SessionId> {
        self.last_interrupt
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-098: count of `destroy_session` calls observed by this mock.
    pub fn destroy_session_calls(&self) -> usize {
        self.destroy_session_calls.load(Ordering::SeqCst)
    }

    /// RPC-098: most recently destroyed SessionId.
    pub fn last_destroyed_session(&self) -> Option<SessionId> {
        self.last_destroyed_session
            .lock()
            .expect("MockBackend mutex")
            .clone()
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

    /// RPC-365: how many times `restore_checkpoint_file()` has been
    /// awaited.
    pub fn restore_checkpoint_file_calls(&self) -> usize {
        self.restore_checkpoint_file_calls.load(Ordering::SeqCst)
    }

    /// RPC-365: the `(work_unit_id, name, path)` of the most recent
    /// `restore_checkpoint_file()` call, if any.
    pub fn last_restore_file(&self) -> Option<(String, String, String)> {
        self.last_restore_file
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-365: how many times `restore_checkpoint_all()` has been
    /// awaited.
    pub fn restore_checkpoint_all_calls(&self) -> usize {
        self.restore_checkpoint_all_calls.load(Ordering::SeqCst)
    }

    /// RPC-366: how many times `delete_checkpoint()` has been awaited.
    pub fn delete_checkpoint_calls(&self) -> usize {
        self.delete_checkpoint_calls.load(Ordering::SeqCst)
    }

    /// RPC-366: the `(work_unit_id, name)` of the most recent
    /// `delete_checkpoint()` call, if any.
    pub fn last_delete_checkpoint(&self) -> Option<(String, String)> {
        self.last_delete_checkpoint
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-366: how many times `delete_all_checkpoints()` has been
    /// awaited.
    pub fn delete_all_checkpoints_calls(&self) -> usize {
        self.delete_all_checkpoints_calls.load(Ordering::SeqCst)
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
    pub fn set_history_search_results(&self, results: Vec<codelet_rpc_types::HistoryMatch>) {
        *self
            .history_search_results
            .lock()
            .expect("MockBackend mutex") = results;
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

    /// PROV-118: how many times `set_default_model` was awaited.
    pub fn set_default_model_calls(&self) -> usize {
        self.set_default_model_calls.load(Ordering::SeqCst)
    }

    /// PROV-118: the last model string passed to `set_default_model`.
    pub fn last_set_default_model(&self) -> Option<String> {
        self.last_set_default_model
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

    /// RPC-046: how many times `clear_history` was awaited.
    pub fn clear_history_calls(&self) -> usize {
        self.clear_history_calls.load(Ordering::SeqCst)
    }

    /// RPC-046: the last SessionId passed to `clear_history`.
    pub fn last_clear_history_session(&self) -> Option<SessionId> {
        self.last_clear_history_session
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-046: force the next `clear_history` call to fail with the
    /// supplied message — exercises the error-notice branch.
    pub fn set_clear_history_error(&self, message: String) {
        *self.clear_history_error.lock().expect("MockBackend mutex") = Some(message);
    }

    /// RPC-047: how many times `compact_session` was awaited.
    pub fn compact_session_calls(&self) -> usize {
        self.compact_session_calls.load(Ordering::SeqCst)
    }

    /// RPC-047: the last SessionId passed to `compact_session`.
    pub fn last_compact_session(&self) -> Option<SessionId> {
        self.last_compact_session
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-047: script the next `compact_session` call to return
    /// `Ok(result)`.
    pub fn set_compact_session_result_ok(&self, result: CompactionResult) {
        *self
            .compact_session_result
            .lock()
            .expect("MockBackend mutex") = Ok(result);
    }

    /// RPC-047: script the next `compact_session` call to return
    /// `Err(message)` — exercises the error-notice branch.
    pub fn set_compact_session_result_err(&self, message: String) {
        *self
            .compact_session_result
            .lock()
            .expect("MockBackend mutex") = Err(message);
    }

    /// RPC-049: how many times `resume_session` was awaited.
    pub fn resume_session_calls(&self) -> usize {
        self.resume_session_calls.load(Ordering::SeqCst)
    }

    /// RPC-049: the last SessionId passed to `resume_session`.
    pub fn last_resume_session(&self) -> Option<SessionId> {
        self.last_resume_session
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-049: force the next `resume_session` call to fail with the
    /// supplied message — exercises the error-notice branch.
    pub fn set_resume_session_error(&self, message: String) {
        *self.resume_session_error.lock().expect("MockBackend mutex") = Some(message);
    }

    /// RPC-049: script the replay-buffer returned by
    /// `get_buffered_output` (used by `SessionResumeComplete`).
    pub fn set_buffered_output(&self, chunks: Vec<StreamChunk>) {
        *self.buffered_output.lock().expect("MockBackend mutex") = chunks;
    }

    /// RPC-049: how many times `get_buffered_output` was awaited.
    pub fn get_buffered_output_calls(&self) -> usize {
        self.get_buffered_output_calls.load(Ordering::SeqCst)
    }

    /// RPC-049: the last `(SessionId, limit)` pair passed to
    /// `get_buffered_output`.
    pub fn last_get_buffered_output(&self) -> Option<(SessionId, u32)> {
        self.last_get_buffered_output
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-050: how many times `set_work_unit_context` was awaited.
    pub fn set_work_unit_context_calls(&self) -> usize {
        self.set_work_unit_context_calls.load(Ordering::SeqCst)
    }

    /// RPC-050: how many times `get_work_unit_context` was awaited.
    pub fn get_work_unit_context_calls(&self) -> usize {
        self.get_work_unit_context_calls.load(Ordering::SeqCst)
    }

    /// RPC-050: the last `(SessionId, Option<WorkUnitContext>)` passed to
    /// `set_work_unit_context`. `None` until the first call.
    pub fn last_set_work_unit_context(&self) -> Option<(SessionId, Option<WorkUnitContext>)> {
        self.last_set_work_unit_context
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-050: force the next `set_work_unit_context` call to fail
    /// with the supplied message — exercises the error-notice branch.
    pub fn set_work_unit_context_error(&self, message: String) {
        *self
            .set_work_unit_context_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// RPC-051: seed the snapshot the App's Shift+↑ first-press fetch
    /// task receives via `persistence_get_history`. Defaults to an
    /// empty Vec for sessions that have not been scripted.
    pub fn script_history(&self, session: SessionId, entries: Vec<String>) {
        self.scripted_history
            .lock()
            .expect("MockBackend mutex")
            .insert(session, entries);
    }

    // ── RPC-052 helpers ──────────────────────────────────────────────────

    /// RPC-052: how many times `set_pending_input` was awaited.
    pub fn set_pending_input_calls(&self) -> usize {
        self.set_pending_input_calls.load(Ordering::SeqCst)
    }

    /// RPC-052: how many times `get_pending_input` was awaited.
    pub fn get_pending_input_calls(&self) -> usize {
        self.get_pending_input_calls.load(Ordering::SeqCst)
    }

    /// RPC-052: every `(SessionId, Option<String>)` write captured by
    /// `set_pending_input`, in call order. Lets debounce-coalescing
    /// tests assert that only ONE write landed AND its final value.
    pub fn pending_input_writes(&self) -> Vec<(SessionId, Option<String>)> {
        self.pending_input_writes
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-052: convenience accessor for the most recent
    /// `set_pending_input` argument tuple.
    pub fn last_set_pending_input(&self) -> Option<(SessionId, Option<String>)> {
        self.pending_input_writes
            .lock()
            .expect("MockBackend mutex")
            .last()
            .cloned()
    }

    /// RPC-052: seed the value `get_pending_input(session)` returns.
    /// Pass `Some(text)` to simulate a restored draft; pass `None` to
    /// simulate "no draft" (the default for un-scripted sessions is
    /// also `None`).
    pub fn script_pending_input(&self, session: SessionId, value: Option<String>) {
        self.pending_input_store
            .lock()
            .expect("MockBackend mutex")
            .insert(session, value);
    }

    /// RPC-052: force the next `get_pending_input` call to fail with
    /// the supplied message.
    pub fn set_get_pending_input_error(&self, message: String) {
        *self
            .get_pending_input_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// RPC-052: force the next `set_pending_input` call to fail with
    /// the supplied message.
    pub fn set_set_pending_input_error(&self, message: String) {
        *self
            .set_pending_input_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    // ── RPC-053 helpers ──────────────────────────────────────────────────

    /// RPC-053: seed the value `get_pause_state(session)` returns.
    /// Pass `Some(state)` to simulate an active pause; pass `None` to
    /// simulate "no pause" (the default for un-scripted sessions is
    /// also `None`).
    pub fn script_pause_state(&self, session: SessionId, value: Option<PauseState>) {
        self.pause_state_store
            .lock()
            .expect("MockBackend mutex")
            .insert(session, value);
    }

    /// RPC-053: seed the value `get_hitl_request(session)` returns.
    pub fn script_hitl_request(&self, session: SessionId, value: Option<HitlRequest>) {
        self.hitl_request_store
            .lock()
            .expect("MockBackend mutex")
            .insert(session, value);
    }

    /// RPC-053: per-call counter for `get_pause_state`.
    pub fn get_pause_state_calls(&self) -> usize {
        self.get_pause_state_calls.load(Ordering::SeqCst)
    }

    /// RPC-053: per-call counter for `get_hitl_request`.
    pub fn get_hitl_request_calls(&self) -> usize {
        self.get_hitl_request_calls.load(Ordering::SeqCst)
    }

    /// RPC-053: per-call counter for `pause_resume`.
    pub fn pause_resume_calls(&self) -> usize {
        self.pause_resume_calls.load(Ordering::SeqCst)
    }

    /// RPC-053: per-call counter for `pause_confirm`.
    pub fn pause_confirm_calls(&self) -> usize {
        self.pause_confirm_calls.load(Ordering::SeqCst)
    }

    /// RPC-053: per-call counter for `pause_triple`.
    pub fn pause_triple_calls(&self) -> usize {
        self.pause_triple_calls.load(Ordering::SeqCst)
    }

    /// RPC-053: per-call counter for `send_hitl_response`.
    pub fn send_hitl_response_calls(&self) -> usize {
        self.send_hitl_response_calls.load(Ordering::SeqCst)
    }

    /// RPC-053: full call log for `pause_confirm` (every (session, accept)).
    pub fn pause_confirm_log(&self) -> Vec<(SessionId, bool)> {
        self.pause_confirm_calls_log
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-053: full call log for `pause_triple`.
    pub fn pause_triple_log(&self) -> Vec<(SessionId, ApprovalChoice)> {
        self.pause_triple_calls_log
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-053: full call log for `pause_resume`.
    pub fn pause_resume_log(&self) -> Vec<SessionId> {
        self.pause_resume_calls_log
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-053: full call log for `send_hitl_response`.
    pub fn send_hitl_response_log(&self) -> Vec<(SessionId, HitlResponse)> {
        self.send_hitl_response_calls_log
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-053: force the next `get_pause_state` call to fail.
    pub fn set_get_pause_state_error(&self, message: String) {
        *self
            .get_pause_state_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// RPC-053: force the next `get_hitl_request` call to fail.
    pub fn set_get_hitl_request_error(&self, message: String) {
        *self
            .get_hitl_request_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// RPC-053: force the next `pause_resume` call to fail.
    pub fn set_pause_resume_error(&self, message: String) {
        *self.pause_resume_error.lock().expect("MockBackend mutex") = Some(message);
    }

    /// RPC-053: force the next `pause_confirm` call to fail.
    pub fn set_pause_confirm_error(&self, message: String) {
        *self.pause_confirm_error.lock().expect("MockBackend mutex") = Some(message);
    }

    /// RPC-053: force the next `pause_triple` call to fail.
    pub fn set_pause_triple_error(&self, message: String) {
        *self.pause_triple_error.lock().expect("MockBackend mutex") = Some(message);
    }

    /// RPC-053: force the next `send_hitl_response` call to fail.
    pub fn set_send_hitl_response_error(&self, message: String) {
        *self
            .send_hitl_response_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    // ── RPC-054 seed / mutator helpers ───────────────────────────────────

    /// RPC-054: replace the in-memory provider credential list returned
    /// by `list_provider_credentials`.
    pub fn seed_provider_credentials(&self, list: Vec<ProviderCredentialInfo>) {
        *self.provider_credentials.lock().expect("MockBackend mutex") = list;
    }

    /// RPC-054: script the list returned by the very NEXT
    /// `list_provider_credentials` call after a successful save
    /// (one-shot — consumed by the call).
    pub fn set_post_save_provider_credentials(&self, list: Vec<ProviderCredentialInfo>) {
        *self
            .provider_credentials_after_save
            .lock()
            .expect("MockBackend mutex") = Some(list);
    }

    /// RPC-054: script the list returned by the very NEXT
    /// `list_provider_credentials` call after a successful delete.
    pub fn set_post_delete_provider_credentials(&self, list: Vec<ProviderCredentialInfo>) {
        *self
            .provider_credentials_after_delete
            .lock()
            .expect("MockBackend mutex") = Some(list);
    }

    /// RPC-054: script the list returned by the very NEXT
    /// `list_provider_credentials` call after a successful refresh.
    pub fn set_post_refresh_provider_credentials(&self, list: Vec<ProviderCredentialInfo>) {
        *self
            .provider_credentials_after_refresh
            .lock()
            .expect("MockBackend mutex") = Some(list);
    }

    /// RPC-054: script the result returned by
    /// `test_provider_connection` for the given provider_id.
    pub fn set_test_connection_result(
        &self,
        provider_id: impl Into<String>,
        result: TestConnectionResult,
    ) {
        self.test_connection_results
            .lock()
            .expect("MockBackend mutex")
            .insert(provider_id.into(), result);
    }

    /// RPC-054: script the model list returned by
    /// `refresh_models_cache` for the given provider_id.
    pub fn set_refresh_models_result(
        &self,
        provider_id: impl Into<String>,
        models: Vec<ModelEntry>,
    ) {
        self.refresh_models_results
            .lock()
            .expect("MockBackend mutex")
            .insert(provider_id.into(), models);
    }

    /// RPC-054: force the next `set_provider_credentials` call to fail.
    pub fn set_set_provider_credentials_error(&self, message: String) {
        *self
            .set_provider_credentials_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// RPC-054: force the next `list_provider_credentials` call to fail.
    pub fn set_list_provider_credentials_error(&self, message: String) {
        *self
            .list_provider_credentials_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// PROV-109: force the next `save_profile` call to fail.
    pub fn set_save_profile_error(&self, message: String) {
        *self.save_profile_error.lock().expect("MockBackend mutex") = Some(message);
    }

    /// PROV-109: force the next `delete_profile` call to fail.
    pub fn set_delete_profile_error(&self, message: String) {
        *self.delete_profile_error.lock().expect("MockBackend mutex") = Some(message);
    }

    /// PROV-112: force `oauth_clear_tokens` to fail with `message`.
    pub fn set_oauth_clear_tokens_error(&self, message: String) {
        *self
            .oauth_clear_tokens_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// PROV-112: per-call counter for `oauth_clear_tokens`.
    pub fn oauth_clear_tokens_calls(&self) -> usize {
        self.oauth_clear_tokens_calls.load(Ordering::SeqCst)
    }

    /// PROV-112: every `provider_id` passed to `oauth_clear_tokens`, in order.
    pub fn oauth_clear_tokens_providers(&self) -> Vec<String> {
        self.oauth_clear_tokens_providers
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// PROV-112: per-call counter for `oauth_get_tokens`.
    pub fn oauth_get_tokens_calls(&self) -> usize {
        self.oauth_get_tokens_calls.load(Ordering::SeqCst)
    }

    /// PROV-112: seed the `has_tokens` answer for a provider's
    /// `oauth_get_tokens`.
    pub fn seed_oauth_get_tokens(&self, provider_id: &str, has_tokens: bool) {
        self.oauth_get_tokens_results
            .lock()
            .expect("MockBackend mutex")
            .insert(provider_id.to_string(), has_tokens);
    }

    /// PROV-113: force `oauth_browser_login` to fail with `message`.
    pub fn set_oauth_browser_login_error(&self, message: String) {
        *self
            .oauth_browser_login_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// PROV-113: per-call counter for `oauth_browser_login`.
    pub fn oauth_browser_login_calls(&self) -> usize {
        self.oauth_browser_login_calls.load(Ordering::SeqCst)
    }

    /// PROV-113: every `provider_id` passed to `oauth_browser_login`, in order.
    pub fn oauth_browser_login_providers(&self) -> Vec<String> {
        self.oauth_browser_login_providers
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// PROV-113: script the `(authorize_url, pkce_verifier)` returned by
    /// `oauth_headless_start`.
    pub fn seed_oauth_headless_start(&self, authorize_url: &str, pkce_verifier: &str) {
        *self
            .oauth_headless_start_result
            .lock()
            .expect("MockBackend mutex") = (authorize_url.to_string(), pkce_verifier.to_string());
    }

    /// PROV-113: per-call counter for `oauth_headless_start`.
    pub fn oauth_headless_start_calls(&self) -> usize {
        self.oauth_headless_start_calls.load(Ordering::SeqCst)
    }

    /// PROV-113: per-call counter for `oauth_headless_complete`.
    pub fn oauth_headless_complete_calls(&self) -> usize {
        self.oauth_headless_complete_calls.load(Ordering::SeqCst)
    }

    /// PROV-113: every `(provider, code, verifier)` passed to
    /// `oauth_headless_complete`, in call order.
    pub fn oauth_headless_complete_args(&self) -> Vec<(String, String, String)> {
        self.oauth_headless_complete_args
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// PROV-113: script the `(user_code, verification_url, device_auth_id,
    /// interval)` returned by `oauth_device_start`.
    pub fn seed_oauth_device_start(
        &self,
        user_code: &str,
        verification_url: &str,
        device_auth_id: &str,
        interval: u64,
    ) {
        *self
            .oauth_device_start_result
            .lock()
            .expect("MockBackend mutex") = (
            user_code.to_string(),
            verification_url.to_string(),
            device_auth_id.to_string(),
            interval,
        );
    }

    /// PROV-113: per-call counter for `oauth_device_start`.
    pub fn oauth_device_start_calls(&self) -> usize {
        self.oauth_device_start_calls.load(Ordering::SeqCst)
    }

    /// PROV-113: per-call counter for `oauth_device_poll`.
    pub fn oauth_device_poll_calls(&self) -> usize {
        self.oauth_device_poll_calls.load(Ordering::SeqCst)
    }

    /// PROV-114: script the `(user_code, verification_url, device_auth_id,
    /// interval)` returned by `oauth_copilot_device_start`.
    pub fn seed_oauth_copilot_device_start(
        &self,
        user_code: &str,
        verification_url: &str,
        device_auth_id: &str,
        interval: u64,
    ) {
        *self
            .oauth_copilot_device_start_result
            .lock()
            .expect("MockBackend mutex") = (
            user_code.to_string(),
            verification_url.to_string(),
            device_auth_id.to_string(),
            interval,
        );
    }

    /// PROV-114: force `oauth_copilot_device_start` to fail with `message`.
    pub fn set_oauth_copilot_device_start_error(&self, message: String) {
        *self
            .oauth_copilot_device_start_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// PROV-114: per-call counter for `oauth_copilot_device_start`.
    pub fn oauth_copilot_device_start_calls(&self) -> usize {
        self.oauth_copilot_device_start_calls.load(Ordering::SeqCst)
    }

    /// PROV-114: every `enterprise_host` passed to
    /// `oauth_copilot_device_start`, in call order.
    pub fn oauth_copilot_device_start_hosts(&self) -> Vec<Option<String>> {
        self.oauth_copilot_device_start_hosts
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// PROV-109: per-call counter for `save_profile`.
    pub fn save_profile_calls(&self) -> usize {
        self.save_profile_calls.load(Ordering::SeqCst)
    }

    /// PROV-109: per-call counter for `delete_profile`.
    pub fn delete_profile_calls(&self) -> usize {
        self.delete_profile_calls.load(Ordering::SeqCst)
    }

    /// PROV-109: capture of the last `(provider_id, profile_name, definition)`
    /// tuple passed to `save_profile`.
    pub fn last_save_profile(
        &self,
    ) -> Option<(String, String, codelet_rpc_types::ProfileDefinition)> {
        self.last_save_profile
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// PROV-109: capture of the last `(provider_id, profile_name)` pair passed
    /// to `delete_profile`.
    pub fn last_delete_profile(&self) -> Option<(String, String)> {
        self.last_delete_profile
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-054: per-call counter for `list_provider_credentials`.
    pub fn list_provider_credentials_calls(&self) -> usize {
        self.list_provider_credentials_calls.load(Ordering::SeqCst)
    }

    /// RPC-054: per-call counter for `set_provider_credentials`.
    pub fn set_provider_credentials_calls(&self) -> usize {
        self.set_provider_credentials_calls.load(Ordering::SeqCst)
    }

    /// RPC-054: per-call counter for `delete_provider_credentials`.
    pub fn delete_provider_credentials_calls(&self) -> usize {
        self.delete_provider_credentials_calls
            .load(Ordering::SeqCst)
    }

    /// RPC-054: per-call counter for `test_provider_connection`.
    pub fn test_provider_connection_calls(&self) -> usize {
        self.test_provider_connection_calls.load(Ordering::SeqCst)
    }

    /// RPC-054: per-call counter for `refresh_models_cache`.
    pub fn refresh_models_cache_calls(&self) -> usize {
        self.refresh_models_cache_calls.load(Ordering::SeqCst)
    }

    /// RPC-054: capture of the last `(provider_id, input)` tuple passed
    /// to `set_provider_credentials`.
    pub fn last_set_provider_credentials(&self) -> Option<(String, ProviderCredentialInput)> {
        self.last_set_provider_credentials
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-054: capture of the last provider_id passed to
    /// `test_provider_connection`.
    pub fn last_test_provider_connection(&self) -> Option<String> {
        self.last_test_provider_connection
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-054: capture of the last provider_id passed to
    /// `refresh_models_cache`.
    pub fn last_refresh_models_cache(&self) -> Option<String> {
        self.last_refresh_models_cache
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-054: capture of the last provider_id passed to
    /// `delete_provider_credentials`.
    pub fn last_delete_provider_credentials(&self) -> Option<String> {
        self.last_delete_provider_credentials
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    // ── RPC-055 debug-capture surface helpers ────────────────────────

    /// RPC-055: how many times `toggle_debug` was awaited.
    pub fn toggle_debug_calls(&self) -> usize {
        self.toggle_debug_calls.load(Ordering::SeqCst)
    }

    /// RPC-055: the last `(SessionId, debug_dir)` tuple passed to
    /// `toggle_debug`.
    pub fn last_toggle_debug(&self) -> Option<(SessionId, String)> {
        self.last_toggle_debug
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    /// RPC-055: script the next `toggle_debug` call to return `Ok(path)`.
    pub fn set_toggle_debug_result_ok(&self, path: String) {
        *self.toggle_debug_result.lock().expect("MockBackend mutex") = Ok(path);
    }

    /// RPC-055: script the next `toggle_debug` call to return
    /// `Err(message)` — exercises the error-notice branch.
    pub fn set_toggle_debug_result_err(&self, message: String) {
        *self.toggle_debug_result.lock().expect("MockBackend mutex") = Err(message);
    }

    /// RPC-055: how many times `set_debug_directory` was awaited.
    pub fn set_debug_directory_calls(&self) -> usize {
        self.set_debug_directory_calls.load(Ordering::SeqCst)
    }

    /// RPC-055: the last path passed to `set_debug_directory`.
    pub fn last_set_debug_directory(&self) -> Option<String> {
        self.last_set_debug_directory
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    // ── RPC-430 debug hydration / propagation helpers ──────────────────

    /// RPC-430: how many times `get_debug_enabled` was awaited.
    pub fn get_debug_enabled_calls(&self) -> usize {
        self.get_debug_enabled_calls.load(Ordering::SeqCst)
    }

    /// RPC-430: script the next `get_debug_enabled` call to return
    /// `Ok(val)`.
    pub fn set_get_debug_enabled_result_ok(&self, val: bool) {
        *self
            .get_debug_enabled_result
            .lock()
            .expect("MockBackend mutex") = Ok(val);
    }

    /// RPC-430: how many times `set_debug_enabled` was awaited.
    pub fn set_debug_enabled_calls(&self) -> usize {
        self.set_debug_enabled_calls.load(Ordering::SeqCst)
    }

    /// RPC-430: the last `(SessionId, bool)` passed to `set_debug_enabled`.
    pub fn last_set_debug_enabled(&self) -> Option<(SessionId, bool)> {
        self.last_set_debug_enabled
            .lock()
            .expect("MockBackend mutex")
            .clone()
    }

    // ── RPC-056 blocklist surface helpers ────────────────────────────

    /// RPC-056: replace the in-memory rule list returned by
    /// `blocklist_list`.
    pub fn seed_blocklist_rules(&self, rules: Vec<BlocklistRuleInfo>) {
        *self.blocklist_rules.lock().expect("MockBackend mutex") = rules;
    }

    /// RPC-056: how many times `blocklist_list` was awaited.
    pub fn blocklist_list_calls(&self) -> usize {
        self.blocklist_list_calls.load(Ordering::SeqCst)
    }

    /// RPC-056: force the next `blocklist_list` call to fail.
    #[allow(dead_code)]
    pub fn set_blocklist_list_error(&self, message: String) {
        *self.blocklist_list_error.lock().expect("MockBackend mutex") = Some(message);
    }

    // ── RPC-057 /merge-worktree surface helpers ──────────────────────

    /// RPC-057: seed the `MergeOutcome` returned by `merge_session_worktree`.
    #[allow(dead_code)]
    pub fn seed_merge_outcome(&self, outcome: codelet_rpc_types::MergeOutcome) {
        *self.merge_outcome.lock().expect("MockBackend mutex") = outcome;
    }

    /// RPC-057: force the next `merge_session_worktree` call to fail.
    #[allow(dead_code)]
    pub fn set_merge_session_worktree_error(&self, message: String) {
        *self
            .merge_session_worktree_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// RPC-057: per-call counter for `merge_session_worktree`.
    #[allow(dead_code)]
    pub fn merge_session_worktree_calls(&self) -> usize {
        self.merge_session_worktree_calls.load(Ordering::SeqCst)
    }

    /// RPC-057: force the next `discard_session_worktree` call to fail.
    #[allow(dead_code)]
    pub fn set_discard_session_worktree_error(&self, message: String) {
        *self
            .discard_session_worktree_error
            .lock()
            .expect("MockBackend mutex") = Some(message);
    }

    /// RPC-057: per-call counter for `discard_session_worktree`.
    #[allow(dead_code)]
    pub fn discard_session_worktree_calls(&self) -> usize {
        self.discard_session_worktree_calls.load(Ordering::SeqCst)
    }

    /// RPC-057: seed the pruned session ids list.
    #[allow(dead_code)]
    pub fn seed_pruned_sessions(&self, ids: Vec<String>) {
        *self.pruned_sessions.lock().expect("MockBackend mutex") = ids;
    }

    /// RPC-057: per-call counter for `prune_orphaned_worktrees`.
    #[allow(dead_code)]
    pub fn prune_orphaned_worktrees_calls(&self) -> usize {
        self.prune_orphaned_worktrees_calls.load(Ordering::SeqCst)
    }

    /// RPC-057: seed the `SessionWorktreeInfo` rows.
    #[allow(dead_code)]
    pub fn seed_session_worktrees(&self, rows: Vec<codelet_rpc_types::SessionWorktreeInfo>) {
        *self.session_worktrees.lock().expect("MockBackend mutex") = rows;
    }

    /// RPC-057: per-call counter for `list_session_worktrees`.
    #[allow(dead_code)]
    pub fn list_session_worktrees_calls(&self) -> usize {
        self.list_session_worktrees_calls.load(Ordering::SeqCst)
    }

    /// RPC-057: seed the `SessionChangesSummary`.
    #[allow(dead_code)]
    pub fn seed_session_changes_summary(&self, summary: codelet_rpc_types::SessionChangesSummary) {
        *self
            .session_changes_summary
            .lock()
            .expect("MockBackend mutex") = summary;
    }

    /// RPC-057: per-call counter for `inspect_session_changes`.
    #[allow(dead_code)]
    pub fn inspect_session_changes_calls(&self) -> usize {
        self.inspect_session_changes_calls.load(Ordering::SeqCst)
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule seeds + counters.
    // ─────────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn seed_schedule_add_result(
        &self,
        result: std::result::Result<codelet_rpc_types::ScheduledJob, String>,
    ) {
        *self.schedule_add_result.lock().expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn seed_schedule_list_result(
        &self,
        result: std::result::Result<Vec<codelet_rpc_types::ScheduledJob>, String>,
    ) {
        *self.schedule_list_result.lock().expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn seed_schedule_pause_result(
        &self,
        result: std::result::Result<codelet_rpc_types::ScheduledJob, String>,
    ) {
        *self
            .schedule_pause_result
            .lock()
            .expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn seed_schedule_resume_result(
        &self,
        result: std::result::Result<codelet_rpc_types::ScheduledJob, String>,
    ) {
        *self
            .schedule_resume_result
            .lock()
            .expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn seed_schedule_remove_result(&self, result: std::result::Result<(), String>) {
        *self
            .schedule_remove_result
            .lock()
            .expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn schedule_add_calls(&self) -> usize {
        self.schedule_add_calls.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn schedule_list_calls(&self) -> usize {
        self.schedule_list_calls.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn schedule_pause_calls(&self) -> usize {
        self.schedule_pause_calls.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn schedule_resume_calls(&self) -> usize {
        self.schedule_resume_calls.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn schedule_remove_calls(&self) -> usize {
        self.schedule_remove_calls.load(Ordering::SeqCst)
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop seeds + counters.
    // ─────────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn seed_loop_add_result(
        &self,
        result: std::result::Result<codelet_rpc_types::RegisteredLoop, String>,
    ) {
        *self.loop_add_result.lock().expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn seed_loop_cancel_result(&self, result: std::result::Result<bool, String>) {
        *self.loop_cancel_result.lock().expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn seed_loop_list_result(
        &self,
        result: std::result::Result<Vec<codelet_rpc_types::RegisteredLoop>, String>,
    ) {
        *self.loop_list_result.lock().expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn loop_add_calls(&self) -> usize {
        self.loop_add_calls.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn loop_cancel_calls(&self) -> usize {
        self.loop_cancel_calls.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn loop_list_calls(&self) -> usize {
        self.loop_list_calls.load(Ordering::SeqCst)
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-060 — create_isolated_session seeds + counter.
    // ─────────────────────────────────────────────────────────────────

    #[allow(dead_code)]
    pub fn seed_create_isolated_session_result(
        &self,
        result: std::result::Result<IsolatedSessionInfo, String>,
    ) {
        *self
            .create_isolated_session_result
            .lock()
            .expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn create_isolated_session_calls(&self) -> usize {
        self.create_isolated_session_calls.load(Ordering::SeqCst)
    }

    // ── RPC-061 seeders / accessors ─────────────────────────────────────

    /// Seed the per-session supervisors list returned by `get_supervisors`.
    pub fn seed_supervisors_for(&self, session: SessionId, supervisors: Vec<SessionId>) {
        self.supervisors_results
            .lock()
            .expect("MockBackend mutex")
            .insert(session, supervisors);
    }

    /// Seed the `Result<(), String>` returned by `add_supervisor`.
    #[allow(dead_code)]
    pub fn seed_add_supervisor_result(&self, result: std::result::Result<(), String>) {
        *self
            .add_supervisor_result
            .lock()
            .expect("MockBackend mutex") = result;
    }

    /// Seed the `Result<(), String>` returned by `receive_incoming_message`.
    pub fn seed_receive_incoming_message_result(&self, result: std::result::Result<(), String>) {
        *self
            .receive_incoming_message_result
            .lock()
            .expect("MockBackend mutex") = result;
    }

    #[allow(dead_code)]
    pub fn add_supervisor_calls(&self) -> u64 {
        self.add_supervisor_calls.load(Ordering::SeqCst) as u64
    }
    #[allow(dead_code)]
    pub fn remove_supervisor_calls(&self) -> u64 {
        self.remove_supervisor_calls.load(Ordering::SeqCst) as u64
    }
    #[allow(dead_code)]
    pub fn get_supervisors_calls(&self) -> u64 {
        self.get_supervisors_calls.load(Ordering::SeqCst) as u64
    }
    #[allow(dead_code)]
    pub fn get_subordinate_calls(&self) -> u64 {
        self.get_subordinate_calls.load(Ordering::SeqCst) as u64
    }
    #[allow(dead_code)]
    pub fn get_subordinates_calls(&self) -> u64 {
        self.get_subordinates_calls.load(Ordering::SeqCst) as u64
    }
    pub fn receive_incoming_message_calls(&self) -> u64 {
        self.receive_incoming_message_calls.load(Ordering::SeqCst) as u64
    }

    /// Borrow the last `(SessionId, IncomingMessageInput)` payload
    /// passed to `receive_incoming_message`.
    pub fn last_received_incoming_message(&self) -> Option<(SessionId, IncomingMessageInput)> {
        self.last_received_incoming_message
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

    async fn list_sessions(&self, project_path: String) -> Result<Vec<SessionInfo>> {
        *self.last_list_sessions_project.lock().expect("MockBackend mutex") = Some(project_path);
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
        *self.last_send_input.lock().expect("MockBackend mutex") = Some((id, text));
        Ok(())
    }

    async fn interrupt(&self, id: SessionId) -> Result<()> {
        self.interrupt_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_interrupt.lock().expect("MockBackend mutex") = Some(id);
        Ok(())
    }

    /// RPC-098: ESC exit-confirmation tests rely on this override to
    /// verify that `Close Session` reaches the backend and that
    /// `Detach`/`Cancel` do NOT.
    async fn destroy_session(&self, session_id: SessionId) -> Result<()> {
        self.destroy_session_calls.fetch_add(1, Ordering::SeqCst);
        *self
            .last_destroyed_session
            .lock()
            .expect("MockBackend mutex") = Some(session_id);
        Ok(())
    }

    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        let guard = self.work_units_tx.lock().expect("MockBackend mutex");
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                // RPC-415: disconnected — hand back a pre-closed receiver
                // so the subscriber's next recv() returns RecvError::Closed.
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        let guard = self.chunks_tx.lock().expect("MockBackend mutex");
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                // chunks_tx was dropped by `close_chunks_tx` — hand back a
                // pre-closed receiver so the subscriber's next recv()
                // returns `RecvError::Closed`.
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        let guard = self.logs_tx.lock().expect("MockBackend mutex");
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }

    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        let guard = self.status_changes_tx.lock().expect("MockBackend mutex");
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }

    fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo> {
        let guard = self.session_created_tx.lock().expect("MockBackend mutex");
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
    }

    fn checkpoints_progress_rx(
        &self,
    ) -> broadcast::Receiver<codelet_rpc_types::CheckpointsProgress> {
        let guard = self
            .checkpoints_progress_tx
            .lock()
            .expect("MockBackend mutex");
        match guard.as_ref() {
            Some(tx) => tx.subscribe(),
            None => {
                let (closed_tx, closed_rx) = broadcast::channel(1);
                drop(closed_tx);
                closed_rx
            }
        }
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

    async fn restore_checkpoint_file(
        &self,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<()> {
        self.restore_checkpoint_file_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.last_restore_file.lock().expect("MockBackend mutex") =
            Some((work_unit_id, name, path));
        Ok(())
    }

    async fn restore_checkpoint_all(&self, _work_unit_id: String, _name: String) -> Result<()> {
        self.restore_checkpoint_all_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn delete_checkpoint(&self, work_unit_id: String, name: String) -> Result<()> {
        self.delete_checkpoint_calls.fetch_add(1, Ordering::SeqCst);
        *self
            .last_delete_checkpoint
            .lock()
            .expect("MockBackend mutex") = Some((work_unit_id, name));
        Ok(())
    }

    async fn delete_all_checkpoints(&self) -> Result<()> {
        self.delete_all_checkpoints_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn move_work_unit_up(&self, id: String) -> Result<()> {
        self.move_work_unit_up_calls.fetch_add(1, Ordering::SeqCst);
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
        Ok(self
            .workspace_info
            .lock()
            .expect("MockBackend mutex")
            .clone())
    }

    async fn search_files(&self, prefix: String, limit: u32) -> Result<Vec<String>> {
        // RPC-020: MockBackend returns scripted matches (or an empty
        // Vec) so tests can drive the file search popup without
        // touching the real filesystem.
        let all = self
            .file_search_results
            .lock()
            .expect("MockBackend mutex")
            .clone();
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
        session: SessionId,
        _limit: u32,
    ) -> Result<Vec<String>> {
        Ok(self
            .scripted_history
            .lock()
            .expect("MockBackend mutex")
            .get(&session)
            .cloned()
            .unwrap_or_default())
    }

    async fn persistence_search_history(
        &self,
        query: String,
    ) -> Result<Vec<codelet_rpc_types::HistoryMatch>> {
        self.search_history_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_history_query.lock().expect("MockBackend mutex") = Some(query);
        Ok(self
            .history_search_results
            .lock()
            .expect("MockBackend mutex")
            .clone())
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

    async fn set_default_model(&self, model: String) -> Result<()> {
        self.set_default_model_calls.fetch_add(1, Ordering::SeqCst);
        *self
            .last_set_default_model
            .lock()
            .expect("MockBackend mutex") = Some(model);
        Ok(())
    }

    async fn set_thinking_level(&self, session_id: SessionId, level: ThinkingLevel) -> Result<()> {
        self.set_thinking_level_calls.fetch_add(1, Ordering::SeqCst);
        *self
            .last_set_thinking_level
            .lock()
            .expect("MockBackend mutex") = Some((session_id, level));
        Ok(())
    }

    async fn get_session_role(&self, session_id: SessionId) -> Result<Option<String>> {
        self.get_session_role_calls.fetch_add(1, Ordering::SeqCst);
        *self
            .last_get_session_role
            .lock()
            .expect("MockBackend mutex") = Some(session_id.clone());
        let roles = self.session_roles.lock().expect("MockBackend mutex");
        let role = roles
            .iter()
            .find(|(s, _)| s == &session_id)
            .and_then(|(_, r)| r.clone());
        Ok(role)
    }

    async fn set_session_role(&self, session_id: SessionId, role: Option<String>) -> Result<()> {
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

    async fn send_fspec_result(&self, session_id: SessionId, result: FspecResult) -> Result<()> {
        self.send_fspec_result_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_fspec_result.lock().expect("MockBackend mutex") = Some((session_id, result));
        Ok(())
    }

    async fn clear_history(&self, session_id: SessionId) -> Result<()> {
        self.clear_history_calls.fetch_add(1, Ordering::SeqCst);
        *self
            .last_clear_history_session
            .lock()
            .expect("MockBackend mutex") = Some(session_id);
        if let Some(msg) = self
            .clear_history_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn compact_session(&self, session_id: SessionId) -> Result<CompactionResult> {
        self.compact_session_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_compact_session.lock().expect("MockBackend mutex") = Some(session_id);
        match self
            .compact_session_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(result) => Ok(result),
            Err(msg) => Err(anyhow::anyhow!("{msg}")),
        }
    }

    async fn resume_session(&self, session_id: SessionId) -> Result<()> {
        self.resume_session_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_resume_session.lock().expect("MockBackend mutex") = Some(session_id);
        if let Some(msg) = self
            .resume_session_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn get_buffered_output(
        &self,
        session_id: SessionId,
        limit: u32,
    ) -> Result<Vec<StreamChunk>> {
        self.get_buffered_output_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_get_buffered_output
            .lock()
            .expect("MockBackend mutex") = Some((session_id, limit));
        Ok(self
            .buffered_output
            .lock()
            .expect("MockBackend mutex")
            .clone())
    }

    async fn get_work_unit_context(
        &self,
        session_id: SessionId,
    ) -> Result<Option<WorkUnitContext>> {
        self.get_work_unit_context_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self
            .work_unit_contexts
            .lock()
            .expect("MockBackend mutex")
            .get(&session_id)
            .cloned())
    }

    async fn set_work_unit_context(
        &self,
        session_id: SessionId,
        context: Option<WorkUnitContext>,
    ) -> Result<()> {
        self.set_work_unit_context_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_set_work_unit_context
            .lock()
            .expect("MockBackend mutex") = Some((session_id.clone(), context.clone()));
        if let Some(msg) = self
            .set_work_unit_context_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let mut guard = self.work_unit_contexts.lock().expect("MockBackend mutex");
        match context {
            Some(c) => {
                guard.insert(session_id, c);
            }
            None => {
                guard.remove(&session_id);
            }
        }
        Ok(())
    }

    // ── RPC-052: pending-input draft persistence ─────────────────────────

    async fn get_pending_input(&self, session_id: SessionId) -> Result<Option<String>> {
        self.get_pending_input_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .get_pending_input_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let store = self.pending_input_store.lock().expect("MockBackend mutex");
        Ok(store.get(&session_id).cloned().unwrap_or(None))
    }

    async fn set_pending_input(&self, session_id: SessionId, text: Option<String>) -> Result<()> {
        self.set_pending_input_calls.fetch_add(1, Ordering::SeqCst);
        self.pending_input_writes
            .lock()
            .expect("MockBackend mutex")
            .push((session_id.clone(), text.clone()));
        if let Some(msg) = self
            .set_pending_input_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        self.pending_input_store
            .lock()
            .expect("MockBackend mutex")
            .insert(session_id, text);
        Ok(())
    }

    // ── RPC-053: pause / HITL surface ────────────────────────────────────

    async fn get_pause_state(&self, session_id: SessionId) -> Result<Option<PauseState>> {
        self.get_pause_state_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .get_pause_state_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let store = self.pause_state_store.lock().expect("MockBackend mutex");
        Ok(store.get(&session_id).cloned().unwrap_or(None))
    }

    async fn get_hitl_request(&self, session_id: SessionId) -> Result<Option<HitlRequest>> {
        self.get_hitl_request_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .get_hitl_request_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let store = self.hitl_request_store.lock().expect("MockBackend mutex");
        Ok(store.get(&session_id).cloned().unwrap_or(None))
    }

    async fn pause_resume(&self, session_id: SessionId) -> Result<()> {
        self.pause_resume_calls.fetch_add(1, Ordering::SeqCst);
        self.pause_resume_calls_log
            .lock()
            .expect("MockBackend mutex")
            .push(session_id);
        if let Some(msg) = self
            .pause_resume_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn pause_confirm(&self, session_id: SessionId, accept: bool) -> Result<()> {
        self.pause_confirm_calls.fetch_add(1, Ordering::SeqCst);
        self.pause_confirm_calls_log
            .lock()
            .expect("MockBackend mutex")
            .push((session_id, accept));
        if let Some(msg) = self
            .pause_confirm_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn pause_triple(&self, session_id: SessionId, choice: ApprovalChoice) -> Result<()> {
        self.pause_triple_calls.fetch_add(1, Ordering::SeqCst);
        self.pause_triple_calls_log
            .lock()
            .expect("MockBackend mutex")
            .push((session_id, choice));
        if let Some(msg) = self
            .pause_triple_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn send_hitl_response(
        &self,
        session_id: SessionId,
        response: HitlResponse,
    ) -> Result<()> {
        self.send_hitl_response_calls.fetch_add(1, Ordering::SeqCst);
        self.send_hitl_response_calls_log
            .lock()
            .expect("MockBackend mutex")
            .push((session_id, response));
        if let Some(msg) = self
            .send_hitl_response_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    // ── RPC-054 provider-credentials surface ─────────────────────────────

    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>> {
        self.list_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .list_provider_credentials_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        // One-shot overrides take priority: after-save → after-delete →
        // after-refresh → next_list_override → seeded list.
        if let Some(list) = self
            .provider_credentials_after_save
            .lock()
            .expect("MockBackend mutex")
            .take()
        {
            *self.provider_credentials.lock().expect("MockBackend mutex") = list.clone();
            return Ok(list);
        }
        if let Some(list) = self
            .provider_credentials_after_delete
            .lock()
            .expect("MockBackend mutex")
            .take()
        {
            *self.provider_credentials.lock().expect("MockBackend mutex") = list.clone();
            return Ok(list);
        }
        if let Some(list) = self
            .provider_credentials_after_refresh
            .lock()
            .expect("MockBackend mutex")
            .take()
        {
            *self.provider_credentials.lock().expect("MockBackend mutex") = list.clone();
            return Ok(list);
        }
        if let Some(list) = self
            .next_list_override
            .lock()
            .expect("MockBackend mutex")
            .take()
        {
            *self.provider_credentials.lock().expect("MockBackend mutex") = list.clone();
            return Ok(list);
        }
        Ok(self
            .provider_credentials
            .lock()
            .expect("MockBackend mutex")
            .clone())
    }

    async fn get_provider_credential(
        &self,
        provider_id: String,
    ) -> Result<Option<ProviderCredentialInfo>> {
        self.get_provider_credential_calls
            .fetch_add(1, Ordering::SeqCst);
        let store = self.provider_credentials.lock().expect("MockBackend mutex");
        Ok(store.iter().find(|p| p.provider_id == provider_id).cloned())
    }

    async fn set_provider_credentials(
        &self,
        provider_id: String,
        creds: ProviderCredentialInput,
    ) -> Result<()> {
        self.set_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_set_provider_credentials
            .lock()
            .expect("MockBackend mutex") = Some((provider_id.clone(), creds));
        if let Some(msg) = self
            .set_provider_credentials_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        // Mark the row as configured in the in-memory store so a follow-
        // up `list_provider_credentials` reflects the save (unless a
        // one-shot post-save override is set).
        let mut store = self.provider_credentials.lock().expect("MockBackend mutex");
        if let Some(row) = store.iter_mut().find(|p| p.provider_id == provider_id) {
            row.configured = true;
        }
        Ok(())
    }

    async fn delete_provider_credentials(&self, provider_id: String) -> Result<()> {
        self.delete_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_delete_provider_credentials
            .lock()
            .expect("MockBackend mutex") = Some(provider_id.clone());
        if let Some(msg) = self
            .delete_provider_credentials_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let mut store = self.provider_credentials.lock().expect("MockBackend mutex");
        if let Some(row) = store.iter_mut().find(|p| p.provider_id == provider_id) {
            row.configured = false;
            row.model_count = 0;
        }
        Ok(())
    }

    async fn save_profile(
        &self,
        provider_id: String,
        profile_name: String,
        definition: codelet_rpc_types::ProfileDefinition,
    ) -> Result<()> {
        self.save_profile_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_save_profile.lock().expect("MockBackend mutex") =
            Some((provider_id, profile_name, definition));
        if let Some(msg) = self
            .save_profile_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn delete_profile(&self, provider_id: String, profile_name: String) -> Result<()> {
        self.delete_profile_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_delete_profile.lock().expect("MockBackend mutex") =
            Some((provider_id, profile_name));
        if let Some(msg) = self
            .delete_profile_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn oauth_clear_tokens(&self, provider_id: String) -> Result<()> {
        self.oauth_clear_tokens_calls.fetch_add(1, Ordering::SeqCst);
        self.oauth_clear_tokens_providers
            .lock()
            .expect("MockBackend mutex")
            .push(provider_id.clone());
        if let Some(msg) = self
            .oauth_clear_tokens_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        // Mirror the real clear: the provider's OAuth tokens go away, so a
        // follow-up `list_provider_credentials` reflects it as unconfigured
        // (projection then drops the oauth-status / Logout row). Idempotent.
        let mut store = self.provider_credentials.lock().expect("MockBackend mutex");
        if let Some(row) = store.iter_mut().find(|p| p.provider_id == provider_id) {
            row.configured = false;
            row.masked_key = None;
            row.source = None;
        }
        Ok(())
    }

    async fn oauth_get_tokens(&self, provider_id: String) -> Result<bool> {
        self.oauth_get_tokens_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self
            .oauth_get_tokens_results
            .lock()
            .expect("MockBackend mutex")
            .get(&provider_id)
            .copied()
            .unwrap_or(false))
    }

    async fn oauth_browser_login(&self, provider_id: String) -> Result<()> {
        self.oauth_browser_login_calls
            .fetch_add(1, Ordering::SeqCst);
        self.oauth_browser_login_providers
            .lock()
            .expect("MockBackend mutex")
            .push(provider_id);
        if let Some(msg) = self
            .oauth_browser_login_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn oauth_headless_start(
        &self,
        _provider_id: String,
    ) -> Result<codelet_rpc_types::OAuthHeadlessStart> {
        self.oauth_headless_start_calls
            .fetch_add(1, Ordering::SeqCst);
        let (authorize_url, pkce_verifier) = self
            .oauth_headless_start_result
            .lock()
            .expect("MockBackend mutex")
            .clone();
        Ok(codelet_rpc_types::OAuthHeadlessStart {
            authorize_url,
            pkce_verifier,
        })
    }

    async fn oauth_headless_complete(
        &self,
        provider_id: String,
        code_with_state: String,
        pkce_verifier: String,
    ) -> Result<()> {
        self.oauth_headless_complete_calls
            .fetch_add(1, Ordering::SeqCst);
        self.oauth_headless_complete_args
            .lock()
            .expect("MockBackend mutex")
            .push((provider_id, code_with_state, pkce_verifier));
        if let Some(msg) = self
            .oauth_headless_complete_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn oauth_device_start(
        &self,
        provider_id: String,
    ) -> Result<codelet_rpc_types::OAuthDeviceStart> {
        self.oauth_device_start_calls.fetch_add(1, Ordering::SeqCst);
        self.oauth_device_start_providers
            .lock()
            .expect("MockBackend mutex")
            .push(provider_id);
        if let Some(msg) = self
            .oauth_device_start_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let (user_code, verification_url, device_auth_id, interval) = self
            .oauth_device_start_result
            .lock()
            .expect("MockBackend mutex")
            .clone();
        Ok(codelet_rpc_types::OAuthDeviceStart {
            user_code,
            verification_url,
            device_auth_id,
            interval,
        })
    }

    async fn oauth_device_poll(
        &self,
        _provider_id: String,
        _device_auth_id: String,
        _interval: u64,
    ) -> Result<()> {
        self.oauth_device_poll_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .oauth_device_poll_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn oauth_copilot_device_start(
        &self,
        enterprise_host: Option<String>,
    ) -> Result<codelet_rpc_types::OAuthDeviceStart> {
        self.oauth_copilot_device_start_calls
            .fetch_add(1, Ordering::SeqCst);
        self.oauth_copilot_device_start_hosts
            .lock()
            .expect("MockBackend mutex")
            .push(enterprise_host);
        if let Some(msg) = self
            .oauth_copilot_device_start_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let (user_code, verification_url, device_auth_id, interval) = self
            .oauth_copilot_device_start_result
            .lock()
            .expect("MockBackend mutex")
            .clone();
        Ok(codelet_rpc_types::OAuthDeviceStart {
            user_code,
            verification_url,
            device_auth_id,
            interval,
        })
    }

    async fn test_provider_connection(&self, provider_id: String) -> Result<TestConnectionResult> {
        self.test_provider_connection_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_test_provider_connection
            .lock()
            .expect("MockBackend mutex") = Some(provider_id.clone());
        if let Some(msg) = self
            .test_provider_connection_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let scripted = self
            .test_connection_results
            .lock()
            .expect("MockBackend mutex")
            .get(&provider_id)
            .cloned();
        Ok(scripted.unwrap_or(TestConnectionResult {
            success: true,
            error: None,
            latency_ms: 0,
        }))
    }

    async fn refresh_models_cache(&self, provider_id: String) -> Result<Vec<ModelEntry>> {
        self.refresh_models_cache_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_refresh_models_cache
            .lock()
            .expect("MockBackend mutex") = Some(provider_id.clone());
        if let Some(msg) = self
            .refresh_models_cache_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let scripted = self
            .refresh_models_results
            .lock()
            .expect("MockBackend mutex")
            .get(&provider_id)
            .cloned();
        Ok(scripted.unwrap_or_default())
    }

    // ── RPC-055 debug-capture surface ────────────────────────────────

    async fn toggle_debug(&self, session_id: SessionId, debug_dir: String) -> Result<String> {
        self.toggle_debug_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_toggle_debug.lock().expect("MockBackend mutex") = Some((session_id, debug_dir));
        match self
            .toggle_debug_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(path) => Ok(path),
            Err(msg) => Err(anyhow::anyhow!("{msg}")),
        }
    }

    async fn set_debug_directory(&self, path: String) -> Result<()> {
        self.set_debug_directory_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_set_debug_directory
            .lock()
            .expect("MockBackend mutex") = Some(path);
        if let Some(msg) = self
            .set_debug_directory_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    // ── RPC-430 debug hydration / propagation ──────────────────────────

    async fn get_debug_enabled(&self, _session_id: SessionId) -> Result<bool> {
        self.get_debug_enabled_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .get_debug_enabled_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(val) => Ok(val),
            Err(msg) => Err(anyhow::anyhow!("{msg}")),
        }
    }

    async fn set_debug_enabled(&self, session_id: SessionId, enabled: bool) -> Result<()> {
        self.set_debug_enabled_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_set_debug_enabled.lock().expect("MockBackend mutex") = Some((session_id, enabled));
        Ok(())
    }

    async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        self.blocklist_list_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .blocklist_list_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(self
            .blocklist_rules
            .lock()
            .expect("MockBackend mutex")
            .clone())
    }

    async fn merge_session_worktree(
        &self,
        _session_id: SessionId,
        _strategy: codelet_rpc_types::MergeStrategy,
    ) -> Result<codelet_rpc_types::MergeOutcome> {
        self.merge_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .merge_session_worktree_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(self
            .merge_outcome
            .lock()
            .expect("MockBackend mutex")
            .clone())
    }

    async fn discard_session_worktree(&self, _session_id: SessionId) -> Result<()> {
        self.discard_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .discard_session_worktree_error
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }

    async fn prune_orphaned_worktrees(&self) -> Result<Vec<String>> {
        self.prune_orphaned_worktrees_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self
            .pruned_sessions
            .lock()
            .expect("MockBackend mutex")
            .clone())
    }

    async fn list_session_worktrees(&self) -> Result<Vec<codelet_rpc_types::SessionWorktreeInfo>> {
        self.list_session_worktrees_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self
            .session_worktrees
            .lock()
            .expect("MockBackend mutex")
            .clone())
    }

    async fn inspect_session_changes(
        &self,
        _session_id: SessionId,
    ) -> Result<codelet_rpc_types::SessionChangesSummary> {
        self.inspect_session_changes_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self
            .session_changes_summary
            .lock()
            .expect("MockBackend mutex")
            .clone())
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule.
    // ─────────────────────────────────────────────────────────────────

    async fn schedule_add(
        &self,
        _job: codelet_rpc_types::ScheduledJob,
    ) -> Result<codelet_rpc_types::ScheduledJob> {
        self.schedule_add_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .schedule_add_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(j) => Ok(j),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    async fn schedule_list(&self) -> Result<Vec<codelet_rpc_types::ScheduledJob>> {
        self.schedule_list_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .schedule_list_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    async fn schedule_pause(&self, _name: String) -> Result<codelet_rpc_types::ScheduledJob> {
        self.schedule_pause_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .schedule_pause_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(j) => Ok(j),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    async fn schedule_resume(&self, _name: String) -> Result<codelet_rpc_types::ScheduledJob> {
        self.schedule_resume_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .schedule_resume_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(j) => Ok(j),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    async fn schedule_remove(&self, _name: String) -> Result<()> {
        self.schedule_remove_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .schedule_remove_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop.
    // ─────────────────────────────────────────────────────────────────

    async fn loop_add(
        &self,
        _session_id: SessionId,
        _interval_seconds: u32,
        _prompt: String,
    ) -> Result<codelet_rpc_types::RegisteredLoop> {
        self.loop_add_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .loop_add_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(j) => Ok(j),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    async fn loop_cancel(&self, _id: String) -> Result<bool> {
        self.loop_cancel_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .loop_cancel_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(b) => Ok(b),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    async fn loop_list(
        &self,
        _session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::RegisteredLoop>> {
        self.loop_list_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .loop_list_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(v) => Ok(v),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    async fn create_isolated_session(&self, _role: Option<String>) -> Result<IsolatedSessionInfo> {
        self.create_isolated_session_calls
            .fetch_add(1, Ordering::SeqCst);
        match self
            .create_isolated_session_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(info) => Ok(info),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    // ── RPC-061 supervisor / subordinate forwarders ─────────────────────

    async fn get_supervisors(&self, session_id: SessionId) -> Result<Vec<SessionId>> {
        self.get_supervisors_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self
            .supervisors_results
            .lock()
            .expect("MockBackend mutex")
            .get(&session_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn add_supervisor(
        &self,
        _subordinate_id: SessionId,
        _supervisor_id: SessionId,
    ) -> Result<()> {
        self.add_supervisor_calls.fetch_add(1, Ordering::SeqCst);
        match self
            .add_supervisor_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }

    async fn remove_supervisor(&self, _supervisor_id: SessionId) -> Result<()> {
        self.remove_supervisor_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn get_subordinate(&self, _supervisor_id: SessionId) -> Result<Option<SessionId>> {
        self.get_subordinate_calls.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }

    async fn get_subordinates(&self, _supervisor_id: SessionId) -> Result<Vec<SessionId>> {
        self.get_subordinates_calls.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }

    async fn receive_incoming_message(
        &self,
        subordinate_id: SessionId,
        message: IncomingMessageInput,
    ) -> Result<()> {
        self.receive_incoming_message_calls
            .fetch_add(1, Ordering::SeqCst);
        *self
            .last_received_incoming_message
            .lock()
            .expect("MockBackend mutex") = Some((subordinate_id, message));
        match self
            .receive_incoming_message_result
            .lock()
            .expect("MockBackend mutex")
            .clone()
        {
            Ok(()) => Ok(()),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
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

// ─────────────────────────────────────────────────────────────────────────
// RPC-065 — AppTestHarness sub-module
// ─────────────────────────────────────────────────────────────────────────

pub mod harness;
