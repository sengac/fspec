//! RPC-058 — Cross-transport parity for the /schedule RPC surface.
//!
//! Feature: spec/features/rpc058-schedule-cross-transport-parity.feature
//!
//! Drives identical scripted scenarios against EmbeddedFspecBackend AND
//! WebSocketFspecBackend, constructed against the SAME deterministic
//! StubSessionManagerHandle. Mirrors the RPC-056 / RPC-057
//! cross-transport parity patterns.

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
use codelet_rpc_types::ScheduledJob;
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

fn sample_job() -> ScheduledJob {
    ScheduledJob {
        name: "daily".to_string(),
        cron: "0 9 * * *".to_string(),
        timezone: "UTC".to_string(),
        job_type: "agent".to_string(),
        status: "active".to_string(),
        created_at: None,
        last_run_at: None,
        last_run_status: None,
        role: Some("reviewer".to_string()),
        prompt: Some("daily standup".to_string()),
        command: None,
        overlap_policy: Some("skip".to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket schedule_add both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn schedule_add_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with a ScheduledJob { name: "daily", cron: "0 9 * * *", timezone: "UTC", job_type: "agent", status: "active", role: Some("reviewer"), prompt: Some("daily standup"), command: None, overlap_policy: Some("skip") } behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    let (_temp, service, stub) = build_service();
    stub.seed_scheduled_job(sample_job());
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.schedule_add_calls();

    let job = sample_job();

    // @step When schedule_add is called via the embedded transport with name "daily" and cron "0 9 * * *" and timezone "UTC" and job_type "agent" and role Some("reviewer") and prompt Some("daily standup") and command None and overlap_policy Some("skip")
    let em = embedded
        .schedule_add(job.clone())
        .await
        .expect("embedded schedule_add");

    // @step And schedule_add is called via the WebSocket transport with name "daily" and cron "0 9 * * *" and timezone "UTC" and job_type "agent" and role Some("reviewer") and prompt Some("daily standup") and command None and overlap_policy Some("skip")
    let ws = websocket
        .schedule_add(job.clone())
        .await
        .expect("websocket schedule_add");

    // @step Then the stub's schedule_add_calls counter equals 2
    assert_eq!(
        stub.schedule_add_calls() - initial,
        2,
        "schedule_add_calls should increment by 2"
    );

    // @step And both calls return Ok(ScheduledJob) with byte-identical field values
    assert_eq!(em, ws, "embedded and websocket schedule_add must match");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket schedule_list both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn schedule_list_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with two ScheduledJob rows behind both transports
    let (_temp, service, stub) = build_service();
    let mut second = sample_job();
    second.name = "backup".to_string();
    second.job_type = "shell".to_string();
    second.role = None;
    second.prompt = None;
    second.command = Some("tar -czf /tmp/backup.tar.gz ~/work".to_string());
    stub.seed_scheduled_jobs(vec![sample_job(), second]);
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.schedule_list_calls();

    // @step When schedule_list is called via the embedded transport
    let em = embedded
        .schedule_list()
        .await
        .expect("embedded schedule_list");

    // @step And schedule_list is called via the WebSocket transport
    let ws = websocket
        .schedule_list()
        .await
        .expect("websocket schedule_list");

    // @step Then the stub's schedule_list_calls counter equals 2
    assert_eq!(
        stub.schedule_list_calls() - initial,
        2,
        "schedule_list_calls should increment by 2"
    );

    // @step And both calls return a Vec of length 2
    assert_eq!(em.len(), 2);
    assert_eq!(ws.len(), 2);

    // @step And each entry has identical name, cron, timezone, job_type, status, role, prompt, command, overlap_policy fields across the two transports
    for (e, w) in em.iter().zip(ws.iter()) {
        assert_eq!(e, w);
    }
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket schedule_pause both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn schedule_pause_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with a ScheduledJob whose status is "paused" behind both transports
    let (_temp, service, stub) = build_service();
    let mut paused = sample_job();
    paused.status = "paused".to_string();
    stub.seed_scheduled_job(paused);
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.schedule_pause_calls();

    // @step When schedule_pause is called via the embedded transport with name "daily"
    let em = embedded
        .schedule_pause("daily".to_string())
        .await
        .expect("embedded schedule_pause");

    // @step And schedule_pause is called via the WebSocket transport with name "daily"
    let ws = websocket
        .schedule_pause("daily".to_string())
        .await
        .expect("websocket schedule_pause");

    // @step Then the stub's schedule_pause_calls counter equals 2
    assert_eq!(
        stub.schedule_pause_calls() - initial,
        2,
        "schedule_pause_calls should increment by 2"
    );

    // @step And both calls return Ok(ScheduledJob) with status equal to "paused"
    assert_eq!(em.status, "paused");
    assert_eq!(ws.status, "paused");
    assert_eq!(em, ws);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket schedule_resume both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn schedule_resume_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded with a ScheduledJob whose status is "active" behind both transports
    let (_temp, service, stub) = build_service();
    stub.seed_scheduled_job(sample_job());
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.schedule_resume_calls();

    // @step When schedule_resume is called via the embedded transport with name "daily"
    let em = embedded
        .schedule_resume("daily".to_string())
        .await
        .expect("embedded schedule_resume");

    // @step And schedule_resume is called via the WebSocket transport with name "daily"
    let ws = websocket
        .schedule_resume("daily".to_string())
        .await
        .expect("websocket schedule_resume");

    // @step Then the stub's schedule_resume_calls counter equals 2
    assert_eq!(
        stub.schedule_resume_calls() - initial,
        2,
        "schedule_resume_calls should increment by 2"
    );

    // @step And both calls return Ok(ScheduledJob) with status equal to "active"
    assert_eq!(em.status, "active");
    assert_eq!(ws.status, "active");
    assert_eq!(em, ws);
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Embedded and WebSocket schedule_remove both reach the stub
// ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn schedule_remove_round_trips_identically_across_transports() {
    // @step Given a StubSessionManagerHandle seeded to return Ok(()) for schedule_remove behind both transports
    let (_temp, service, stub) = build_service();
    let (embedded, websocket) = dual_backends(service).await;
    let initial = stub.schedule_remove_calls();

    // @step When schedule_remove is called via the embedded transport with name "daily"
    embedded
        .schedule_remove("daily".to_string())
        .await
        .expect("embedded schedule_remove");

    // @step And schedule_remove is called via the WebSocket transport with name "daily"
    websocket
        .schedule_remove("daily".to_string())
        .await
        .expect("websocket schedule_remove");

    // @step Then the stub's schedule_remove_calls counter equals 2
    assert_eq!(
        stub.schedule_remove_calls() - initial,
        2,
        "schedule_remove_calls should increment by 2"
    );

    // @step And both calls return Ok(())
    // (already asserted via expect("..."))
}
