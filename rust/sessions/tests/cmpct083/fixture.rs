use codelet_core::persistence::*;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub const SENTINEL: &str = "deep-original-sentinel-not-in-summary-83";
pub const SUFFIX: &str = "complete suffix result";

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub manager: Arc<SessionManager>,
    pub background: Arc<BackgroundSession>,
    environment: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

pub fn environment(root: &Path) -> TestResult {
    codelet_common::set_data_directory(root.to_owned())?;
    std::env::set_var("FSPEC_USER_DIR", root);
    std::env::set_var("FSPEC_HOME", root.join("credentials"));
    for key in [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GEMINI_API_KEY",
        "GOOGLE_API_KEY",
        "GOOGLE_GENERATIVE_AI_API_KEY",
        "GITHUB_TOKEN",
        "ZAI_API_KEY",
    ] {
        std::env::set_var(key, "offline-gate-key");
    }
    codelet_providers::stub_provider::register_stub_provider();
    reset_stores_for_tests();
    Ok(())
}

pub fn assert_gemini_suffix(wire: &Value) -> TestResult {
    let contents = wire["contents"].as_array().ok_or("Gemini contents")?;
    let mut calls = Vec::new();
    let mut results = Vec::new();
    for (position, content) in contents.iter().enumerate() {
        for part in content["parts"].as_array().ok_or("Gemini parts")? {
            if part["functionCall"]["name"] == "lookup" {
                assert_eq!(content["role"], "model");
                assert_eq!(part["functionCall"]["args"]["path"], "src/deep.rs");
                calls.push(position);
            }
            if part["functionResponse"]["response"]["result"] == SUFFIX {
                assert_eq!(content["role"], "user");
                assert_eq!(
                    part["functionResponse"]["name"], "lookup",
                    "Gemini must pair its native result with the called function name"
                );
                results.push(position);
            }
        }
    }
    assert_eq!(
        calls.len(),
        1,
        "Gemini duplicated or omitted the suffix call"
    );
    assert_eq!(
        results.len(),
        1,
        "Gemini duplicated or omitted the suffix result"
    );
    assert!(calls[0] < results[0], "Gemini reordered suffix call/result");
    Ok(())
}

pub fn assert_stub_suffix(wire: &Value) -> TestResult {
    let messages = wire["messages"].as_array().ok_or("stub messages")?;
    let mut calls = Vec::new();
    let mut results = Vec::new();
    for (position, message) in messages.iter().enumerate() {
        for part in message["content"].as_array().ok_or("stub content")? {
            if part["id"] != "call_after_snapshot" {
                continue;
            }
            if !part["call_id"].is_null() {
                assert_eq!(part["call_id"], "call_after_snapshot");
            }
            if part["type"] == "toolresult" {
                assert_eq!(message["role"], "user");
                assert_eq!(part["content"][0]["text"], SUFFIX);
                results.push(position);
            } else {
                assert_eq!(message["role"], "assistant");
                assert_eq!(part["function"]["name"], "lookup");
                assert_eq!(part["function"]["arguments"]["path"], "src/deep.rs");
                calls.push(position);
            }
        }
    }
    assert_eq!(calls.len(), 1, "stub duplicated or omitted the suffix call");
    assert_eq!(
        results.len(),
        1,
        "stub duplicated or omitted the suffix result"
    );
    assert!(calls[0] < results[0], "stub reordered suffix call/result");
    Ok(())
}

pub fn model(row: &str) -> &str {
    match row {
        "anthropic" => "anthropic/claude-sonnet-4",
        "openai" => "openai/o3",
        "gemini" => "google/gemini-2.5-pro",
        "copilot" => "github-copilot/gpt-4o",
        "zai" | "zai-plan" => "zai/glm-4.7",
        "codex" => "codex/shared-model",
        "custom" => "gate-custom/smart",
        "stub" => "stub/deterministic",
        _ => "invalid/invalid",
    }
}

pub fn output_limit(row: &str) -> usize {
    match row {
        "zai" | "zai-plan" => codelet_providers::zai::MAX_OUTPUT_TOKENS,
        _ => 4096,
    }
}

impl Fixture {
    pub async fn new(row: &str) -> TestResult<Self> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let environment = [
            "FSPEC_USER_DIR",
            "FSPEC_HOME",
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "OPENAI_MODEL",
            "GEMINI_API_KEY",
            "GOOGLE_API_KEY",
            "GOOGLE_GENERATIVE_AI_API_KEY",
            "GITHUB_TOKEN",
            "ZAI_API_KEY",
            "ZAI_PLAN_API_KEY",
        ]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
        environment_setup(directory.path())?;
        if row == "zai-plan" {
            std::env::set_var("ZAI_PLAN_API_KEY", "offline-gate-plan-key");
        } else {
            std::env::remove_var("ZAI_PLAN_API_KEY");
        }
        let manager = Arc::new(SessionManager::new());
        let mut manifest =
            create_session_with_provider("runtime gate", directory.path(), model(row))?;
        super::originals::append(&mut manifest)?;
        manager
            .create_session_from_manifest(&manifest, model(row))
            .await?;
        let background = manager.get_session(&manifest.id.to_string())?;
        {
            let mut inner = background.inner.lock().await;
            inner
                .provider_manager_mut()
                .override_model_limits(Some(200_000), Some(output_limit(row)));
        }
        Ok(Self {
            directory,
            manager,
            background,
            environment,
        })
    }

    pub fn bytes(&self) -> TestResult<Vec<u8>> {
        manifest_bytes(self.directory.path(), self.background.id)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(None);
        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(self.background.id, None);
        codelet_tools::set_inject_summary_handler(self.background.id, None);
        codelet_tools::set_generate_compaction_handler(self.background.id, None);
        for (key, previous) in &self.environment {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        reset_stores_for_tests();
    }
}

pub fn environment_setup(root: &Path) -> TestResult {
    environment(root)?;
    std::fs::create_dir_all(root.join("cache"))?;
    std::fs::create_dir_all(root.join("credentials"))?;
    std::fs::write(
        root.join("credentials/copilot_auth.json"),
        serde_json::to_vec(&json!({
            "github_oauth_token":"offline-gate-oauth",
            "copilot_token":"offline-gate-token",
            "copilot_token_expires_at":4102444800_u64
        }))?,
    )?;
    let mut registry: Value =
        serde_json::from_str(include_str!("../fixtures/prov123_models.json"))?;
    for row in [
        "anthropic",
        "openai",
        "gemini",
        "copilot",
        "zai",
        "codex",
        "custom",
        "stub",
    ] {
        let (provider, name) = model(row).split_once('/').ok_or("model identity")?;
        registry[provider] = json!({"id":provider,"name":provider,"env":[],"models":{
            (name):{"id":name,"name":name,"tool_call":true,"limit":{"context":200000,"output":output_limit(row)}}
        }});
    }
    std::fs::write(
        root.join("cache/models.json"),
        serde_json::to_vec(&registry)?,
    )?;
    super::custom::write_config(root)?;
    Ok(())
}

pub fn manifest_bytes(root: &Path, id: uuid::Uuid) -> TestResult<Vec<u8>> {
    Ok(std::fs::read(root.join(format!("sessions/{id}.json")))?)
}

pub fn dag(count: usize, cycle: usize) -> TestResult<String> {
    let end = count.checked_sub(1).ok_or("source must not be empty")?;
    Ok(format!("<dag-node depth=\"D1\" turns=\"0-{end}\" label=\"Work\">Committed cycle {cycle}</dag-node>\n<dag-files>\n- src/deep.rs (Modified)\n</dag-files>"))
}

pub fn append_suffix(id: uuid::Uuid) -> TestResult {
    let mut manifest = load_session(id)?;
    for (role, content) in [
        (
            "assistant",
            json!([{"type":"tool_use","id":"call_after_snapshot","name":"lookup","input":{"path":"src/deep.rs"}}]),
        ),
        (
            "user",
            json!([{"type":"tool_result","tool_use_id":"call_after_snapshot","content":SUFFIX}]),
        ),
    ] {
        append_message_with_metadata(
            &mut manifest,
            role,
            "preview",
            [("message".into(), json!({"role":role,"content":content}))]
                .into_iter()
                .collect(),
        )?;
    }
    Ok(())
}
