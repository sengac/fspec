//! Reserved-variants rejection regression test after RPC-007.
//!
//! Feature: spec/features/session-rpcs-streamchunk-logevent-push-channels-repl-backend.feature
//!
//! - Scenario: Envelope::CmdReq and Envelope::CmdRes remain reserved-and-rejected while Event and LogEvent are now legitimate
//!
//! After RPC-007 implements `Envelope::Event { session_id, chunk }` and
//! `Envelope::LogEvent(LogRecord)`, the reserved-variants list narrows from
//! `{Event, LogEvent, CmdReq, CmdRes}` (RPC-006 baseline) to
//! `{CmdReq, CmdRes}`. Event and LogEvent are now legitimate first-class
//! variants and MUST NOT appear in `ServerStats::rejected_variants()`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use codelet_core::session_manager_handle::{
    SessionManagerHandle, StubSessionManagerHandle,
};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_providers::stub_provider::StubProvider;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, Envelope};
use common::{connect_with_retry, make_workspace};
use futures::SinkExt;
use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_cmd_req_and_cmd_res_still_rejected_event_and_log_event_legitimate() {
    // @step Given a WebSocket client is connected to codelet-rpc-server
    let (_dir, path) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = path.parent().unwrap().parent().unwrap();
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace).expect("watcher"));
    let manager: Arc<dyn SessionManagerHandle> = Arc::new(
        StubSessionManagerHandle::with_provider(Arc::new(StubProvider::new())),
    );
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::clone(&watcher),
        Arc::clone(&manager),
    ));
    let (addr, stats, _join) =
        bind_and_serve("127.0.0.1:0", Arc::clone(&service))
            .await
            .expect("bind_and_serve");

    let ws = connect_with_retry(addr.port()).await;
    let (mut sink, _stream) = futures::StreamExt::split(ws);

    // @step When the client sends an Envelope::CmdReq frame
    sink.send(Message::Binary(
        bincode::serialize(&Envelope::CmdReq)
            .expect("encode CmdReq")
            .into(),
    ))
    .await
    .expect("send CmdReq");

    // Allow the server task to process the frame.
    let mut waited = 0u64;
    while stats.rejected_envelopes() < 1 && waited < 1000 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        waited += 10;
    }

    // @step Then the server rejects the frame and increments ServerStats.rejected_envelopes
    assert_eq!(
        stats.rejected_envelopes(),
        1,
        "server must reject CmdReq and increment rejected_envelopes",
    );

    // @step And ServerStats.rejected_variants includes "CmdReq"
    assert!(
        stats.rejected_variants().contains(&"CmdReq"),
        "rejected_variants must include CmdReq, got {:?}",
        stats.rejected_variants(),
    );

    // @step When the client sends an Envelope::CmdRes frame
    sink.send(Message::Binary(
        bincode::serialize(&Envelope::CmdRes)
            .expect("encode CmdRes")
            .into(),
    ))
    .await
    .expect("send CmdRes");

    let mut waited = 0u64;
    while stats.rejected_envelopes() < 2 && waited < 1000 {
        tokio::time::sleep(Duration::from_millis(10)).await;
        waited += 10;
    }

    // @step Then the server rejects the frame and ServerStats.rejected_variants includes "CmdRes"
    assert_eq!(stats.rejected_envelopes(), 2);
    assert!(
        stats.rejected_variants().contains(&"CmdRes"),
        "rejected_variants must include CmdRes",
    );

    // @step And ServerStats.rejected_variants does NOT include "Event" or "LogEvent"
    let recorded = stats.rejected_variants();
    assert!(
        !recorded.contains(&"Event"),
        "Event is now a legitimate variant; must NOT appear in rejected_variants, got {recorded:?}",
    );
    assert!(
        !recorded.contains(&"LogEvent"),
        "LogEvent is now a legitimate variant; must NOT appear in rejected_variants, got {recorded:?}",
    );
    assert!(
        !recorded.contains(&"WorkUnitsUpdate"),
        "WorkUnitsUpdate remains legitimate (RPC-006); must NOT appear",
    );
}
