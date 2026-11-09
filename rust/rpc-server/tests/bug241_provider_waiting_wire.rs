//! Supporting transport parity for spec/features/transient-provider-waiting.feature.

use codelet_core::{
    session_manager_handle::StubSessionManagerHandle, work_units::WorkUnitsWatcher,
};
use codelet_rpc::SharedFspecService;
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_server::{bind_and_serve, ws_client_connect};
use codelet_rpc_types::{SessionId, StreamChunk};
use serde_json::json;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn waiting_set_and_clear_reach_embedded_and_websocket_clients(
) -> Result<(), Box<dyn std::error::Error>> {
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
    let mut local = embedded.chunks_rx();
    let mut remote = client.chunks_rx();
    for message in [Some("Giving this request a little extra thought"), None] {
        let expected = json!({"ProviderWaiting":{"message":message}});
        manager.chunks_tx().send((
            SessionId::new("waiting-session"),
            serde_json::from_value(expected.clone())?,
        ))?;
        let (local_id, local_chunk) =
            codelet_test_helpers::deadlock_guard("local.recv()", local.recv()).await??;
        let (remote_id, remote_chunk) =
            codelet_test_helpers::deadlock_guard("remote.recv()", remote.recv()).await??;
        assert_eq!(local_id, remote_id);
        assert_eq!(serde_json::to_value(local_chunk)?, expected);
        assert_eq!(serde_json::to_value(remote_chunk)?, expected);
    }
    task.abort();
    assert_eq!(
        &bincode::serialize(&StreamChunk::interrupted(Vec::new()))?[..4],
        &[9, 0, 0, 0]
    );
    Ok(())
}
