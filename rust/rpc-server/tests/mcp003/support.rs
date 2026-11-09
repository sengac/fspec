//! MCP-003 test support: a daemon backed by a real SessionManager, the
//! fixture MCP server, and per-session MCP registry probes.

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect, ws_connect, FspecWsClient};
use codelet_rpc_types::{SessionId, SessionMcpServer, SessionMcpTransport};
use codelet_sessions::SessionManager;
use std::collections::BTreeMap;
use std::sync::Arc;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub type Boxed<T> = Result<T, Box<dyn std::error::Error>>;

pub const TOKEN: &str = "mcp003-s3cret";

/// A daemon over a real SessionManager in a fresh data dir and workspace.
pub struct Daemon {
    pub _data: tempfile::TempDir,
    pub _workspace: tempfile::TempDir,
    pub manager: Arc<SessionManager>,
    /// Read only by tests that reach the service in-process (MCP-003).
    pub _service: Arc<SharedFspecService>,
    pub url: String,
}

impl Daemon {
    pub async fn start(token: Option<&str>) -> Boxed<Self> {
        let data = tempfile::tempdir()?;
        let workspace = tempfile::tempdir()?;
        codelet_test_helpers::isolated_user_directory()?;
        codelet_common::set_data_directory(data.path().to_path_buf())?;
        codelet_core::persistence::reset_stores_for_tests();
        std::fs::create_dir_all(data.path().join("cache"))?;
        std::fs::write(
            data.path().join("cache/models.json"),
            include_str!("../../../sessions/tests/fixtures/prov129_codex_models.json"),
        )?;
        std::fs::create_dir_all(workspace.path().join("spec"))?;
        std::fs::write(
            workspace.path().join("spec/work-units.json"),
            r#"{"workUnits":{}}"#,
        )?;
        let manager = SessionManager::new();
        let handle: Arc<dyn SessionManagerHandle> = manager.clone();
        let watcher = Arc::new(WorkUnitsWatcher::new(workspace.path())?);
        let service = Arc::new(SharedFspecService::with_session_manager(watcher, handle));
        if let Some(token) = token {
            service.set_auth_token(token.to_string());
        }
        let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", Arc::clone(&service)).await?;
        Ok(Self {
            _data: data,
            _workspace: workspace,
            manager,
            _service: service,
            url: format!("ws://{addr}/"),
        })
    }

    pub async fn session(&self) -> Boxed<SessionId> {
        let id = self
            .manager
            .create_session(
                "codex/gpt-5.4",
                self._workspace.path().to_str().ok_or("path")?,
            )
            .await?;
        Ok(SessionId { value: id })
    }

    pub async fn client(&self, bearer: Option<&str>) -> Boxed<FspecWsClient> {
        let (ws, _response) = ws_connect(&self.url, bearer).await?;
        Ok(ws_client_connect(ws).await?)
    }
}

pub fn fixture() -> &'static str {
    env!("CARGO_BIN_EXE_fspec-mcp-fixture")
}

/// The fixture as stdio server `name`, launched with the literal `args` (MCP-005).
pub fn stdio(name: &str, args: &[&str]) -> SessionMcpServer {
    SessionMcpServer {
        name: name.to_string(),
        transport: SessionMcpTransport::Stdio {
            command: fixture().to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            env: BTreeMap::new(),
        },
    }
}

/// The names of the servers connected in a session's MCP registry, or None
/// when the session has no registry entry.
pub async fn connection_names(session: &SessionId) -> Boxed<Option<Vec<String>>> {
    let uuid = uuid::Uuid::parse_str(&session.value)?;
    let Some(connections) = codelet_tools::mcp::get_mcp_connections(uuid) else {
        return Ok(None);
    };
    let mut names: Vec<String> = connections.read().await.keys().cloned().collect();
    names.sort();
    Ok(Some(names))
}

/// The qualified MCP tool names a session's agent would be given.
pub async fn tool_names(session: &SessionId) -> Boxed<Vec<String>> {
    let uuid = uuid::Uuid::parse_str(&session.value)?;
    let connections = codelet_tools::mcp::get_mcp_connections(uuid).ok_or("no MCP registry")?;
    let map = connections.read().await;
    Ok(codelet_tools::mcp::gather_mcp_tools(&map)
        .into_iter()
        .map(|tool| tool.name)
        .collect())
}
