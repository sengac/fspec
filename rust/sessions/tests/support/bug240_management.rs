use crate::fixture::{Fixture, Result};
use codelet_rpc_types::SessionId;
use codelet_tools::agent_manager::{set_agent_manager_handler, AgentManagerTool};
use codelet_tools::tool_attribution::configure_tool_server;
use rig::completion::ToolDefinition;
use rig::tool::server::{ToolServer, ToolServerHandle};
use rig::tool::{Tool, ToolDyn, ToolError};
use rig::wasm_compat::WasmBoxedFuture;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub type Calls = Arc<Mutex<Vec<Value>>>;

pub struct HandlerGuard(Uuid);

impl Drop for HandlerGuard {
    fn drop(&mut self) {
        set_agent_manager_handler(self.0, None);
    }
}

struct StrictManager {
    tool: AgentManagerTool,
    calls: Calls,
}

impl ToolDyn for StrictManager {
    fn name(&self) -> String {
        "AgentManager".into()
    }

    fn definition(&self, prompt: String) -> WasmBoxedFuture<'_, ToolDefinition> {
        Box::pin(Tool::definition(&self.tool, prompt))
    }

    fn call(&self, args: String) -> WasmBoxedFuture<'_, std::result::Result<String, ToolError>> {
        Box::pin(async move {
            let business: Value = serde_json::from_str(&args)?;
            assert!(business.get("work_unit_id").is_none());
            let audit =
                codelet_tools::tool_admission::current_audit_arguments().ok_or_else(|| {
                    ToolError::ToolCallError("management call was not admitted".into())
                })?;
            let original: Value = serde_json::from_str(&audit)?;
            let mut expected = original.clone();
            expected
                .as_object_mut()
                .ok_or_else(|| ToolError::ToolCallError("audit object".into()))?
                .remove("work_unit_id");
            assert!(original.get("work_unit_id").is_some());
            assert_eq!(business, expected);
            self.calls
                .lock()
                .map_err(|_| ToolError::ToolCallError("recorded calls lock".into()))?
                .push(business);
            ToolDyn::call(&self.tool, args).await
        })
    }
}

pub async fn server(
    fixture: &Fixture,
    caller: &SessionId,
    calls: Calls,
) -> Result<(ToolServerHandle, HandlerGuard)> {
    let caller_id = Uuid::parse_str(&caller.value)?;
    let handler = codelet_agent_loop::agent_manager_handler::create_handler(
        Some(fixture.manager.clone()),
        fixture.project.path().to_string_lossy().into_owned(),
        Some("anthropic/claude-sonnet-4".into()),
        None,
        None,
    );
    set_agent_manager_handler(caller_id, Some(handler));
    let guard = HandlerGuard(caller_id);
    let server = ToolServer::new().run();
    server
        .add_tool(StrictManager {
            tool: AgentManagerTool::new(caller_id),
            calls,
        })
        .await?;
    configure_tool_server(&server, &caller.value).await?;
    Ok((server, guard))
}
