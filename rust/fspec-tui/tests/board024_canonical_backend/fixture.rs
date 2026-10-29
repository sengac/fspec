use std::path::PathBuf;
use std::sync::Arc;

use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::WebSocketFspecBackend;
use codelet_rpc::{
    mcp_dispatch::{self, InvokeError},
    FspecServiceClient, SharedFspecService,
};
use codelet_rpc_embedded::EmbeddedTransport;
use codelet_rpc_types::WorkUnitInfo;
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::task::JoinHandle;

use super::TestResult;

enum Client {
    Embedded(FspecServiceClient),
    WebSocket(WebSocketFspecBackend),
}

pub struct Fixture {
    pub root: TempDir,
    pub snapshot: Vec<WorkUnitInfo>,
    client: Client,
    embedded: Option<EmbeddedTransport>,
    listener: Option<JoinHandle<()>>,
}

impl Fixture {
    pub async fn new(
        websocket: bool,
        attach_workspace: bool,
        record: &Value,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let root = codelet_test_helpers::setup_temp_directory()?;
        std::fs::create_dir_all(root.path().join("spec"))?;
        std::fs::write(
            root.path().join("spec/work-units.json"),
            serde_json::to_vec(&json!({"workUnits": {"AUTH-001": record}}))?,
        )?;
        let watcher = Arc::new(WorkUnitsWatcher::new(root.path())?);
        let snapshot = watcher.snapshot();
        let service = SharedFspecService::new(watcher);
        let service = Arc::new(if attach_workspace {
            service.with_cwd(root.path().to_path_buf())
        } else {
            service
        });
        let (client, embedded, listener) = if websocket {
            let (address, _stats, listener) =
                codelet_rpc_server::bind_and_serve("127.0.0.1:0", service).await?;
            let url = url::Url::parse(&format!("ws://{address}/"))?;
            (
                Client::WebSocket(WebSocketFspecBackend::connect(url).await?),
                None,
                Some(listener),
            )
        } else {
            let transport = EmbeddedTransport::new(tokio::runtime::Handle::current(), service);
            (Client::Embedded(transport.client()), Some(transport), None)
        };
        Ok(Self {
            root,
            snapshot,
            client,
            embedded,
            listener,
        })
    }

    pub fn store(&self) -> PathBuf {
        self.root.path().join("spec/work-units.json")
    }

    pub fn save(&self, record: &Value) -> TestResult {
        std::fs::write(
            self.store(),
            serde_json::to_vec(&json!({"workUnits": {"AUTH-001": record}}))?,
        )?;
        Ok(())
    }

    pub fn transport_name(&self) -> &'static str {
        if self.embedded.is_some() {
            "embedded"
        } else {
            "websocket"
        }
    }

    pub async fn details(&self, id: &str) -> Result<Value, InvokeError> {
        let args = json!({"id": id});
        match &self.client {
            Client::Embedded(client) => {
                mcp_dispatch::invoke(client, "get_work_unit_details", args).await
            }
            Client::WebSocket(client) => client.invoke_mcp_rpc("get_work_unit_details", args).await,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(listener) = self.listener.take() {
            listener.abort();
        }
    }
}

pub fn envelope(value: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let text = value
        .as_str()
        .ok_or("canonical RPC must return a JSON String, not a Value on the wire")?;
    Ok(serde_json::from_str(text)?)
}
