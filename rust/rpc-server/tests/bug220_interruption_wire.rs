//! Feature: spec/features/interruption-wire-compatibility.feature

use codelet_core::{
    session_manager_handle::StubSessionManagerHandle, work_units::WorkUnitsWatcher,
};
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::{SessionId, StreamChunk};
use serde_json::json;
use std::{sync::Arc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn scenario_typed_and_legacy_interruption_events_retain_their_meaning() -> TestResult {
    // @step Given interruption JSON with user or loopDetector reasons and an older reasonless event
    let legacy = StreamChunk::interrupted(vec!["queued".into()]);
    let legacy_bytes = bincode::serialize(&legacy)?;
    for reason in ["user", "loopDetector"] {
        let value = json!({"InterruptedWithReason":{"queuedInputs":["queued"],"reason":reason}});
        // @step When each event is decoded and encoded again
        let chunk: StreamChunk = serde_json::from_value(value.clone())?;
        let binary: StreamChunk = bincode::deserialize(&bincode::serialize(&chunk)?)?;
        // @step Then explicit causes and queued inputs survive while missing causes remain unspecified
        assert_eq!(serde_json::to_value(binary)?, value);
    }
    assert_eq!(
        serde_json::to_value(bincode::deserialize::<StreamChunk>(&legacy_bytes)?)?,
        json!({"Interrupted":{"queuedInputs":["queued"]}})
    );
    assert_eq!(&legacy_bytes[..4], &[9, 0, 0, 0]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_embedded_and_websocket_subscribers_receive_identical_typed_interruptions(
) -> TestResult {
    // @step Given embedded and WebSocket clients sharing a session broadcast
    let directory = codelet_test_helpers::setup_temp_directory()?;
    std::fs::create_dir_all(directory.path().join("spec"))?;
    std::fs::write(
        directory.path().join("spec/work-units.json"),
        r#"{"workUnits":{}}"#,
    )?;
    let manager = Arc::new(StubSessionManagerHandle::new());
    let service = Arc::new(SharedFspecService::with_session_manager(
        Arc::new(WorkUnitsWatcher::new(directory.path())?),
        manager.clone(),
    ));
    let embedded = EmbeddedTransport::new(tokio::runtime::Handle::current(), service.clone());
    let (address, _stats, task) = bind_and_serve("127.0.0.1:0", service).await?;
    let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}")).await?;
    let client = ws_client_connect(socket).await?;
    let mut embedded_rx = embedded.chunks_rx();
    let mut websocket_rx = client.chunks_rx();
    // @step When a typed interruption is published on the session channel
    let expected =
        json!({"InterruptedWithReason":{"queuedInputs":["pending"],"reason":"loopDetector"}});
    let chunk: StreamChunk = serde_json::from_value(expected.clone())?;
    manager
        .chunks_tx()
        .send((SessionId::new("cause-session"), chunk))?;
    // @step Then both subscribers receive the same reason and queued inputs
    let (_, local) = tokio::time::timeout(Duration::from_secs(3), embedded_rx.recv()).await??;
    let (_, remote) = tokio::time::timeout(Duration::from_secs(3), websocket_rx.recv()).await??;
    assert_eq!(serde_json::to_value(local)?, expected);
    assert_eq!(serde_json::to_value(remote)?, expected);
    task.abort();
    Ok(())
}
