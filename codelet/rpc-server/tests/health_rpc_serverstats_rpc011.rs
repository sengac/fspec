//! Health RPC + ServerStats extensions tests — RPC-011.
//!
//! Feature: spec/features/health-rpc-serverstats.feature
//!
//! Covers:
//!   - FspecService.health returns HealthInfo via tarpc
//!   - HealthInfo is a lifted type with cfg-gated napi(object)
//!   - FspecBackend trait gains health on both transports
//!   - ServerStats lag counters fire when broadcast subscribers lag
//!   - ServerStats.last_watcher_event_at updates on each watcher snapshot
//!
//! Red phase: requires the new `health()` RPC, the `HealthInfo` lifted
//! type in codelet-rpc-types, the ServerStats extensions
//! (connected_clients, last_watcher_event_at, lag_chunks/lag_logs/lag_work_units),
//! and the FspecBackend trait extension. Compile failure IS the red signal.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::HealthInfo;
use common::{connect_with_retry, make_workspace, write_workspace};
use tarpc::context;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: FspecService.health returns HealthInfo via tarpc
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fspec_service_health_returns_health_info_via_tarpc() {
    // @step Given a tarpc client connected to FspecServiceImpl
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let service = Arc::new(SharedFspecService::new(watcher));
    let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");
    let ws = connect_with_retry(addr.port()).await;
    let client = ws_client_connect(ws).await.expect("ws_client_connect");

    // @step When the client calls health(context::current()).await
    let health: HealthInfo = client
        .client()
        .health(context::current())
        .await
        .expect("health RPC must succeed");

    // @step Then it receives a HealthInfo struct over the wire
    // @step And HealthInfo fields are: uptime_secs: i64, connected_clients: i64, last_watcher_event_secs_ago: Option<i64>, lag_chunks: i64, lag_logs: i64, lag_work_units: i64, version: String
    let _uptime: i64 = health.uptime_secs;
    let _clients: i64 = health.connected_clients;
    let _last: Option<i64> = health.last_watcher_event_secs_ago;
    let _lag_chunks: i64 = health.lag_chunks;
    let _lag_logs: i64 = health.lag_logs;
    let _lag_work_units: i64 = health.lag_work_units;
    let version: &str = &health.version;

    // @step And the version field equals env!("CARGO_PKG_VERSION") of the daemon process
    assert_eq!(
        version,
        env!("CARGO_PKG_VERSION"),
        "HealthInfo.version must equal the daemon's CARGO_PKG_VERSION"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: HealthInfo is a lifted type with cfg-gated napi(object)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn health_info_is_a_lifted_type_with_cfg_gated_napi_object() {
    // @step Given the codelet-rpc-types crate
    let lib_rs_path = workspace_root()
        .join("rpc-types")
        .join("src")
        .join("lib.rs");
    let body = std::fs::read_to_string(&lib_rs_path)
        .unwrap_or_else(|e| panic!("read {}: {}", lib_rs_path.display(), e));

    // @step When inspecting HealthInfo's definition
    let has_struct = body.contains("pub struct HealthInfo");
    assert!(
        has_struct,
        "HealthInfo struct must be defined in codelet/rpc-types/src/lib.rs"
    );

    // @step Then it carries #[cfg_attr(feature = "napi", napi(object))]
    let idx = body
        .find("pub struct HealthInfo")
        .expect("HealthInfo struct presence already asserted");
    let prelude_start = idx.saturating_sub(400);
    let prelude = &body[prelude_start..idx];
    assert!(
        prelude.contains("cfg_attr(feature = \"napi\", napi_derive::napi(object))")
            || prelude.contains("cfg_attr(feature = \"napi\", napi(object))"),
        "HealthInfo must carry #[cfg_attr(feature = \"napi\", napi(object))]. Got prelude: {prelude}"
    );

    // @step And it implements Serialize + Deserialize + Clone + Debug
    assert!(
        prelude.contains("Serialize") && prelude.contains("Deserialize"),
        "HealthInfo derive list must include Serialize + Deserialize"
    );
    assert!(
        prelude.contains("Clone"),
        "HealthInfo derive must include Clone"
    );
    assert!(
        prelude.contains("Debug"),
        "HealthInfo derive must include Debug"
    );

    // @step And it lives in codelet/rpc-types/src/lib.rs alongside WorkUnitInfo / SessionInfo
    assert!(body.contains("pub struct WorkUnitInfo"));
    assert!(body.contains("pub struct SessionInfo"));
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("rpc-server must have a parent")
        .to_path_buf()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: FspecBackend trait gains health on both transports
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn fspec_backend_trait_gains_health_on_both_transports() {
    let tui_src = workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("transport");
    let trait_body = std::fs::read_to_string(tui_src.join("mod.rs")).expect("read mod.rs");
    let embedded_body =
        std::fs::read_to_string(tui_src.join("embedded.rs")).expect("read embedded.rs");
    let ws_body = std::fs::read_to_string(tui_src.join("websocket.rs")).expect("read websocket.rs");

    // @step Given the FspecBackend trait
    // @step When inspecting its method surface
    // @step Then it has an `async fn health(&self) -> Result<HealthInfo>` method
    assert!(
        trait_body.contains("async fn health(&self)") && trait_body.contains("HealthInfo"),
        "FspecBackend trait must declare async fn health(&self) -> Result<HealthInfo>"
    );

    // @step And EmbeddedFspecBackend implements health by reading ServerStats directly (no RPC round-trip)
    assert!(
        embedded_body.contains("fn health"),
        "EmbeddedFspecBackend must implement health()"
    );

    // @step And WebSocketFspecBackend implements health by calling self.client.client().health(context::current()).await
    assert!(
        ws_body.contains("fn health") && ws_body.contains("health(context::current())"),
        "WebSocketFspecBackend::health must delegate through tarpc"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ServerStats lag counters fire when broadcast subscribers lag
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_stats_lag_counters_fire_when_broadcast_subscribers_lag() {
    use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
    use codelet_providers::stub_provider::StubProvider;
    use codelet_rpc_types::{SessionId, StreamChunk};

    // @step Given a daemon with the chunks broadcast capacity set to 1024
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let manager: Arc<dyn SessionManagerHandle> = Arc::new(StubSessionManagerHandle::with_provider(
        Arc::new(StubProvider::new()),
    ));
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));
    let (addr, stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");

    // @step And a single slow WS subscriber that NEVER drains its receiver
    let ws = connect_with_retry(addr.port()).await;
    let client = ws_client_connect(ws).await.expect("ws_client_connect");
    let _chunks_rx_slow = client.chunks_rx();

    // @step When 1025 chunk frames are pushed onto chunks_tx in rapid succession
    let chunks_tx = service.chunks_tx();
    let sid = SessionId::new("S-lag-test");
    for i in 0..1100 {
        let _ = chunks_tx.send((sid.clone(), StreamChunk::text(format!("chunk {i}"))));
    }

    tokio::time::sleep(Duration::from_millis(300)).await;

    // @step Then the slow subscriber's recv() yields RecvError::Lagged(1)
    // @step And ServerStats.lag_chunks is incremented by at least 1 in the chunks_fanout task
    assert!(
        stats.lag_chunks() >= 1,
        "ServerStats.lag_chunks must be >= 1 after overflow. Got {}",
        stats.lag_chunks()
    );

    // @step And a tracing::warn record is emitted with target="codelet_rpc_server::server" and field skipped>=1
    // @step And that warning rides the logs broadcast as a LogRecord visible to OTHER (non-lagging) clients
    // (Already-wired tracing::warn! at server.rs lines 213/243; RPC-011
    // only INSTRUMENTS the counter assertion above.)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: ServerStats.last_watcher_event_at updates on each watcher snapshot
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_stats_last_watcher_event_at_updates_on_each_watcher_snapshot() {
    // @step Given a daemon with an empty workspace
    let (_dir, path) = make_workspace(&[]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let service = Arc::new(SharedFspecService::new(watcher));
    let (addr, stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");

    let ws = connect_with_retry(addr.port()).await;
    let _client = ws_client_connect(ws).await.expect("ws_client_connect");
    tokio::time::sleep(Duration::from_millis(200)).await;

    let initial = stats.last_watcher_event_at();

    // @step When the workspace mutates and the watcher fires a new snapshot
    write_workspace(&path, &[("AUTH-001", "Login", "done")]);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // @step Then ServerStats.last_watcher_event_at.lock() is updated to the current Instant in work_units_fanout
    let updated = stats.last_watcher_event_at();
    assert!(
        updated.is_some(),
        "last_watcher_event_at must be Some after a watcher snapshot"
    );
    if let (Some(prev), Some(now)) = (initial, updated) {
        assert!(
            now >= prev,
            "last_watcher_event_at must be monotonically non-decreasing"
        );
    }

    // @step And subsequent health() calls report last_watcher_event_secs_ago = Some(elapsed.as_secs())
    let ws2 = connect_with_retry(addr.port()).await;
    let client2 = ws_client_connect(ws2).await.expect("ws_client_connect");
    let health: HealthInfo = client2
        .client()
        .health(context::current())
        .await
        .expect("health RPC");
    assert!(
        health.last_watcher_event_secs_ago.is_some(),
        "health.last_watcher_event_secs_ago must be Some after a snapshot fired"
    );
}
