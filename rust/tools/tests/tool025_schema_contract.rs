//! Feature: spec/features/mandatory-tool-work-unit-schema.feature

use codelet_tools::tool_attribution::configure_tool_server;
use codelet_tools::{BashTool, DoneTool, ReadTool, RequestUserInputTool};
use rig::completion::ToolDefinition;
use rig::tool::{server::ToolServer, ToolDyn, ToolError};
use rig::wasm_compat::WasmBoxedFuture;
use serde_json::{json, Value};
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "tool025_schema_contract/lifecycle.rs"]
mod lifecycle;
#[path = "tool025_schema_contract/mcp.rs"]
mod mcp;
#[path = "tool025_schema_contract/preservation.rs"]
mod preservation;

struct Remote(Value);

impl ToolDyn for Remote {
    fn name(&self) -> String {
        "remote".into()
    }

    fn definition<'a>(&'a self, _prompt: String) -> WasmBoxedFuture<'a, ToolDefinition> {
        Box::pin(async move {
            ToolDefinition {
                name: self.name(),
                description: "Strict remote business tool".into(),
                parameters: self.0.clone(),
            }
        })
    }

    fn call<'a>(&'a self, args: String) -> WasmBoxedFuture<'a, Result<String, ToolError>> {
        Box::pin(async move { Ok(args) })
    }
}

async fn definitions(
    server: ToolServer,
) -> Result<Vec<ToolDefinition>, rig::tool::server::ToolServerError> {
    let handle = server.run();
    configure_tool_server(&handle, "schema-fixture").await?;
    handle.get_tool_defs(None).await
}

fn assert_contract(definition: &ToolDefinition) {
    let schema = &definition.parameters;
    assert!(
        schema["required"]
            .as_array()
            .is_some_and(|required| required.contains(&json!("work_unit_id"))),
        "{} must require harness metadata: {schema}",
        definition.name
    );
    let attribution = &schema["properties"]["work_unit_id"];
    assert_eq!(attribution["type"], "string");
    assert_eq!(attribution["minLength"], 1);
    assert_ne!(attribution["nullable"], true);
}

#[tokio::test]
async fn scenario_builtin_and_completion_tools_advertise_the_same_required_attribution(
) -> TestResult {
    // @step Given a normal agent toolset including file shell input and completion tools
    let session = Uuid::new_v4();
    let server = ToolServer::new()
        .tool(ReadTool::new(session))
        .tool(BashTool::new(session))
        .tool(RequestUserInputTool::new(session))
        .tool(DoneTool::new(session));
    // @step When their model-visible definitions are requested
    let definitions = definitions(server).await?;
    assert_eq!(definitions.len(), 4);
    // @step Then every definition requires top-level "work_unit_id" as a non-null string
    for definition in &definitions {
        assert_contract(definition);
    }
    // @step And the original business arguments remain intact
    let read = definitions
        .iter()
        .find(|definition| definition.name == "Read")
        .ok_or("Read definition missing")?;
    assert_eq!(read.parameters["properties"]["file_path"]["type"], "string");
    // @step And descriptions explain that "NONE" clears activity rather than retaining it
    for definition in definitions {
        let description = definition.parameters["properties"]["work_unit_id"]["description"]
            .as_str()
            .ok_or("attribution instructions missing")?;
        assert!(description.contains("NONE") && description.contains("clear"));
    }
    Ok(())
}

#[tokio::test]
async fn scenario_strict_remote_tool_schemas_preserve_business_work_unit_arguments() -> TestResult {
    // @step Given a remote tool with strict arguments and its own "target_work_unit_id" property
    let original = json!({
        "type":"object", "additionalProperties":false,
        "$defs":{"target":{"type":"string", "enum":["BUSINESS-001"]}},
        "properties":{"target_work_unit_id":{"$ref":"#/$defs/target"},
            "nested":{"oneOf":[{"type":"string"},{"type":"null"}]}},
        "required":["target_work_unit_id"]
    });
    let handle = ToolServer::new().run();
    handle.add_tool(Remote(original.clone())).await?;
    // @step When its definition is decorated for harness attribution
    configure_tool_server(&handle, "strict-remote").await?;
    let first = handle.get_tool_defs(None).await?;
    // @step Then its business work-unit argument retains its original meaning
    assert_contract(&first[0]);
    assert_eq!(
        first[0].parameters["properties"]["target_work_unit_id"],
        original["properties"]["target_work_unit_id"]
    );
    // @step And required lists references and additional-property restrictions are preserved
    assert_eq!(first[0].parameters["$defs"], original["$defs"]);
    assert_eq!(
        first[0].parameters["properties"]["nested"],
        original["properties"]["nested"]
    );
    assert_eq!(first[0].parameters["additionalProperties"], false);
    assert_eq!(
        first[0].parameters["required"],
        json!(["target_work_unit_id", "work_unit_id"])
    );
    // @step And repeated decoration does not add duplicate requirements
    assert_eq!(first, handle.get_tool_defs(None).await?);
    preservation::assert_preservation()?;
    preservation::assert_unsupported_shapes_fail_closed();
    Ok(())
}

#[tokio::test]
async fn scenario_a_reserved_metadata_collision_fails_explicitly() -> TestResult {
    // @step Given a tool whose business schema already owns top-level "work_unit_id"
    let original = json!({"type":"object","properties":{"work_unit_id":{"type":"string"}}});
    let handle = ToolServer::new().run();
    handle.add_tool(Remote(original.clone())).await?;
    // @step When the agent requests its attributed definition
    let result = configure_tool_server(&handle, "collision").await;
    // @step Then an actionable collision error prevents presenting a corrupted schema
    assert!(
        result.is_err(),
        "reserved business metadata must not be advertised"
    );
    let message = result.err().ok_or("expected collision error")?.to_string();
    assert!(message.contains("work_unit_id") && message.contains("remote"));
    // @step And the remote business property is not silently overwritten
    assert_eq!(original["properties"]["work_unit_id"]["type"], "string");
    Ok(())
}

#[tokio::test]
async fn scenario_newly_added_tools_and_rebuilt_agents_cannot_omit_attribution() -> TestResult {
    // @step Given an agent with static dynamic and manually implemented tools
    let session = Uuid::new_v4();
    let original = json!({"type":"object","properties":{"target":{"type":"string"}}});
    let first = ToolServer::new().tool(ReadTool::new(session)).run();
    configure_tool_server(&first, session.to_string()).await?;
    // @step When an MCP tool is registered mid-turn and the agent is rebuilt for another model
    first.add_tool(Remote(original.clone())).await?;
    let rebuilt = ToolServer::new().tool(ReadTool::new(session)).run();
    configure_tool_server(&rebuilt, session.to_string()).await?;
    rebuilt.add_tool(Remote(original)).await?;
    // @step Then every newly exposed definition retains the attribution requirement
    for handle in [&first, &rebuilt] {
        for definition in handle.get_tool_defs(None).await? {
            assert_contract(&definition);
        }
    }
    // @step And read-only auxiliary agents use the same advertised contract
    let auxiliary = definitions(ToolServer::new().tool(ReadTool::new(Uuid::new_v4()))).await?;
    assert_contract(&auxiliary[0]);
    lifecycle::exercise_lifecycle().await?;
    mcp::exercise_hot_registration().await?;
    mcp::exercise_collision().await?;
    mcp::exercise_actor_rejection().await?;
    Ok(())
}
