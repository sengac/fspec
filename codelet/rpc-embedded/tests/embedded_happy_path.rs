//! Integration test for RPC-005 Scenario 1, post-RPC-006 watcher lift.
//!
//! Feature: spec/features/embedded-transport-rpc.feature
//!
//! This file validates the "Embedded transport returns WorkUnitInfo from a
//! single shared service impl" scenario. It is the smallest end-to-end
//! happy-path test for the dual-transport tarpc spike.
//!
//! The companion source-shape scenario "Embedded transport uses only
//! tarpc::transport::channel for in-process traffic" lives in
//! `architecture_invariants.rs` and asserts the "no network serialization"
//! half of the embedded-transport contract by inspecting source.
//!
//! After RPC-006 the "fixture" is materialised on disk in a temp
//! workspace observed by a real `WorkUnitsWatcher` rather than passed
//! in as a `Vec<WorkUnitInfo>` — the assertion (two records returned by
//! `list_work_units`) is unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::FspecServiceClient;
use codelet_rpc_embedded::{EmbeddedTransport, SharedFspecService};
use codelet_rpc_types::WorkUnitInfo;
use std::fs;
use std::sync::Arc;
use tarpc::context;

#[tokio::test]
async fn embedded_transport_returns_work_unit_info_from_shared_impl() {
    // @step Given the codelet workspace contains the rpc-types, rpc, rpc-embedded, and rpc-server crates and the shared FspecService implementation is seeded with a fixture of two WorkUnitInfo records
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("spec")).unwrap();
    fs::write(
        dir.path().join("spec").join("work-units.json"),
        r#"{"workUnits":{"AUTH-001":{"id":"AUTH-001","title":"User Login","type":"story","status":"done","description":"Sign in with email/password","estimate":5,"epic":"authentication"},"AUTH-002":{"id":"AUTH-002","title":"Password reset","type":"story","status":"implementing","estimate":3,"epic":"authentication"}}}"#,
    )
    .unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(dir.path()).unwrap());
    assert_eq!(
        watcher.snapshot().len(),
        2,
        "shared fixture must seed exactly two WorkUnitInfo records to match the Given"
    );
    let service = Arc::new(SharedFspecService::new(Arc::clone(&watcher)));

    // @step When I construct an EmbeddedTransport with the current tokio runtime Handle, obtain an FspecServiceClient, and call list_work_units on the client
    let handle = tokio::runtime::Handle::current();
    let transport = EmbeddedTransport::new(handle, service);
    let client: FspecServiceClient = transport.client();
    let result = client.list_work_units(context::current()).await;

    // @step Then the call returns Ok with a Vec<WorkUnitInfo> equal to the fixture
    let actual: Vec<WorkUnitInfo> = result.expect("RPC should succeed");
    let expected = watcher.snapshot();
    assert_eq!(actual, expected);
    let mut ids: Vec<String> = actual.into_iter().map(|wu| wu.id).collect();
    ids.sort();
    assert_eq!(ids, vec!["AUTH-001".to_string(), "AUTH-002".to_string()],);
}
