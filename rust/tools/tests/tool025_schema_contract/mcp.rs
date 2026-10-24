use super::{assert_contract, TestResult};
use codelet_tools::{
    mcp::{
        cleanup_mcp_session, gather_mcp_tool_wrappers, init_mcp_session,
        set_mcp_tool_server_handle, ConnectMcpTool, McpConnectArgs,
    },
    tool_attribution::configure_tool_server,
};
use rig::tool::{server::ToolServer, Tool};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use uuid::Uuid;

struct RejectRemote;

impl rig::tool::ToolMiddleware for RejectRemote {
    fn scope_id(&self) -> &str {
        "reject-remote"
    }

    fn decorate_definition(
        &self,
        definition: rig::completion::ToolDefinition,
    ) -> Result<rig::completion::ToolDefinition, rig::tool::ToolError> {
        if definition.name == "mcp__schema__rejected" {
            return Err(rig::tool::ToolError::ToolCallError(
                "MCP fixture registration rejected".into(),
            ));
        }
        codelet_tools::tool_attribution::decorate_definition(definition)
            .map_err(|error| rig::tool::ToolError::ToolCallError(Box::new(error)))
    }
}

struct Session(Uuid);

impl Drop for Session {
    fn drop(&mut self) {
        cleanup_mcp_session(self.0);
    }
}

struct Loopback {
    url: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Loopback {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve_request(mut stream: TcpStream, tools: Value) -> std::io::Result<()> {
    let mut bytes = Vec::new();
    let headers_end = loop {
        let mut chunk = [0; 1024];
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..headers_end]);
    let length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let is_post = headers.starts_with("POST ");
    while bytes.len() < headers_end + length {
        let mut chunk = [0; 1024];
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    let request: Value = serde_json::from_slice(&bytes[headers_end..]).unwrap_or(Value::Null);
    let (status, body) = if !is_post {
        ("405 Method Not Allowed", String::new())
    } else if request.get("id").is_none() {
        ("202 Accepted", String::new())
    } else {
        let result = match request["method"].as_str() {
            Some("initialize") => json!({
                "protocolVersion":"2025-11-25", "capabilities":{"tools":{}},
                "serverInfo":{"name":"schema-loopback","version":"1"}
            }),
            Some("tools/list") => json!({"tools":tools}),
            _ => json!({}),
        };
        (
            "200 OK",
            json!({"jsonrpc":"2.0","id":request["id"],"result":result}).to_string(),
        )
    };
    stream.write_all(format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ).as_bytes()).await?;
    Ok(())
}

async fn loopback(tools: Value) -> Result<Loopback, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/mcp", listener.local_addr()?);
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let tools = tools.clone();
            tokio::spawn(async move {
                let _ = serve_request(stream, tools).await;
            });
        }
    });
    Ok(Loopback { url, task })
}

fn connect_args(url: &str) -> Result<McpConnectArgs, serde_json::Error> {
    serde_json::from_value(json!({
        "action":"connect", "name":"schema", "transport":"http", "url":url,
        "timeout":2
    }))
}

pub async fn exercise_hot_registration() -> TestResult {
    let business = json!({
        "type":"object", "additionalProperties":false,
        "$defs":{"target":{"type":"string"}},
        "properties":{"target_work_unit_id":{"$ref":"#/$defs/target"}},
        "required":["target_work_unit_id"]
    });
    let endpoint = loopback(json!([{
        "name":"business", "description":"Remote business target", "inputSchema":business
    }]))
    .await?;
    let session = Session(Uuid::new_v4());
    let (_injections, _connections) = init_mcp_session(session.0);
    let actor = ToolServer::new().tool(ConnectMcpTool::new(session.0)).run();
    configure_tool_server(&actor, session.0.to_string()).await?;
    set_mcp_tool_server_handle(session.0, actor.clone());
    let project = codelet_test_helpers::setup_temp_directory()?;
    let _admission =
        codelet_tools::tool_admission::register_auxiliary(session.0, project.path().to_path_buf())?;
    let mut arguments = serde_json::to_value(connect_args(&endpoint.url)?)?;
    arguments["work_unit_id"] = json!("NONE");
    let arguments = arguments.to_string();
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        actor.call_tool("ConnectMCP", &arguments),
    )
    .await??;
    assert!(
        response.contains("Connected:"),
        "loopback handshake failed: {response}"
    );
    let definitions = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let definitions = actor.get_tool_defs(None).await?;
            if definitions.len() == 2 {
                return Ok::<_, rig::tool::server::ToolServerError>(definitions);
            }
            tokio::task::yield_now().await;
        }
    })
    .await??;
    let remote = definitions
        .iter()
        .find(|definition| definition.name == "mcp__schema__business")
        .ok_or("hot MCP tool missing")?;
    assert_contract(remote);
    assert_eq!(
        remote.parameters["properties"]["target_work_unit_id"],
        business["properties"]["target_work_unit_id"]
    );
    assert_eq!(remote.parameters["$defs"], business["$defs"]);
    assert_eq!(remote.parameters["additionalProperties"], false);
    let rebuilt = ToolServer::new().run();
    configure_tool_server(&rebuilt, session.0.to_string()).await?;
    for wrapper in gather_mcp_tool_wrappers(session.0) {
        rebuilt.add_tool(wrapper).await?;
    }
    let rebuilt_definitions = rebuilt.get_tool_defs(None).await?;
    assert_eq!(rebuilt_definitions, vec![remote.clone()]);
    Ok(())
}

pub async fn exercise_collision() -> TestResult {
    let business = json!({"type":"object","properties":{"work_unit_id":{"type":"string"}}});
    let endpoint = loopback(json!([{
        "name":"collision", "description":"Reserved business metadata", "inputSchema":business
    }]))
    .await?;
    let session = Session(Uuid::new_v4());
    let (_injections, _connections) = init_mcp_session(session.0);
    let actor = ToolServer::new().tool(ConnectMcpTool::new(session.0)).run();
    configure_tool_server(&actor, session.0.to_string()).await?;
    set_mcp_tool_server_handle(session.0, actor.clone());
    let result = ConnectMcpTool::new(session.0)
        .call(connect_args(&endpoint.url)?)
        .await;
    let message = match result {
        Ok(output) => output,
        Err(error) => error.to_string(),
    };
    assert!(
        message.contains("work_unit_id")
            && message.contains("collision")
            && !message.contains("Connected:"),
        "ConnectMCP must expose schema registration failure, not connection success: {message}"
    );
    assert_eq!(actor.get_tool_defs(None).await?.len(), 1);
    assert!(
        gather_mcp_tool_wrappers(session.0).is_empty(),
        "rebuild must not resurrect rejected tools"
    );
    exercise_actor_rejection().await?;
    Ok(())
}

pub async fn exercise_actor_rejection() -> TestResult {
    let endpoint = loopback(json!([
        {"name":"accepted", "inputSchema":{"type":"object","properties":{}}},
        {"name":"rejected", "inputSchema":{"type":"object","properties":{}}}
    ]))
    .await?;
    let session = Session(Uuid::new_v4());
    let (mut injections, _connections) = init_mcp_session(session.0);
    let actor = ToolServer::new().tool(ConnectMcpTool::new(session.0)).run();
    actor
        .configure_middleware(std::sync::Arc::new(RejectRemote))
        .await?;
    set_mcp_tool_server_handle(session.0, actor.clone());
    let arguments = serde_json::to_string(&connect_args(&endpoint.url)?)?;
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        actor.call_tool("ConnectMCP", &arguments),
    )
    .await??;
    let notification = tokio::time::timeout(std::time::Duration::from_secs(2), injections.recv())
        .await?
        .ok_or("registration failure notification missing")?;
    let codelet_tools::mcp::McpInjection::Notification(message) = notification else {
        return Err("expected registration failure notification".into());
    };
    assert!(message.contains("schema") && message.contains("MCP fixture registration rejected"));
    assert_eq!(actor.get_tool_defs(None).await?.len(), 1);
    assert!(gather_mcp_tool_wrappers(session.0).is_empty());
    Ok(())
}
