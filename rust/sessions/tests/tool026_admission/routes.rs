use super::{
    fixture::Fixture,
    support::{args, server, Calls, Result},
};
use codelet_tools::execution_context::InvocationReceipt;
use rig::tool::server::{ToolServerError, ToolServerHandle};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub async fn strict_delegation(fixture: &Fixture) -> Result {
    super::delegation::typed(fixture).await?;
    super::delegation::mcp(fixture).await?;
    super::delegation::mapped(fixture).await?;
    Ok(())
}

pub struct PendingCall {
    pub gate: Arc<tokio::sync::Notify>,
    pub task: tokio::task::JoinHandle<std::result::Result<String, ToolServerError>>,
    pub receipt: InvocationReceipt,
}

pub async fn admit_ordered_calls(fixture: &Fixture) -> Result<PendingCall> {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let gate = Arc::new(tokio::sync::Notify::new());
    let slow = server(&fixture.first.value, calls.clone(), Some(gate.clone())).await?;
    let fast = server(&fixture.first.value, calls.clone(), None).await?;
    let mut input: Value = serde_json::from_str(&args("AUTH-001"))?;
    input["fail"] = json!(true);
    let task = tokio::spawn(async move { slow.call_tool("probe", &input.to_string()).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while calls.lock().map(|calls| calls.is_empty()).unwrap_or(false) {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    let receipt = calls.lock().map_err(|_| "calls")?[0]
        .1
        .clone()
        .ok_or("missing receipt")?;
    fast.call_tool("probe", &args("AUTH-002")).await?;
    fast.call_tool("probe", &args("NONE")).await?;
    Ok(PendingCall {
        gate,
        task,
        receipt,
    })
}

pub async fn hot_tools(fixture: &Fixture, calls: Calls) -> Result<ToolServerHandle> {
    let schema_only = rig::tool::server::ToolServer::new().run();
    schema_only
        .configure_middleware(Arc::new(
            codelet_tools::tool_attribution::SchemaMiddleware::new(&fixture.first.value),
        ))
        .await?;
    assert!(
        codelet_tools::tool_attribution::configure_tool_server(&schema_only, &fixture.first.value)
            .await
            .is_err(),
        "schema-only policy silently prevented runtime installation"
    );
    let actor = server(&fixture.first.value, calls, None).await?;
    actor
        .add_tool(codelet_tools::DoneTool::new(uuid::Uuid::parse_str(
            &fixture.first.value,
        )?))
        .await?;
    Ok(actor)
}
