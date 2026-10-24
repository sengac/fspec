use codelet_core::persistence::reset_stores_for_tests;
use codelet_sessions::SessionManager;
use serde_json::json;
use std::ffi::OsString;
use std::sync::Arc;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub manager: Arc<SessionManager>,
    environment: Vec<(&'static str, Option<OsString>)>,
    data_directory: Option<std::path::PathBuf>,
}

impl Fixture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let environment = [
            "FSPEC_USER_DIR",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "OPENAI_MODEL",
            "OPENAI_CONTEXT_WINDOW",
            "OPENAI_STREAMING",
            "ANTHROPIC_API_KEY",
        ]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect();
        let data_directory = codelet_common::get_data_dir().ok();
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        std::env::set_var("OPENAI_API_KEY", "offline");
        std::env::set_var("ANTHROPIC_API_KEY", "offline");
        std::env::remove_var("OPENAI_CONTEXT_WINDOW");
        std::env::remove_var("OPENAI_STREAMING");
        codelet_common::set_data_directory(directory.path().to_owned())?;
        reset_stores_for_tests();
        let cache = directory.path().join("cache");
        std::fs::create_dir_all(&cache)?;
        std::fs::write(
            cache.join("models.json"),
            include_str!("prov129_codex_models.json"),
        )?;
        let fixture = Self {
            directory,
            manager: Arc::new(SessionManager::new()),
            environment,
            data_directory,
        };
        fixture.save(7)?;
        Ok(fixture)
    }

    pub fn save(&self, budget: u32) -> TestResult {
        std::fs::write(
            self.directory.path().join("fspec-config.json"),
            serde_json::to_vec(&json!({
                "providers": {
                    "openai": {"settings": {
                        "contextWindow": 180000, "maxOutputTokens": 12000,
                        "autoContinue": budget, "maxImages": 2,
                        "preserveThinking": false, "loopDetectionEnabled": false,
                        "loopDetectionWindow": 80
                    }, "profiles": {"careful": {
                        "baseUrl": "http://127.0.0.1:1/v1", "apiKey": "other-profile-key"
                    }}},
                    "anthropic": {"settings": {
                        "contextWindow": 300000, "maxOutputTokens": 16000,
                        "autoContinue": 3, "maxImages": 6,
                        "preserveThinking": true, "loopDetectionEnabled": true,
                        "loopDetectionWindow": 240
                    }, "profiles": {"careful": {
                        "autoContinue": 5, "maxImages": 8, "maxOutputTokens": 18000
                    }}}
                }
            }))?,
        )?;
        Ok(())
    }

    pub async fn create(&self) -> Result<String, String> {
        self.create_with_model("openai/gpt-4o").await
    }

    pub async fn create_with_model(&self, model: &str) -> Result<String, String> {
        let identifier = uuid::Uuid::new_v4().to_string();
        self.manager
            .create_session_with_id(
                &identifier,
                model,
                self.directory.path().to_string_lossy().as_ref(),
                "Runtime settings",
            )
            .await?;
        Ok(identifier)
    }
}

impl Drop for Fixture {
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
