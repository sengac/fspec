use codelet_providers::codex::CodexProvider;
use codelet_providers::copilot::{CopilotDeploymentType, CopilotProvider};
use codelet_providers::{ClaudeProvider, GeminiProvider, OpenAIProvider, ZAIProvider};
use codelet_tools::tool_attribution::{configure_agent_tools, decorate_definition};
use rig::agent::Agent;
use rig::completion::{Completion, CompletionModel};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) struct CompletionRegistration(pub(super) Uuid);

impl CompletionRegistration {
    pub(super) fn new() -> Self {
        let session = Uuid::new_v4();
        codelet_tools::set_continue_armed(session, true);
        Self(session)
    }
}

impl Drop for CompletionRegistration {
    fn drop(&mut self) {
        codelet_tools::set_continue_armed(self.0, false);
    }
}

pub(super) async fn builtins() -> TestResult {
    let session = CompletionRegistration::new();
    let openai = OpenAIProvider::from_api_key_with_options("test-key", "gpt-4o", None, None)?;
    assert_agent(
        openai.create_rig_agent(session.0, None, None),
        "openai",
        true,
    )
    .await?;
    let claude = ClaudeProvider::from_api_key_with_model("test-key", "claude-sonnet-4-6")?;
    assert_agent(
        claude.create_rig_agent(session.0, None, None),
        "anthropic",
        true,
    )
    .await?;
    let gemini = GeminiProvider::from_api_key("test-key", "gemini-2.5-pro")?;
    assert_agent(
        gemini.create_rig_agent(session.0, None, None),
        "gemini",
        true,
    )
    .await?;
    let zai = ZAIProvider::from_api_key("test-key", "glm-4.7")?;
    assert_agent(zai.create_rig_agent(session.0, None, None), "zai", true).await?;
    let codex = CodexProvider::from_api_key("test-key", "gpt-5-codex")?;
    assert_agent(codex.create_rig_agent(session.0, None, None), "codex", true).await?;
    let copilot = CopilotProvider::new(
        CopilotDeploymentType::GitHubCom,
        "test-token".into(),
        "gpt-4o",
    )?;
    assert_agent(
        copilot.create_rig_agent(session.0, None, None),
        "copilot",
        true,
    )
    .await?;
    Ok(())
}

pub(super) async fn assert_agent<M: CompletionModel + 'static>(
    agent: Agent<M>,
    name: &str,
    includes_completion: bool,
) -> TestResult {
    let original = agent.tool_server_handle.get_tool_defs(None).await?;
    assert!(!original.is_empty(), "{name} has no tools");
    if includes_completion {
        assert!(
            original.len() >= 20,
            "{name} did not build its full catalog"
        );
        assert!(
            original
                .iter()
                .any(|tool| tool.name == codelet_tools::DONE_TOOL_NAME),
            "{name} omitted completion"
        );
    }
    configure_agent_tools(&agent, format!("transport-matrix-{name}")).await?;
    let request = agent
        .completion("Inspect project files", Vec::new())
        .await?
        .build();
    assert_eq!(
        request.tools.len(),
        original.len(),
        "{name} lost tools during collection"
    );
    for definition in &original {
        let expected = decorate_definition(definition.clone())?;
        let actual = request
            .tools
            .iter()
            .find(|tool| tool.name == definition.name)
            .ok_or("configured request lost a registered tool")?;
        assert_eq!(
            actual.parameters, expected.parameters,
            "{name}/{}",
            definition.name
        );
    }
    let body: Value = serde_json::from_slice(&agent.model.prepare_request(request)?)?;
    let mut serialized = BTreeMap::new();
    collect(&body, &mut serialized)?;
    assert_eq!(
        serialized.len(),
        original.len(),
        "{name} lost serialized tools"
    );
    for definition in original {
        let parameters = serialized
            .get(&definition.name)
            .ok_or("serialized tool missing")?;
        let decorated = decorate_definition(definition.clone())?;
        let expected = decorated.parameters.clone();
        let expected = if name == "codex" {
            rig::providers::openai::responses_api::ResponsesToolDefinition::from(decorated)
                .parameters
        } else {
            expected
        };
        assert_eq!(
            parameters, &expected,
            "{name}/{} changed the complete schema",
            definition.name
        );
        let required = parameters["required"]
            .as_array()
            .ok_or("required missing")?;
        assert!(
            required.contains(&json!("work_unit_id")),
            "{name}/{}: {parameters}",
            definition.name
        );
        let attribution = &parameters["properties"]["work_unit_id"];
        assert_eq!(attribution["type"], "string", "{name}/{}", definition.name);
        assert_eq!(attribution["minLength"], 1);
        assert_ne!(parameters["nullable"], true);
        assert_ne!(attribution["nullable"], true);
        if let Some(business) = definition.parameters["required"].as_array() {
            for field in business {
                assert!(
                    required.contains(field),
                    "{name}/{} lost {field}",
                    definition.name
                );
            }
        }
    }
    Ok(())
}

fn collect(value: &Value, output: &mut BTreeMap<String, Value>) -> TestResult {
    if value.get("parametersJsonSchema").is_some() {
        assert!(
            value.get("parameters").is_none(),
            "Gemini parameter schema fields are mutually exclusive"
        );
    }
    if let (Some(name), Some(parameters)) = (
        value["name"].as_str(),
        value
            .get("parametersJsonSchema")
            .or_else(|| value.get("parameters"))
            .or_else(|| value.get("input_schema")),
    ) {
        assert!(
            output.insert(name.into(), parameters.clone()).is_none(),
            "duplicate serialized tool {name}"
        );
        return Ok(());
    }
    match value {
        Value::Object(object) => {
            for child in object.values() {
                collect(child, output)?;
            }
        }
        Value::Array(array) => {
            for child in array {
                collect(child, output)?;
            }
        }
        _ => {}
    }
    Ok(())
}
