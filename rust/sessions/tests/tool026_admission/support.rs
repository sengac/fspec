use codelet_tools::execution_context::{current_invocation, InvocationReceipt};
use rig::{
    completion::ToolDefinition,
    tool::{server::ToolServer, server::ToolServerHandle, ToolDyn, ToolError},
    wasm_compat::WasmBoxedFuture,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type Calls = Arc<Mutex<Vec<(Value, Option<InvocationReceipt>)>>>;
pub type ProbeCallback = Arc<dyn Fn() -> std::result::Result<(), ToolError> + Send + Sync>;

pub struct Probe {
    pub calls: Calls,
    pub gate: Option<Arc<tokio::sync::Notify>>,
    pub callback: Option<ProbeCallback>,
}

impl ToolDyn for Probe {
    fn name(&self) -> String {
        "probe".into()
    }

    fn definition(&self, _prompt: String) -> WasmBoxedFuture<'_, ToolDefinition> {
        Box::pin(async {
            ToolDefinition {
                name: "probe".into(),
                description: "strict external execution boundary".into(),
                parameters: json!({"type":"object","properties":{
                    "payload":{"type":"object"},"target_work_unit_id":{"type":"string"},
                    "session_id":{"type":"string"},"project_root":{"type":"string"},
                    "fail":{"type":"boolean"}},"additionalProperties":false}),
            }
        })
    }

    fn call(&self, args: String) -> WasmBoxedFuture<'_, std::result::Result<String, ToolError>> {
        Box::pin(async move {
            let value: Value = serde_json::from_str(&args)?;
            if let Some(callback) = &self.callback {
                callback()?;
            }
            self.calls
                .lock()
                .map_err(|_| ToolError::ToolCallError("calls poisoned".into()))?
                .push((value.clone(), current_invocation()));
            if let Some(gate) = &self.gate {
                gate.notified().await;
            }
            if value.get("work_unit_id").is_some() {
                return Err(ToolError::ToolCallError(
                    "strict boundary received metadata".into(),
                ));
            }
            if value["fail"] == true {
                return Err(ToolError::ToolCallError("execution failed".into()));
            }
            Ok(value.to_string())
        })
    }
}

pub async fn server(
    scope: &str,
    calls: Calls,
    gate: Option<Arc<tokio::sync::Notify>>,
) -> Result<ToolServerHandle> {
    server_with_callback(scope, calls, gate, None).await
}

pub async fn server_with_callback(
    scope: &str,
    calls: Calls,
    gate: Option<Arc<tokio::sync::Notify>>,
    callback: Option<ProbeCallback>,
) -> Result<ToolServerHandle> {
    let actor = ToolServer::new().run();
    actor
        .add_tool(Probe {
            calls,
            gate,
            callback,
        })
        .await?;
    codelet_tools::tool_attribution::configure_tool_server(&actor, scope).await?;
    Ok(actor)
}

pub fn args(id: &str) -> String {
    json!({"work_unit_id":id,"payload":{"nested":[1,{"work_unit_id":"business"}]},"target_work_unit_id":"business-target"}).to_string()
}
