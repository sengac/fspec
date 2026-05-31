//! RPC-056 — Cross-transport parity for the /blocklist RPC surface.
//!
//! Feature: spec/features/rpc056-blocklist-view-cross-transport-parity.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend AND
//! WebSocketFspecBackend, constructed against the SAME deterministic
//! StubSessionManagerHandle. Mirrors the RPC-054 / RPC-055 cross-transport
//! parity patterns.

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

use codelet_core::session_manager_handle::{SessionManagerHandle, StubSessionManagerHandle};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::bind_and_serve;
use codelet_rpc_types::BlocklistRuleInfo;
use tempfile::TempDir;

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

fn build_service() -> (TempDir, Arc<SharedFspecService>, Arc<StubSessionManagerHandle>) {
    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path().to_path_buf();
    workspace_with_seed(&cwd);
    let watcher = Arc::new(WorkUnitsWatcher::new(&cwd).expect("watcher"));
    let stub = Arc::new(StubSessionManagerHandle::new());
    // Seed the stub with three rules so both transports observe the same
    // payload.
    stub.seed_blocklist_rules(vec![
        BlocklistRuleInfo {
            id: "git-checkout-block".to_string(),
            pattern: "^git\\s+checkout\\b".to_string(),
            action: "block".to_string(),
            reason: "Use git switch instead".to_string(),
            guidance: Some("git switch <branch>".to_string()),
            source: "system".to_string(),
        },
        BlocklistRuleInfo {
            id: "cat-block".to_string(),
            pattern: "^cat\\s+".to_string(),
            action: "block".to_string(),
            reason: "Use the Read tool for file reading".to_string(),
            guidance: None,
            source: "project".to_string(),
        },
        BlocklistRuleInfo {
            id: "etc-passwd".to_string(),
            pattern: "/etc/passwd".to_string(),
            action: "block".to_string(),
            reason: "System file is sensitive".to_string(),
            guidance: None,
            source: "system".to_string(),
        },
    ]);
    let handle: Arc<dyn SessionManagerHandle> = stub.clone();
    let service = Arc::new(
        SharedFspecService::with_session_manager(watcher, handle).with_cwd(cwd),
    );
    (temp, service, stub)
}

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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket blocklist_list both reach the stub
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn blocklist_list_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with three rules behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;

    let initial = stub.blocklist_list_calls();

    // @step When blocklist_list is called via the embedded transport
    let em = embedded.blocklist_list().await.expect("embedded blocklist_list");

    // @step And blocklist_list is called via the WebSocket transport
    let ws = websocket.blocklist_list().await.expect("websocket blocklist_list");

    // @step Then the stub's blocklist_list_calls counter equals 2
    let final_calls = stub.blocklist_list_calls();
    assert_eq!(
        final_calls - initial,
        2,
        "stub.blocklist_list_calls() should increment by 2 (once per transport)"
    );

    // @step And both calls return a Vec of length 3
    assert_eq!(em.len(), 3, "embedded blocklist_list returned {} rows", em.len());
    assert_eq!(ws.len(), 3, "websocket blocklist_list returned {} rows", ws.len());

    // @step And each entry has identical id, pattern, action, source fields across the two transports
    for (e, w) in em.iter().zip(ws.iter()) {
        assert_eq!(e.id, w.id, "id mismatch across transports");
        assert_eq!(e.pattern, w.pattern, "pattern mismatch across transports");
        assert_eq!(e.action, w.action, "action mismatch across transports");
        assert_eq!(e.source, w.source, "source mismatch across transports");
    }
}
