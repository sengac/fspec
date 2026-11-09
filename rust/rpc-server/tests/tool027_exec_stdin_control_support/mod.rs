//! TOOL-027 test support: a daemon over a real SessionManager, exec
//! sessions started in-process (same global unified_exec store as the
//! daemon), and polling helpers.

use std::path::Path;
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_rpc::SharedFspecService;
use codelet_rpc_server::{bind_and_serve, ws_client_connect, ws_connect, FspecWsClient};
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use codelet_tools::unified_exec::{UnifiedExecArgs, UnifiedExecResult, UnifiedExecTool};
use rig::tool::Tool;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub type Boxed<T> = Result<T, Box<dyn std::error::Error>>;

/// Prints "got-int" and exits 3 on SIGINT, otherwise waits forever.
pub const TRAP_SCRIPT: &str = "trap 'echo got-int; exit 3' INT; while true; do sleep 0.1; done";

/// A daemon over a real SessionManager in a fresh data dir and workspace.
pub struct Daemon {
    pub _data: tempfile::TempDir,
    pub workspace: tempfile::TempDir,
    pub manager: Arc<SessionManager>,
    pub url: String,
}

impl Daemon {
    pub async fn start() -> Boxed<Self> {
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
        let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service).await?;
        Ok(Self {
            _data: data,
            workspace,
            manager,
            url: format!("ws://{addr}/"),
        })
    }

    pub async fn session(&self) -> Boxed<SessionId> {
        let workspace = self
            .workspace
            .path()
            .to_str()
            .ok_or("workspace path is not UTF-8")?;
        let id = self
            .manager
            .create_session("codex/gpt-5.4", workspace)
            .await?;
        Ok(SessionId { value: id })
    }

    pub async fn client(&self) -> Boxed<FspecWsClient> {
        let (ws, _response) = ws_connect(&self.url, None).await?;
        Ok(ws_client_connect(ws).await?)
    }
}

/// An exec session started for an agent session.
pub struct Exec {
    pub tool: UnifiedExecTool,
    pub id: String,
}

/// Start a still-running exec session for `agent` (`tty` picks PTY mode).
pub async fn start_exec(agent: &SessionId, command: &str, tty: bool) -> Boxed<Exec> {
    // BUG-447: this test drives unified_exec directly, without the session
    // manager's isolation callback for the agent session.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "tests drive unified_exec without a session manager callback",
    );
    let tool = UnifiedExecTool::new(uuid::Uuid::parse_str(&agent.value)?);
    let result: UnifiedExecResult = tool
        .call(UnifiedExecArgs(serde_json::json!({
            "action": "run",
            "command": command,
            "tty": tty,
            "yield_time_ms": 300
        })))
        .await?;
    let id = result
        .session_id
        .ok_or("the command exited instead of staying live")?;
    Ok(Exec { tool, id })
}

/// What a poll loop saw once the exec session exited.
pub struct Exited {
    pub output: String,
    pub exit_code: Option<i32>,
}

/// Poll the exec session until it reports an exit (TEST-042: under the
/// deadlock guard, not a 10 s limit).
pub async fn wait_for_exit(exec: &Exec) -> Boxed<Exited> {
    let mut output = String::new();
    codelet_test_helpers::deadlock_guard("the exec session to exit", async {
        loop {
            let result: UnifiedExecResult = exec
                .tool
                .call(UnifiedExecArgs(serde_json::json!({
                    "action": "poll",
                    "session_id": exec.id,
                    "yield_time_ms": 200
                })))
                .await?;
            if let Some(chunk) = result.output {
                output.push_str(&chunk);
            }
            if result.session_id.is_none() {
                return Ok::<_, Box<dyn std::error::Error>>(Exited {
                    output,
                    exit_code: result.exit_code,
                });
            }
        }
    })
    .await?
}

/// FspecService method names in declaration (variant-index) order.
pub fn declared_methods() -> Boxed<Vec<String>> {
    let rpc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rpc/src/service_declaration");
    let mut methods = Vec::new();
    for fragment in [
        "workspace.rs",
        "providers_sessions.rs",
        "supervision_credentials.rs",
        "session_mcp.rs",
    ] {
        for line in std::fs::read_to_string(rpc.join(fragment))?.lines() {
            if let Some(rest) = line.trim_start().strip_prefix("async fn ") {
                let name = rest.split(['(', '<']).next().ok_or("empty method line")?;
                methods.push(name.to_string());
            }
        }
    }
    Ok(methods)
}
