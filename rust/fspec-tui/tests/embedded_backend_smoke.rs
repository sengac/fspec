//! Embedded backend smoke test (RPC-008).
//!
//! Feature: spec/features/fspec-tui-embedded-backend.feature
//! Scenario: EmbeddedFspecBackend smoke test round-trips list_work_units
//! and work_units_rx
//!
//! Constructs a real EmbeddedFspecBackend wrapping a real tempdir-backed
//! WorkUnitsWatcher hosting a real SharedFspecService (via the
//! `temp_service` fixture in common/mod.rs), then exercises
//! `list_work_units().await` and `work_units_rx()` against the same data
//! the watcher's `snapshot()` returns. Mirrors the existing baseline in
//! `rust/rpc-embedded/tests/embedded_happy_path.rs` (the seed JSON is
//! shared verbatim) so cross-transport parity is straightforward to spot
//! when the WS smoke test (`tests/ws_backend_smoke.rs`) lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::time::Duration;

use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend};
use codelet_rpc_types::WorkUnitInfo;

mod common;

#[tokio::test]
async fn embedded_backend_round_trips_list_work_units_and_work_units_rx() {
    // @step Given a tempdir-backed WorkUnitsWatcher hosting a SharedFspecService
    let (dir, service) = common::temp_service();

    // @step And an EmbeddedFspecBackend constructed via `EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service)`
    let handle = tokio::runtime::Handle::current();
    let backend = EmbeddedFspecBackend::new(handle, std::sync::Arc::clone(&service));

    // @step When the test calls `backend.list_work_units().await`
    let actual: Vec<WorkUnitInfo> = backend.list_work_units().await.expect("list_work_units");

    // @step Then the returned Vec<WorkUnitInfo> equals the watcher's snapshot()
    let mut actual_ids: Vec<String> = actual.iter().map(|w| w.id.clone()).collect();
    actual_ids.sort();
    assert_eq!(
        actual_ids,
        vec!["AUTH-001".to_string(), "AUTH-002".to_string()],
        "list_work_units must round-trip the seeded fixture exactly"
    );

    // @step When the test subscribes via `backend.work_units_rx()` and the workspace receives a fresh fs event
    let mut rx = backend.work_units_rx();

    // Mutate the spec/work-units.json file to trigger the watcher's
    // broadcast. The watcher debounces fs events, so we use a single
    // atomic-rewrite of the file (overwrite, not append) to ensure a
    // clean update event.
    let new_payload = r#"{"workUnits":{"AUTH-001":{"id":"AUTH-001","title":"User Login","type":"story","status":"done","estimate":5,"epic":"authentication"},"AUTH-002":{"id":"AUTH-002","title":"Password reset","type":"story","status":"implementing","estimate":3,"epic":"authentication"},"AUTH-003":{"id":"AUTH-003","title":"OAuth provider","type":"story","status":"backlog","estimate":8,"epic":"authentication"}}}"#;
    fs::write(dir.path().join("spec").join("work-units.json"), new_payload)
        .expect("rewrite work-units.json");

    // @step Then a Vec<WorkUnitInfo> reflecting the new state arrives on the receiver within 5 seconds
    let next: Vec<WorkUnitInfo> = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("broadcast::Receiver::recv timed out after 5s")
        .expect("broadcast channel closed");

    let mut next_ids: Vec<String> = next.iter().map(|w| w.id.clone()).collect();
    next_ids.sort();
    assert_eq!(
        next_ids,
        vec![
            "AUTH-001".to_string(),
            "AUTH-002".to_string(),
            "AUTH-003".to_string(),
        ],
        "work_units_rx must observe the post-rewrite snapshot",
    );

    drop(dir);
}
