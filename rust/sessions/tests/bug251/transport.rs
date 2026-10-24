use super::support::TestResult;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{IncomingMessageImage, IncomingMessageInput, SessionId};
use codelet_sessions::SessionManager;
use std::{path::Path, sync::Arc};

pub async fn send(
    directory: &Path,
    manager: Arc<SessionManager>,
    id: SessionId,
    websocket: bool,
    images: Vec<IncomingMessageImage>,
) -> TestResult {
    std::fs::create_dir_all(directory.join("spec"))?;
    std::fs::write(directory.join("spec/work-units.json"), "{\"workUnits\":{}}")?;
    let watcher = Arc::new(WorkUnitsWatcher::new(directory)?);
    let service = Arc::new(
        SharedFspecService::with_session_manager(watcher, manager).with_cwd(directory.into()),
    );
    let input = IncomingMessageInput {
        source_session_id: "bridge".into(),
        role_name: "user".into(),
        message: "原文 α".into(),
        images: Some(images),
    };
    if websocket {
        let (address, _, server) =
            codelet_rpc_server::bind_and_serve("127.0.0.1:0", service).await?;
        let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/")).await?;
        let client = codelet_rpc_server::ws_client_connect(socket).await?;
        let result = client
            .rpc
            .receive_incoming_message(tarpc::context::current(), id, input)
            .await?;
        server.abort();
        result?;
    } else {
        codelet_rpc_embedded::EmbeddedTransport::new(tokio::runtime::Handle::current(), service)
            .client()
            .receive_incoming_message(tarpc::context::current(), id, input)
            .await??;
    }
    Ok(())
}
