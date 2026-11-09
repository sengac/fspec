#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/complete-customprovider-create-rig-agent-construct-real-rig-agent-agent.feature
//!
//! PROV-092 integration tests. The keystone work unit: replace the
//! opaque `CustomRigAgent` shim with a fully wired
//! `rig::agent::Agent<RhaiCustomProviderModel>` driven by rig's normal
//! completion / streaming / tool plumbing.

use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use codelet_common::{Message, MessageContent};
use codelet_providers::custom::tool_facade::RhaiToolDef;
use codelet_providers::custom::{
    rig_message_convert::rig_messages_to_internal, AuthConfig, CustomProvider, ModelDef,
    ProviderConfig, RhaiCustomCompletion, RhaiCustomProvider, RhaiCustomProviderModel,
    RhaiToolArgs, RhaiToolFacadeAdapter, RhaiToolWrapper, ScriptLoader,
};
use codelet_tools::facade::SystemPromptFacade;
use rig::completion::{
    CompletionModel, CompletionRequest, Message as RigMessage, ToolDefinition as RigToolDefinition,
};
use rig::message::{AssistantContent, UserContent};
use rig::streaming::RawStreamingChoice;
use rig::tool::Tool;
use rig::OneOrMany;
use serde_json::json;
use serial_test::serial;
use tempfile::TempDir;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =========================================================================
// Helpers
// =========================================================================

struct ProjectFixture {
    _tmp: TempDir,
    _home_tmp: TempDir,
    project_root: std::path::PathBuf,
    providers_dir: std::path::PathBuf,
    prior_cwd: std::path::PathBuf,
    prior_home: Option<String>,
    prior_fspec_home: Option<String>,
}

impl ProjectFixture {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let home_tmp = TempDir::new().expect("home tempdir");
        let project_root = tmp.path().to_path_buf();
        let providers_dir = project_root.join(".fspec").join("providers");
        fs::create_dir_all(&providers_dir).expect("mkdir");
        let fspec_creds = home_tmp.path().join(".fspec").join("credentials");
        fs::create_dir_all(&fspec_creds).expect("mkdir creds");
        let prior_cwd = std::env::current_dir().expect("cwd");
        let prior_home = std::env::var("HOME").ok();
        let prior_fspec_home = std::env::var("FSPEC_HOME").ok();
        std::env::set_var("HOME", home_tmp.path());
        std::env::set_var("FSPEC_HOME", &fspec_creds);
        std::env::set_current_dir(&project_root).expect("chdir");
        Self {
            _tmp: tmp,
            _home_tmp: home_tmp,
            project_root,
            providers_dir,
            prior_cwd,
            prior_home,
            prior_fspec_home,
        }
    }
}

impl Drop for ProjectFixture {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior_cwd);
        match self.prior_home.take() {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match self.prior_fspec_home.take() {
            Some(v) => std::env::set_var("FSPEC_HOME", v),
            None => std::env::remove_var("FSPEC_HOME"),
        }
    }
}

impl ProjectFixture {
    fn write_provider(&self, name: &str, script_body: &str, base_url: &str) {
        let script_path = self.providers_dir.join(format!("{name}.rhai"));
        fs::write(&script_path, script_body).expect("write rhai");
        let json_body = json!({
            "name": name,
            "display_name": name,
            "base_url": base_url,
            "script": format!("{name}.rhai"),
            "facade": null,
            "auth": {
                "type": "bearer",
                "env_var": "TEST_KEY",
                "token_prefix": "Bearer"
            },
            "models": {
                "smart": {
                    "id": "model-smart-v2",
                    "context_window": 128000,
                    "max_output_tokens": 4096,
                    "supports_tools": true,
                    "supports_streaming": true,
                    "supports_thinking": true
                }
            },
            "tool_style": "openai",
            "api_style": "openai_chat",
            "headers": {}
        });
        fs::write(
            self.providers_dir.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&json_body).unwrap(),
        )
        .expect("write json");
    }
}

fn build_provider_inline(script_body: &str, name: &str, base_url: &str) -> RhaiCustomProvider {
    let tmp = TempDir::new().expect("tempdir");
    let script_path = tmp.path().join(format!("{name}.rhai"));
    fs::write(&script_path, script_body).expect("write rhai");
    let mut models = HashMap::new();
    models.insert(
        "smart".to_string(),
        ModelDef {
            id: "model-smart-v2".to_string(),
            context_window: Some(128_000),
            max_output_tokens: Some(4096),
            supports_tools: true,
            supports_streaming: true,
            supports_thinking: true,
            supports_vision: false,
        },
    );
    let cfg = ProviderConfig {
        name: name.to_string(),
        display_name: name.to_string(),
        base_url: base_url.to_string(),
        script: script_path.to_string_lossy().to_string(),
        facade: None,
        api_key_env_var: None,
        auth: AuthConfig::Bearer {
            env_var: "TEST_KEY".to_string(),
            token_prefix: "Bearer".to_string(),
        },
        models,
        defaults: None,
        system_prompt: None,
        tool_style: codelet_providers::custom::ToolStyle::Openai,
        api_style: codelet_providers::custom::ApiStyle::OpenaiChat,
        headers: HashMap::new(),
        env_prefix: None,
        resolved_tools: None,
    };
    // tmp must live for the test — leak it intentionally by boxing the
    // provider config path; the script file stays alive until the test
    // process exits.
    std::mem::forget(tmp);
    let loader = Arc::new(ScriptLoader::with_default_engine());
    RhaiCustomProvider::new(Arc::new(cfg), loader, "smart".to_string())
        .expect("construct RhaiCustomProvider")
}

fn make_completion_request(user_text: &str) -> CompletionRequest {
    let history = OneOrMany::one(RigMessage::User {
        content: OneOrMany::one(UserContent::Text(user_text.into())),
    });
    CompletionRequest {
        preamble: None,
        chat_history: history,
        documents: vec![],
        tools: vec![],
        temperature: None,
        max_tokens: None,
        tool_choice: None,
        additional_params: None,
    }
}

fn happy_script(base_url: &str) -> String {
    let base = base_url;
    format!(
        r#"
fn build_url(config) {{ "{base}/v1/chat/completions" }}
fn build_headers(config) {{ #{{ "Content-Type": "application/json" }} }}
fn build_request(request) {{
    let thinking = if type_of(request.thinking_config) == "()" {{ () }} else {{ request.thinking_config }};
    #{{
        messages: request.messages,
        thinking_config: thinking
    }}
}}
fn parse_response(raw) {{
    #{{
        content: raw.choices[0].message.content,
        stop_reason: "end_turn"
    }}
}}
fn parse_stream_chunk(config, data) {{ #{{ kind: "ignore" }} }}
fn build_stream_request(ctx) {{ #{{}} }}
fn map_error(status, body) {{ #{{ type: "api", message: "other" }} }}
"#,
    )
}

// Test functions live in submodules (REFAC-030) so every file stays under 300 lines.
#[path = "rhai_rig_agent_keystone/agent_and_completion.rs"]
mod agent_and_completion;
#[path = "rhai_rig_agent_keystone/dispatch_and_conversion.rs"]
mod dispatch_and_conversion;
#[path = "rhai_rig_agent_keystone/thinking_tools_prompt.rs"]
mod thinking_tools_prompt;
