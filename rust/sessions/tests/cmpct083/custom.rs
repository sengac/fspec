use super::fixture::TestResult;
use codelet_providers::custom::{
    ProviderConfig, RhaiCustomProvider, RhaiCustomProviderModel, ScriptLoader,
};
use serde_json::json;
use std::{path::Path, sync::Arc};

pub fn write_config(root: &Path) -> TestResult {
    std::fs::create_dir_all(root.join("providers"))?;
    let script = root.join("providers/gate-custom.rhai");
    std::fs::write(
        &script,
        r#"
fn build_request(request) { #{ messages: request.messages, tools: request.tools } }
fn build_stream_request(request) { #{ messages: request.messages, tools: request.tools, stream: true } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_response(raw) { #{ content: "ready", stop_reason: "end_turn" } }
fn parse_stream_chunk(config, data) { parse_json(data) }
"#,
    )?;
    let config = json!({
        "name":"gate-custom", "display_name":"Gate custom", "base_url":"http://127.0.0.1:1",
        "script":script, "auth":{"type":"bearer","env_var":"OPENAI_API_KEY"},
        "models":{"smart":{"id":"smart","context_window":200000,"max_output_tokens":4096,
            "supports_streaming":true,"supports_tools":true}},
        "tool_style":"openai","api_style":"openai_chat"
    });
    std::fs::write(
        root.join("providers/gate-custom.json"),
        serde_json::to_vec(&config)?,
    )?;
    Ok(())
}

pub fn model(root: &Path, url: &str) -> TestResult<RhaiCustomProviderModel> {
    let mut config: ProviderConfig =
        serde_json::from_slice(&std::fs::read(root.join("providers/gate-custom.json"))?)?;
    config.base_url = url.into();
    let provider = RhaiCustomProvider::new(
        Arc::new(config),
        Arc::new(ScriptLoader::with_default_engine()),
        "smart".into(),
    )?;
    Ok(RhaiCustomProviderModel::new(Arc::new(provider)))
}
