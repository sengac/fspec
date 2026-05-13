//! Reserved-variants rejection regression test after RPC-006.
//!
//! Feature: spec/features/reserved-envelope-variants-narrowed.feature
//!
//! - Scenario: Reserved envelope variants are still rejected after WorkUnitsUpdate is implemented
//!
//! After RPC-006 implements `Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>)`,
//! the reserved-variants list narrows to {Event, LogEvent, CmdReq, CmdRes}.
//! WorkUnitsUpdate is now a legitimate first-class variant and MUST NOT be
//! counted as rejected by `ServerStats::rejected_variants()`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, Envelope};
use common::{connect_with_retry, make_workspace};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_reserved_variants_still_rejected_after_work_units_update_implemented() {
    // @step Given the rpc-server is running after RPC-006
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let service = Arc::new(SharedFspecService::new(Arc::clone(&watcher)));
    let (addr, stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service))
        .await
        .expect("bind_and_serve");

    let ws = connect_with_retry(addr.port()).await;
    let (mut sink, mut _stream) = ws.split();

    // @step When a WebSocket client sends a frame whose Envelope variant is one of Event, LogEvent, CmdReq, or CmdRes
    // RPC-007 update: Event/LogEvent are now payload-bearing variants
    // (chunks and log records). They still flow server → client only,
    // so a client pushing them is still rejected — but the test must
    // construct payload-bearing values rather than the old unit
    // variants.
    let reserved: Vec<(Envelope, &'static str)> = vec![
        (
            Envelope::Event {
                session_id: codelet_rpc_types::SessionId::new(""),
                chunk: codelet_rpc_types::StreamChunk::done(),
            },
            "Event",
        ),
        (
            Envelope::LogEvent(codelet_rpc_types::LogRecord {
                level: "INFO".to_string(),
                target: "test".to_string(),
                message: "".to_string(),
                timestamp_ms: 0,
            }),
            "LogEvent",
        ),
        (Envelope::CmdReq, "CmdReq"),
        (Envelope::CmdRes, "CmdRes"),
    ];
    for (variant, _name) in &reserved {
        let bytes = bincode::serialize(variant).expect("envelope encodes");
        sink.send(Message::Binary(bytes.into()))
            .await
            .expect("send reserved envelope");
    }

    // Allow the server task to process all four frames.
    let mut waited = 0u64;
    while stats.rejected_envelopes() < reserved.len() as u64 && waited < 1000 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        waited += 10;
    }

    // @step Then the server records the unsupported variant by name in its rejection log, does not invoke any FspecService method as a result of that frame, and the rejected-variants list reported by ServerStats does not contain WorkUnitsUpdate
    assert_eq!(
        stats.rejected_envelopes(),
        reserved.len() as u64,
        "server must reject every still-reserved Envelope variant"
    );
    let recorded = stats.rejected_variants();
    let expected_names: Vec<&'static str> = reserved.iter().map(|(_, n)| *n).collect();
    assert_eq!(
        recorded, expected_names,
        "server must record each rejected variant by name in arrival order; the post-RPC-006 set is {{Event, LogEvent, CmdReq, CmdRes}}"
    );
    assert!(
        !recorded.contains(&"WorkUnitsUpdate"),
        "WorkUnitsUpdate is no longer reserved; it must NOT appear in rejected_variants"
    );
    assert_eq!(
        service.list_work_units_calls(),
        0,
        "no FspecService method must be invoked by reserved frames"
    );
}
