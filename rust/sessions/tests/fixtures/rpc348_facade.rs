use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use codelet_core::persistence::reset_stores_for_tests;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

const MY_LLM_JSON: &str = r#"{
  "name": "my-llm",
  "display_name": "Test my-llm",
  "base_url": "https://api.example.com/v1",
  "facade": "openai",
  "api_key_env_var": "MY_LLM_API_KEY",
  "tool_style": "openai",
  "api_style": "openai_chat",
  "models": { "llama-3.1-70b": { "id": "llama-3.1-70b" } }
}"#;

const CLAUDE_COMPAT_JSON: &str = r#"{
  "name": "claude-compat",
  "display_name": "Test claude-compat",
  "base_url": "https://api.anthropic.com",
  "api_key_env_var": "ANTHROPIC_API_KEY",
  "tool_style": "claude",
  "api_style": "anthropic_messages",
  "models": { "opus-compat": { "id": "opus-compat" } }
}"#;

pub struct TestDirectory {
    directory: tempfile::TempDir,
    environment: Vec<(&'static str, Option<OsString>)>,
    data_directory: Option<PathBuf>,
}

impl TestDirectory {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let fixture = Self {
            directory: codelet_test_helpers::setup_temp_directory()?,
            environment: [
                "HOME",
                "FSPEC_HOME",
                "FSPEC_USER_DIR",
                "ANTHROPIC_API_KEY",
                "GOOGLE_GENERATIVE_AI_API_KEY",
                "MY_LLM_API_KEY",
                "OPENAI_API_KEY",
                "OPENAI_BASE_URL",
                "OPENAI_MODEL",
                "OPENAI_CONTEXT_WINDOW",
                "OPENAI_STREAMING",
            ]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect(),
            data_directory: codelet_common::get_data_dir().ok(),
        };
        let fspec = fixture.directory.path().join(".fspec");
        let credentials = fspec.join("credentials");
        let providers = fspec.join("providers");
        std::fs::create_dir_all(&credentials)?;
        std::fs::create_dir_all(&providers)?;
        std::fs::write(providers.join("my-llm.json"), MY_LLM_JSON)?;
        std::fs::write(providers.join("claude-compat.json"), CLAUDE_COMPAT_JSON)?;

        std::env::set_var("HOME", fixture.directory.path());
        std::env::set_var("FSPEC_HOME", credentials);
        std::env::set_var("FSPEC_USER_DIR", &fspec);
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
        std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
        std::env::set_var("MY_LLM_API_KEY", "my-llm-test-dummy-key");
        for key in [
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "OPENAI_MODEL",
            "OPENAI_CONTEXT_WINDOW",
            "OPENAI_STREAMING",
        ] {
            std::env::remove_var(key);
        }
        codelet_common::set_data_directory(fspec)?;
        reset_stores_for_tests();
        Ok(fixture)
    }

    pub fn manager_with_default(
        &self,
        default_model: &str,
    ) -> Result<(Arc<SessionManager>, SessionId), Box<dyn std::error::Error>> {
        let manager = Arc::new(SessionManager::new());
        manager.set_default_model(default_model);
        let handle: &dyn SessionManagerHandle = manager.as_ref();
        let identifier = handle.create_session(None);
        if identifier.value.is_empty() {
            return Err("session creation must succeed offline".into());
        }
        Ok((manager, identifier))
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        reset_stores_for_tests();
        if let Some(directory) = &self.data_directory {
            let _ = codelet_common::set_data_directory(directory.clone());
        }
        for (key, value) in &self.environment {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

pub async fn facade_for(
    manager: &Arc<SessionManager>,
    identifier: &SessionId,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let session = manager.get_session(&identifier.value)?;
    let inner = session.inner.lock().await;
    Ok(inner
        .provider_manager()
        .facade_override()
        .map(str::to_string))
}
