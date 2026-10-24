use crate::{fixture::Fixture, support::Result};
use codelet_providers::custom::{
    ProviderConfig, RhaiToolDef, RhaiToolFacadeAdapter, RhaiToolWrapper, ScriptLoader,
};
use codelet_tools::{
    facade::{FileToolFacadeWrapper, GeminiReadFileFacade},
    mcp::{cleanup_mcp_session, gather_mcp_tool_wrappers, init_mcp_session, McpConnectArgs},
    tool_attribution::configure_tool_server,
    ConnectMcpTool,
};
use rig::{
    completion::ToolDefinition,
    tool::{server::ToolServer, Tool},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Business {
    payload: Value,
    target_work_unit_id: String,
}

struct Typed(Arc<Mutex<Vec<Value>>>);

impl Tool for Typed {
    const NAME: &'static str = "strict_typed";
    type Args = Business;
    type Error = codelet_tools::ToolError;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Strict typed business arguments".into(),
            parameters: schema(),
        }
    }

    async fn call(&self, args: Business) -> std::result::Result<Value, Self::Error> {
        let audit = codelet_tools::tool_admission::current_audit_arguments().ok_or_else(|| {
            Self::Error::Execution {
                tool: Self::NAME,
                message: "audit arguments missing".into(),
            }
        })?;
        assert_eq!(
            serde_json::from_str::<Value>(&audit).map_err(|error| Self::Error::Execution {
                tool: Self::NAME,
                message: error.to_string()
            })?,
            attributed()
        );
        let value = serde_json::to_value(args).map_err(|error| Self::Error::Execution {
            tool: Self::NAME,
            message: error.to_string(),
        })?;
        self.0
            .lock()
            .map_err(|_| Self::Error::Execution {
                tool: Self::NAME,
                message: "typed calls lock poisoned".into(),
            })?
            .push(value.clone());
        Ok(value)
    }
}

fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,
        "properties":{"payload":{"type":"object"},"target_work_unit_id":{"type":"string"}},
        "required":["payload","target_work_unit_id"]})
}

fn business() -> Value {
    json!({"payload":{"nested":[1,{"work_unit_id":"business"}]},"target_work_unit_id":"business-target"})
}

fn attributed() -> Value {
    let mut arguments = business();
    arguments["work_unit_id"] = json!("AUTH-003");
    arguments
}

pub async fn typed(fixture: &Fixture) -> Result {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let actor = ToolServer::new().tool(Typed(calls.clone())).run();
    configure_tool_server(&actor, &fixture.first.value).await?;
    assert!(actor
        .call_tool("strict_typed", &business().to_string())
        .await
        .is_err());
    assert!(calls.lock().map_err(|_| "typed calls")?.is_empty());
    let original = attributed().to_string();
    let output: Value = serde_json::from_str(&actor.call_tool("strict_typed", &original).await?)?;
    assert_eq!(output, business());
    assert_eq!(*calls.lock().map_err(|_| "typed calls")?, vec![business()]);
    let mut extra = attributed();
    extra["unexpected_business_field"] = json!(true);
    assert!(actor
        .call_tool("strict_typed", &extra.to_string())
        .await
        .is_err());
    assert_eq!(calls.lock().map_err(|_| "typed calls")?.len(), 1);
    Ok(())
}

struct McpCleanup(Uuid);

impl Drop for McpCleanup {
    fn drop(&mut self) {
        cleanup_mcp_session(self.0);
    }
}

pub async fn mcp(fixture: &Fixture) -> Result {
    let endpoint = MockServer::start().await;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let observed = calls.clone();
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(405))
        .mount(&endpoint)
        .await;
    Mock::given(method("POST")).respond_with(move |request: &wiremock::Request| {
        let request: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
        if request.get("id").is_none() {
            return ResponseTemplate::new(202);
        }
        let result = match request["method"].as_str() {
            Some("initialize") => json!({"protocolVersion":"2025-11-25",
                "capabilities":{"tools":{}},"serverInfo":{"name":"delegation","version":"1"}}),
            Some("tools/list") => json!({"tools":[{"name":"strict","inputSchema":schema()}]}),
            Some("tools/call") => {
                let arguments = request["params"]["arguments"].clone();
                let accepted = serde_json::from_value::<Business>(arguments.clone()).is_ok();
                match observed.lock() {
                    Ok(mut recorded) => recorded.push(arguments.clone()),
                    Err(_) => return ResponseTemplate::new(500),
                }
                json!({"content":[{"type":"text","text":arguments.to_string()}],"isError":!accepted})
            }
            _ => json!({}),
        };
        ResponseTemplate::new(200)
            .set_body_json(json!({"jsonrpc":"2.0","id":request["id"],"result":result}))
    }).mount(&endpoint).await;
    let session = Uuid::parse_str(&fixture.first.value)?;
    let _cleanup = McpCleanup(session);
    let (_injections, _connections) = init_mcp_session(session);
    let connect: McpConnectArgs = serde_json::from_value(json!({"action":"connect",
        "name":"delegation","transport":"http","url":endpoint.uri(),"timeout":2}))?;
    let connected = ConnectMcpTool::new(session).call(connect).await?;
    assert!(connected.contains("Connected:"), "{connected}");
    let actor = ToolServer::new().run();
    configure_tool_server(&actor, &fixture.first.value).await?;
    for wrapper in gather_mcp_tool_wrappers(session) {
        actor.add_tool(wrapper).await?;
    }
    assert!(actor
        .call_tool("mcp__delegation__strict", &business().to_string())
        .await
        .is_err());
    assert!(calls.lock().map_err(|_| "MCP calls")?.is_empty());
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        actor.call_tool("mcp__delegation__strict", &attributed().to_string()),
    )
    .await??;
    assert!(output.contains("business-target"), "{output}");
    assert_eq!(*calls.lock().map_err(|_| "MCP calls")?, vec![business()]);
    Ok(())
}

pub async fn mapped(fixture: &Fixture) -> Result {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let script = directory.path().join("delegation.rhai");
    std::fs::write(&script, SCRIPT)?;
    let target = fixture.project.path().join("delegation.txt");
    std::fs::write(&target, "TOOL026 mapping reached the real file executor")?;
    let configuration: ProviderConfig = serde_json::from_value(json!({
        "name":"tool026-mapping","display_name":"Delegation","base_url":"http://127.0.0.1:1",
        "script":script,"models":{"default":{"id":"unused"}}
    }))?;
    let adapter = RhaiToolFacadeAdapter::new(
        Arc::new(RhaiToolDef {
            name: "mapped_read".into(),
            description: "Read mapped path".into(),
            parameters: schema(),
            maps_to: "file:read".into(),
        }),
        Arc::new(configuration),
        Arc::new(ScriptLoader::with_default_engine()),
    )?;
    let session = Uuid::parse_str(&fixture.first.value)?;
    let actor = ToolServer::new()
        .tool(RhaiToolWrapper::new(adapter, session))
        .tool(FileToolFacadeWrapper::new(
            Arc::new(GeminiReadFileFacade),
            session,
        ))
        .run();
    configure_tool_server(&actor, &fixture.first.value).await?;
    let mut arguments = attributed();
    arguments["payload"]["file"] = json!(target);
    let output = actor
        .call_tool("mapped_read", &arguments.to_string())
        .await?;
    assert!(
        output.contains("TOOL026 mapping reached the real file executor"),
        "{output}"
    );
    arguments
        .as_object_mut()
        .ok_or("arguments")?
        .remove("work_unit_id");
    assert!(actor
        .call_tool("mapped_read", &arguments.to_string())
        .await
        .is_err());
    let mut facade_arguments = json!({"file_path":target});
    assert!(actor
        .call_tool("read_file", &facade_arguments.to_string())
        .await
        .is_err());
    facade_arguments["work_unit_id"] = json!("AUTH-003");
    let output = actor
        .call_tool("read_file", &facade_arguments.to_string())
        .await?;
    assert!(
        output.contains("TOOL026 mapping reached the real file executor"),
        "{output}"
    );
    Ok(())
}

const SCRIPT: &str = r#"
fn build_request(request) { #{} }
fn build_stream_request(request) { #{} }
fn build_headers(config) { #{} }
fn build_url(config) { config.base_url }
fn parse_response(raw) { #{} }
fn parse_stream_chunk(config, data) { #{} }
fn map_error(status, body) { #{} }
fn map_tool_params(config, tool_name, maps_to, params) {
    if params.contains("work_unit_id") || params.len() != 2 ||
       params.target_work_unit_id != "business-target" ||
       params.payload.nested[1].work_unit_id != "business" {
        return #{ file_path: "/tool026-mapping-received-wrong-business-arguments" };
    }
    #{ file_path: params.payload.file }
}
"#;
