//! RPC-061 — Cross-transport parity for the supervisor / subordinate
//! links surface.
//!
//! Feature: spec/features/rpc061-cross-transport-parity.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend AND
//! WebSocketFspecBackend, constructed against the SAME deterministic
//! StubSessionManagerHandle. Mirrors the RPC-059 cross-transport parity
//! pattern.

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
use codelet_rpc_types::{IncomingMessageInput, SessionId};
use tempfile::TempDir;

fn workspace_with_seed(cwd: &Path) {
    fs::create_dir_all(cwd.join("spec")).expect("mkdir spec/");
    fs::write(
        cwd.join("spec").join("work-units.json"),
        r#"{"workUnits":{}}"#,
    )
    .expect("write work-units.json");
}

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

fn sample_input(source: &str, role: &str, message: &str) -> IncomingMessageInput {
    IncomingMessageInput {
        source_session_id: source.to_string(),
        role_name: role.to_string(),
        message: message.to_string(),
        images: None,
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket add_supervisor both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn add_supervisor_round_trips_identically_across_transports() {
    // @step Given a fresh StubSessionManagerHandle behind both transports
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.add_supervisor_calls();

    // @step When add_supervisor is called via the embedded transport with subordinate=SessionId("sub-em") and supervisor=SessionId("sup")
    embedded
        .add_supervisor(SessionId::new("sub-em"), SessionId::new("sup"))
        .await
        .expect("embedded add_supervisor");

    // @step And add_supervisor is called via the WebSocket transport with subordinate=SessionId("sub-ws") and supervisor=SessionId("sup")
    websocket
        .add_supervisor(SessionId::new("sub-ws"), SessionId::new("sup"))
        .await
        .expect("websocket add_supervisor");

    // @step Then the stub's add_supervisor_calls counter increased by 2
    assert_eq!(
        stub.add_supervisor_calls() - initial,
        2,
        "add_supervisor_calls should increment by 2"
    );

    // @step And the stub now reports two subordinates for "sup"
    let subs = stub.get_subordinates(&SessionId::new("sup"));
    assert_eq!(subs.len(), 2, "stub should report two subordinates");
    let strings: Vec<String> = subs.iter().map(|s| s.value.clone()).collect();
    assert!(strings.contains(&"sub-em".to_string()));
    assert!(strings.contains(&"sub-ws".to_string()));
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket get_supervisors return identical lists
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_supervisors_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with add_supervisor(sub, sup)
    let (_temp, service, stub) = build_service();
    stub.add_supervisor(&SessionId::new("sub"), &SessionId::new("sup"))
        .expect("seed add_supervisor");
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.get_supervisors_calls();

    // @step When get_supervisors is called via the embedded transport
    let em = embedded
        .get_supervisors(SessionId::new("sub"))
        .await
        .expect("embedded get_supervisors");
    // @step And get_supervisors is called via the WebSocket transport
    let ws = websocket
        .get_supervisors(SessionId::new("sub"))
        .await
        .expect("websocket get_supervisors");

    // @step Then both calls return [SessionId("sup")]
    assert_eq!(em, vec![SessionId::new("sup")]);
    assert_eq!(ws, vec![SessionId::new("sup")]);
    assert_eq!(em, ws);

    // @step And the stub's get_supervisors_calls counter increased by 2
    assert_eq!(
        stub.get_supervisors_calls() - initial,
        2,
        "get_supervisors_calls should increment by 2"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket get_subordinates return identical lists
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_subordinates_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with two subordinates of "sup"
    let (_temp, service, stub) = build_service();
    stub.add_supervisor(&SessionId::new("sub-a"), &SessionId::new("sup"))
        .expect("seed add_supervisor a");
    stub.add_supervisor(&SessionId::new("sub-b"), &SessionId::new("sup"))
        .expect("seed add_supervisor b");
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.get_subordinates_calls();

    // @step When get_subordinates is called via the embedded transport
    let em = embedded
        .get_subordinates(SessionId::new("sup"))
        .await
        .expect("embedded get_subordinates");
    // @step And get_subordinates is called via the WebSocket transport
    let ws = websocket
        .get_subordinates(SessionId::new("sup"))
        .await
        .expect("websocket get_subordinates");

    // @step Then both calls return [SessionId("sub-a"), SessionId("sub-b")] (same order)
    assert_eq!(em.len(), 2);
    assert_eq!(em, ws);
    assert_eq!(
        stub.get_subordinates_calls() - initial,
        2,
        "get_subordinates_calls should increment by 2"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket get_subordinate (single)
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_subordinate_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with add_supervisor(sub, sup)
    let (_temp, service, stub) = build_service();
    stub.add_supervisor(&SessionId::new("sub"), &SessionId::new("sup"))
        .expect("seed add_supervisor");
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.get_subordinate_calls();

    // @step When get_subordinate is called via each transport
    let em = embedded
        .get_subordinate(SessionId::new("sup"))
        .await
        .expect("embedded get_subordinate");
    let ws = websocket
        .get_subordinate(SessionId::new("sup"))
        .await
        .expect("websocket get_subordinate");

    // @step Then both calls return Some(SessionId("sub"))
    assert_eq!(em, Some(SessionId::new("sub")));
    assert_eq!(ws, Some(SessionId::new("sub")));

    assert_eq!(
        stub.get_subordinate_calls() - initial,
        2,
        "get_subordinate_calls should increment by 2"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket receive_incoming_message both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn receive_incoming_message_round_trips_identically_across_transports() {
    // @step Given a fresh StubSessionManagerHandle behind both transports
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.receive_incoming_message_calls();

    let em_input = sample_input("sup", "reviewer", "embedded fix lint");
    let ws_input = sample_input("sup", "reviewer", "websocket fix lint");

    // @step When receive_incoming_message is called via the embedded transport
    embedded
        .receive_incoming_message(SessionId::new("sub"), em_input.clone())
        .await
        .expect("embedded receive_incoming_message");

    // @step And receive_incoming_message is called via the WebSocket transport
    websocket
        .receive_incoming_message(SessionId::new("sub"), ws_input.clone())
        .await
        .expect("websocket receive_incoming_message");

    // @step Then the stub's receive_incoming_message_calls counter increased by 2
    assert_eq!(
        stub.receive_incoming_message_calls() - initial,
        2,
        "receive_incoming_message_calls should increment by 2"
    );

    // @step And the stub's recorded_incoming_messages contains both payloads
    let recorded = stub.recorded_incoming_messages();
    let messages: Vec<String> = recorded
        .iter()
        .map(|(_, input)| input.message.clone())
        .collect();
    assert!(
        messages.iter().any(|m| m == "embedded fix lint"),
        "stub should have recorded embedded payload"
    );
    assert!(
        messages.iter().any(|m| m == "websocket fix lint"),
        "stub should have recorded websocket payload"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket remove_supervisor clear state
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn remove_supervisor_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with add_supervisor(sub, sup)
    let (_temp, service, stub) = build_service();
    stub.add_supervisor(&SessionId::new("sub"), &SessionId::new("sup"))
        .expect("seed add_supervisor");
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.remove_supervisor_calls();

    // @step When remove_supervisor is called via the embedded transport
    embedded
        .remove_supervisor(SessionId::new("sup"))
        .await
        .expect("embedded remove_supervisor");

    // @step Then the stub now reports no subordinates for "sup"
    assert!(
        stub.get_subordinates(&SessionId::new("sup")).is_empty(),
        "stub should report no subordinates after remove_supervisor"
    );

    // @step When the call is repeated via the WebSocket transport (idempotent)
    websocket
        .remove_supervisor(SessionId::new("sup"))
        .await
        .expect("websocket remove_supervisor");

    // @step Then both transports landed exactly one call each on the stub
    assert_eq!(
        stub.remove_supervisor_calls() - initial,
        2,
        "remove_supervisor_calls should increment by 2"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Stub rejects circular add_supervisor identically on both transports
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn circular_add_supervisor_is_rejected_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with add_supervisor(sub, sup)
    let (_temp, service, stub) = build_service();
    stub.add_supervisor(&SessionId::new("sub"), &SessionId::new("sup"))
        .expect("seed add_supervisor");
    let (embedded, websocket) = dual_backends(service).await;

    // @step When add_supervisor(sup, sub) is attempted via the embedded transport
    let em_err = embedded
        .add_supervisor(SessionId::new("sup"), SessionId::new("sub"))
        .await;

    // @step Then it returns Err with message "circular supervision not allowed"
    let em_msg = em_err
        .expect_err("embedded should reject cycle")
        .to_string();
    assert!(
        em_msg.contains("circular supervision not allowed"),
        "embedded error must include 'circular supervision not allowed': {em_msg}"
    );

    // @step And the same call via WebSocket returns the same error
    let ws_err = websocket
        .add_supervisor(SessionId::new("sup"), SessionId::new("sub"))
        .await;
    let ws_msg = ws_err
        .expect_err("websocket should reject cycle")
        .to_string();
    assert!(
        ws_msg.contains("circular supervision not allowed"),
        "websocket error must include 'circular supervision not allowed': {ws_msg}"
    );
}
