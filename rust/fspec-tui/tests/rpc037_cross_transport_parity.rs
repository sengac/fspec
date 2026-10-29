//! RPC-037 — Cross-transport parity for the widened SessionManagerHandle /
//! FspecService / FspecBackend surface.
//!
//! Feature: spec/features/widen-sessionmanagerhandle-fspecservice-both-backends-stub-with-cross-transport-parity-tests.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend AND
//! WebSocketFspecBackend, constructed against the SAME deterministic
//! StubSessionManagerHandle. Mirrors the RPC-026 parity pattern (shared
//! service + bind_and_serve + WS client). Every new method added by
//! RPC-037 has at least one parity assertion here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::await_holding_lock,
    clippy::too_many_lines
)]

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::{
    ApprovalChoice, FspecResult, HitlAnswer, HitlOption, HitlQuestion, HitlRequest, HitlResponse,
    PauseKind, PauseState, SessionId, SessionModel, SessionStatus, SessionTokens, StreamChunk,
    ThinkingLevel, TokenRestoreState, WorkUnitContext,
};
use tempfile::TempDir;
use tokio::time::timeout;

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

/// Build a `SharedFspecService` against the supplied workspace and a
/// fresh deterministic `StubSessionManagerHandle`. Returns the temp dir
/// (kept alive by caller), the service, and the handle (so tests can
/// seed pause/HITL state).
fn build_service() -> (
    TempDir,
    Arc<SharedFspecService>,
    Arc<StubSessionManagerHandle>,
) {
    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).expect("watcher"));
    let stub = Arc::new(StubSessionManagerHandle::new());
    let handle: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle).with_cwd(cwd));
    (temp, service, stub)
}

/// Construct both transports against the SAME `SharedFspecService` so
/// scenarios can compare their behaviour.
async fn dual_backends(
    service: Arc<SharedFspecService>,
) -> (Arc<dyn FspecBackend>, Arc<dyn FspecBackend>) {
    let embedded: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service.clone(),
    ));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service)
        .await
        .expect("bind_and_serve");
    let url = url::Url::parse(&format!("ws://{addr}/")).expect("ws url");
    let websocket: Arc<dyn FspecBackend> =
        Arc::new(WebSocketFspecBackend::connect(url).await.expect("connect"));
    (embedded, websocket)
}

include!("rpc037_cross_transport_parity/defaults_history.rs");
include!("rpc037_cross_transport_parity/restore_context.rs");
include!("rpc037_cross_transport_parity/debug_pause.rs");
include!("rpc037_cross_transport_parity/hitl_lifecycle.rs");
include!("rpc037_cross_transport_parity/stream_parity.rs");
include!("rpc037_cross_transport_parity/source_contracts.rs");
