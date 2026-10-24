use super::catalog::{assert_agent, CompletionRegistration};
use codelet_providers::custom::CustomProvider;
use serde_json::json;

#[path = "../fixtures/mod.rs"]
mod fixtures;

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) async fn custom_builders() -> TestResult {
    let (home, _environment) = fixtures::setup_fspec_home()?;
    let project = codelet_test_helpers::setup_temp_directory()?;
    let directory = home.path().join("providers");
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join("catalog.rhai"), SCRIPT)?;
    let session = CompletionRegistration::new();
    for style in ["openai", "claude", "anthropic", "gemini", "codex"] {
        let name = format!("tool025-{style}");
        let configuration = json!({
            "name":name, "display_name":"Catalog fixture", "base_url":"http://127.0.0.1:1",
            "script":"catalog.rhai", "auth":{"type":"bearer","env_var":"TOOL025_UNUSED"},
            "models":{"smart":{"id":"fixture-model","context_window":100000,
                "max_output_tokens":4096,"supports_streaming":true,"supports_tools":true}},
            "tool_style":style,"api_style":"openai_chat"
        });
        std::fs::write(
            directory.join(format!("{name}.json")),
            serde_json::to_vec(&configuration)?,
        )?;
        let main = CustomProvider::create_rig_agent(
            project.path(),
            &name,
            "smart",
            session.0,
            Some("Fixture instructions"),
            None,
            false,
        )?;
        assert!(main.tool_adapter_count() > 0, "{style} omitted Rhai tools");
        assert_agent(main.into_inner(), &name, true).await?;
        let auxiliary = CustomProvider::create_rig_agent(
            project.path(),
            &name,
            "smart",
            session.0,
            Some("Fixture instructions"),
            None,
            true,
        )?;
        let auxiliary = auxiliary.into_inner();
        let tools = auxiliary.tool_server_handle.get_tool_defs(None).await?;
        assert_eq!(tools.len(), 7, "auxiliary tool allowlist changed");
        assert!(!tools
            .iter()
            .any(|tool| tool.name == codelet_tools::DONE_TOOL_NAME));
        assert_agent(auxiliary, &format!("{name}-auxiliary"), false).await?;
    }
    Ok(())
}

const SCRIPT: &str = r#"
fn build_request(request) { #{ tools: request.tools } }
fn build_stream_request(request) { #{ tools: request.tools } }
fn build_headers(config) { #{} }
fn build_url(config) { config.base_url }
fn parse_response(raw) { #{ content: "done", stop_reason: "end_turn" } }
fn parse_stream_chunk(config, data) { #{ kind: "text", text: "done" } }
fn map_error(status, body) { #{ type: "api", message: body } }
"#;
