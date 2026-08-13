#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Shared helpers for PROV-064 custom-provider streaming SSE bridge tests.
//!
//! Included via `#[path = "custom_streaming_test_helpers.rs"] mod helpers;`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use codelet_providers::custom::stream::{RhaiStreamProcessor, StreamChunk};
use codelet_providers::custom::{
    ApiStyle, AuthConfig, ModelDef, ProviderConfig, RhaiCustomProvider, ScriptLoader, ToolStyle,
};
use codelet_providers::error::ProviderError;
use tempfile::TempDir;

/// Write a `.rhai` script body to `dir/filename` and return the path.
pub fn write_script(dir: &Path, filename: &str, body: &str) -> PathBuf {
    let path = dir.join(filename);
    fs::write(&path, body).expect("write script");
    path
}

/// Build a minimal `ProviderConfig` pointing at a freshly-written script on
/// disk. The returned `TempDir` must be retained by the caller for the
/// lifetime of the test.
pub fn streaming_config_with_script(
    name: &str,
    base_url: &str,
    script_body: &str,
) -> (TempDir, ProviderConfig) {
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_script(tmp.path(), "p.rhai", script_body);

    let mut models: HashMap<String, ModelDef> = HashMap::new();
    models.insert(
        "smart".to_string(),
        ModelDef {
            id: "model-smart-v2".to_string(),
            context_window: 128_000,
            max_output_tokens: 4096,
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_vision: false,
        },
    );

    let cfg = ProviderConfig {
        name: name.to_string(),
        display_name: "My LLM".to_string(),
        base_url: base_url.to_string(),
        script: script_path.to_string_lossy().to_string(),
        facade: None,
        api_key_env_var: None,
        auth: AuthConfig::Bearer {
            env_var: "MY_KEY".to_string(),
            token_prefix: "Bearer".to_string(),
        },
        models,
        defaults: None,
        system_prompt: None,
        tool_style: ToolStyle::Openai,
        api_style: ApiStyle::OpenaiChat,
        headers: HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };
    (tmp, cfg)
}

/// Construct a `RhaiCustomProvider` from a config + freshly-allocated
/// script loader.
pub fn build_streaming_provider(cfg: ProviderConfig) -> RhaiCustomProvider {
    let loader = Arc::new(ScriptLoader::with_default_engine());
    RhaiCustomProvider::new(Arc::new(cfg), loader, "smart".to_string())
        .expect("construct RhaiCustomProvider")
}

/// Build a `RhaiStreamProcessor` ready for unit-level event processing.
///
/// Each test feeds a sequence of strings through `process_event` just as
/// the HTTP streaming path would.
pub fn build_processor(script_body: &str) -> (TempDir, RhaiStreamProcessor) {
    let tmp = TempDir::new().expect("tempdir");
    let script_path = write_script(tmp.path(), "p.rhai", script_body);
    let loader = Arc::new(ScriptLoader::with_default_engine());
    let ast = loader.load(&script_path).expect("load script");
    loader
        .validate_required_functions(&ast)
        .expect("required fns");
    let engine = loader.engine_arc();
    let processor = RhaiStreamProcessor::new(
        engine,
        ast,
        "my-llm".to_string(),
        rhai::Dynamic::from_map(rhai::Map::new()),
    );
    (tmp, processor)
}

/// Convenience: feed the processor a sequence of SSE data payloads and
/// return the concatenated chunks/errors in the order they were produced.
pub async fn process_events(
    script_body: &str,
    events: &[&str],
) -> (TempDir, Vec<Result<StreamChunk, ProviderError>>) {
    let (tmp, mut processor) = build_processor(script_body);
    let mut out: Vec<Result<StreamChunk, ProviderError>> = Vec::new();
    for data in events {
        if *data == "[DONE]" || data.trim() == "[DONE]" {
            let flushed = processor.mark_done();
            for c in flushed {
                out.push(Ok(c));
            }
            break;
        }
        match processor.process_event(data).await {
            Ok(chunks) => {
                for c in chunks {
                    out.push(Ok(c));
                }
            }
            Err(e) => {
                out.push(Err(e));
                break;
            }
        }
    }
    // Flush any pending tool calls on stream end.
    for chunk in processor.finish() {
        out.push(Ok(chunk));
    }
    (tmp, out)
}

/// A Rhai script that emits a single `text_delta` chunk per event using
/// the OpenAI-compatible shape `{"choices":[{"delta":{"content":"..."}}]}`.
pub const OPENAI_TEXT_DELTA_SCRIPT: &str = r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url + "/v1/chat/completions" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ messages: request.messages, stream: true } }
fn map_error(status, body) {
    if status == 401 { #{ type: "auth", message: "unauthorized" } }
    else { #{ type: "api", message: body } }
}
fn parse_stream_chunk(config, data) {
    let event = json::parse(data);
    if (type_of(event["choices"]) != "()") {
        let delta = event.choices[0].delta;
        if (type_of(delta["content"]) != "()") && type_of(delta.content) == "string" {
            return #{ kind: "text_delta", text: delta.content };
        }
    }
    #{ kind: "ignore" }
}
"#;

/// A Rhai script where `parse_stream_chunk` must NEVER be invoked.
/// Triggers a runtime error if called.
pub const FAIL_IF_CALLED_SCRIPT: &str = r#"
fn build_request(request) { #{ messages: request.messages } }
fn build_headers(config) { #{ "Content-Type": "application/json" } }
fn build_url(config) { config.base_url + "/v1/chat/completions" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn build_stream_request(request) { #{ stream: true } }
fn map_error(status, body) { #{ type: "api", message: body } }
fn parse_stream_chunk(config, data) { throw "parse_stream_chunk must not be called"; }
"#;
