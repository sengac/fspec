use codelet_common::{get_data_dir, set_data_directory};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::PathBuf;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub struct Fixture {
    directory: tempfile::TempDir,
    previous_directory: PathBuf,
    previous_data: Option<PathBuf>,
    environment: Vec<(&'static str, Option<OsString>)>,
}

impl Fixture {
    pub fn new(config: &Value) -> TestResult<Self> {
        let directory = setup_temp_directory()?;
        let mut fixture = Self {
            directory,
            previous_directory: std::env::current_dir()?,
            previous_data: get_data_dir().ok(),
            environment: Vec::new(),
        };
        for (name, value) in [
            ("OPENAI_API_KEY", "prov161-openai"),
            ("OPENAI_CONTEXT_WINDOW", "128000"),
            ("OPENAI_MAX_OUTPUT_TOKENS", "4096"),
            ("ANTHROPIC_API_KEY", "prov161-anthropic"),
            ("GOOGLE_GENERATIVE_AI_API_KEY", "prov161-google"),
            ("ZAI_API_KEY", "prov161-zai"),
            ("FSPEC_DISABLE_SCRIPT_SHADOWING", "1"),
        ] {
            fixture.environment.push((name, std::env::var_os(name)));
            std::env::set_var(name, value);
        }
        fixture
            .environment
            .push(("FSPEC_HOME", std::env::var_os("FSPEC_HOME")));
        std::env::set_var("FSPEC_HOME", fixture.directory.path().join("credentials"));
        fixture
            .environment
            .push(("CODEX_HOME", std::env::var_os("CODEX_HOME")));
        std::env::set_var("CODEX_HOME", fixture.directory.path().join("codex"));
        set_data_directory(fixture.directory.path().to_path_buf())?;
        std::env::set_current_dir(fixture.directory.path())?;
        std::fs::create_dir_all(fixture.directory.path().join("cache"))?;
        std::fs::write(
            fixture.directory.path().join("cache/models.json"),
            serde_json::to_vec(&catalog())?,
        )?;
        std::fs::create_dir_all(fixture.directory.path().join("providers"))?;
        std::fs::write(
            fixture.directory.path().join("providers/private.json"),
            serde_json::to_vec(&json!({
                "name":"private", "display_name":"Private", "base_url":"https://private.invalid",
                "facade":"openai", "models":{"smart":{"id":"api/model-v1"}}
            }))?,
        )?;
        fixture.save(config)?;
        Ok(fixture)
    }

    pub fn save(&self, config: &Value) -> TestResult {
        std::fs::write(
            self.directory.path().join("fspec-config.json"),
            serde_json::to_vec(config)?,
        )?;
        Ok(())
    }

    pub fn retain_openai_credentials(&mut self) {
        for name in ["CLAUDE_CODE_OAUTH_TOKEN", "ZAI_PLAN_API_KEY"] {
            self.environment.push((name, std::env::var_os(name)));
        }
        for name in [
            "ANTHROPIC_API_KEY",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "GOOGLE_GENERATIVE_AI_API_KEY",
            "ZAI_API_KEY",
            "ZAI_PLAN_API_KEY",
        ] {
            std::env::remove_var(name);
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for (name, value) in &self.environment {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        if let Some(directory) = &self.previous_data {
            let _ = set_data_directory(directory.clone());
        }
        let _ = std::env::set_current_dir(&self.previous_directory);
    }
}

fn catalog() -> Value {
    let mut providers = serde_json::Map::new();
    for provider in ["openai", "anthropic", "google", "z-ai"] {
        providers.insert(
            provider.into(),
            json!({
                "id":provider, "name":provider, "models":{
                    "catalog-key": {"id":"api/model-v1", "name":"Catalog model", "tool_call":true,
                        "limit":{"context":1_000_000,"output":64_000}},
                    "ambiguous-one": {"id":"ambiguous-one", "name":"One", "tool_call":true,
                        "limit":{"context":500_000,"output":32_000}},
                    "ambiguous-two": {"id":"ambiguous-two", "name":"Two", "tool_call":true,
                        "limit":{"context":700_000,"output":48_000}}
                }
            }),
        );
    }
    Value::Object(providers)
}
