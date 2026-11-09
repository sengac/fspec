//! BUG-547 — a local profile model missing a context window or max output is
//! resolved when a session is created on it (engine report, else refused with
//! where to set it), never discovered at its first turn.
//!
//! Feature: spec/features/a-missing-profile-model-limit-is-resolved-at-selection.feature
//!
//! The TUI scenarios (prompt, cancel, provider settings) are in
//! rust/fspec-tui/tests/bug547_missing_profile_limit_prompt.rs.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::profile_limits::{LimitField, MissingProfileLimit};
use codelet_sessions::SessionManager;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");
const STRATA_MODEL: &str = "qwen3.8-flash-next-iq3_s";

/// An engine on a local port answering `path` with `body`; every request
/// is counted.
struct Engine {
    base_url: String,
    requests: Arc<AtomicUsize>,
}

fn engine(path: &'static str, body: Value) -> Result<Engine, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let base_url = format!("http://{}/v1", listener.local_addr()?);
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    let body = body.to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            counter.fetch_add(1, Ordering::SeqCst);
            let mut request_line = String::new();
            let mut reader = BufReader::new(&stream);
            if reader.read_line(&mut request_line).is_err() {
                continue;
            }
            let mut header = String::new();
            while reader.read_line(&mut header).is_ok_and(|read| read > 2) {
                header.clear();
            }
            let found = request_line
                .split_whitespace()
                .nth(1)
                .is_some_and(|target| target.split('?').next() == Some(path));
            let (status, payload) = if found {
                ("200 OK", body.as_str())
            } else {
                ("404 Not Found", "{}")
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{payload}",
                payload.len()
            );
        }
    });
    Ok(Engine { base_url, requests })
}

/// A hermetic user directory holding `profiles` and a data directory with a
/// seeded models cache; the environment the resolver writes is restored on
/// drop.
struct Fixture {
    user: tempfile::TempDir,
    project: tempfile::TempDir,
    saved: Vec<(&'static str, Option<String>)>,
}

impl Fixture {
    fn new(profiles: Value) -> Result<Self, Box<dyn std::error::Error>> {
        let saved = [
            "FSPEC_USER_DIR",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "OPENAI_MODEL",
            "OPENAI_CONTEXT_WINDOW",
            "OPENAI_MAX_OUTPUT_TOKENS",
            "OPENAI_STREAMING",
        ]
        .into_iter()
        .map(|key| (key, std::env::var(key).ok()))
        .collect();
        let user = codelet_test_helpers::setup_temp_directory()?;
        std::fs::write(
            user.path().join("fspec-config.json"),
            serde_json::to_vec(&json!({"providers": {"openai": {"profiles": profiles}}}))?,
        )?;
        std::env::set_var("FSPEC_USER_DIR", user.path());
        std::env::set_var("OPENAI_API_KEY", "offline");
        let data = user.path().join("data");
        std::fs::create_dir_all(data.join("cache"))?;
        std::fs::write(data.join("cache/models.json"), MODELS_FIXTURE)?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(data)?;
        codelet_sessions::credentials::reset_credential_store();
        let project = codelet_test_helpers::setup_temp_directory()?;
        Ok(Self {
            user,
            project,
            saved,
        })
    }

    fn config(&self) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(&std::fs::read(
            self.user.path().join("fspec-config.json"),
        )?)?)
    }

    async fn create(
        &self,
        manager: &SessionManager,
        model: &str,
    ) -> Result<Result<SessionId, String>, Box<dyn std::error::Error>> {
        let id = uuid::Uuid::new_v4().to_string();
        let project = self.project.path().to_str().ok_or("utf8 project path")?;
        Ok(manager
            .create_session_with_id(&id, model, project, "bug547")
            .await
            .map(|()| SessionId::new(id)))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        codelet_core::persistence::reset_stores_for_tests();
        for (key, value) in &self.saved {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn strata_profile() -> Value {
    json!({"Strata": {
        "baseUrl": "http://127.0.0.1:9/v1",
        "apiKey": "offline",
        "engine": "strata",
        "contextWindow": 200000,
        "customModels": [{"id": STRATA_MODEL}]
    }})
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_declared_limit_always_wins_over_the_engine() -> TestResult {
    // @step Given a vLLM profile declares contextWindow 32768 and maxOutputTokens 4096 and the engine reports max_model_len 131072
    let served = engine(
        "/v1/models",
        json!({"data": [{"id": "served", "max_model_len": 131072}]}),
    )?;
    let fixture = Fixture::new(json!({"Local": {
        "baseUrl": served.base_url,
        "apiKey": "offline",
        "engine": "vllm",
        "contextWindow": 32768,
        "maxOutputTokens": 4096,
        "customModels": [{"id": "served"}]
    }}))?;
    let manager = SessionManager::new();

    // @step When a session is created on its model
    let session = fixture.create(&manager, "openai:Local/served").await??;

    // @step Then the session's context window is 32768 and its max output is 4096
    let model = manager.get_session_model(&session)?;
    assert_eq!(model.context_window, Some(32768));
    assert_eq!(model.max_output_tokens, Some(4096));
    // @step And the engine is not asked
    assert_eq!(served.requests.load(Ordering::SeqCst), 0);
    Ok(())
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_context_window_the_engine_reports_is_used_and_its_source_recorded() -> TestResult {
    // @step Given a llama.cpp profile declares maxOutputTokens 4096 and no contextWindow
    // @step And GET /props reports default_generation_settings.n_ctx 65536
    let served = engine(
        "/props",
        json!({"default_generation_settings": {"n_ctx": 65536, "params": {"n_predict": -1}}}),
    )?;
    let fixture = Fixture::new(json!({"Llama": {
        "baseUrl": served.base_url,
        "apiKey": "offline",
        "engine": "llama-cpp",
        "maxOutputTokens": 4096,
        "customModels": [{"id": "gguf"}]
    }}))?;
    let manager = SessionManager::new();

    // @step When a session is created on its model
    let session = fixture.create(&manager, "openai:Llama/gguf").await??;

    // @step Then the session's context window is 65536
    let model = manager.get_session_model(&session)?;
    assert_eq!(model.context_window, Some(65536));
    assert_eq!(model.max_output_tokens, Some(4096));
    // @step And the context window is recorded as reported by llama.cpp GET /props
    let background = manager.get_session(&session.value)?;
    let inner = background.inner.lock().await;
    let reported = inner
        .provider_manager()
        .engine_context_window()
        .ok_or("the engine-reported context window is recorded")?;
    assert_eq!(reported.tokens, 65536);
    assert_eq!(
        reported.source,
        "llama.cpp GET /props default_generation_settings.n_ctx"
    );
    Ok(())
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_limit_is_refused_at_selection_not_at_the_first_turn() -> TestResult {
    // @step Given the Strata profile declares contextWindow 200000 and no maxOutputTokens, and no engine reports a max output
    let fixture = Fixture::new(strata_profile())?;
    let manager = SessionManager::new();

    // @step When a session is created on openai:Strata/qwen3.8-flash-next-iq3_s without the TUI
    let result = fixture
        .create(&manager, &format!("openai:Strata/{STRATA_MODEL}"))
        .await?;

    // @step Then creation is refused naming maxOutputTokens, the Strata profile, the fspec-config.json path and the TUI provider settings
    let error = result.err().ok_or("creation is refused")?;
    let config_path = fixture.user.path().join("fspec-config.json");
    assert_eq!(
        error,
        format!(
            "openai:Strata/{STRATA_MODEL} has no max output tokens: set maxOutputTokens on the Strata profile in {} (providers.openai.profiles.Strata) or in the TUI provider settings",
            config_path.display()
        )
    );
    let parsed = MissingProfileLimit::find_in(&format!("[error] create session: {error}"))
        .ok_or("the refusal is recognised")?;
    assert_eq!(parsed.field, LimitField::MaxOutputTokens);
    assert_eq!(parsed.provider, "openai");
    assert_eq!(parsed.profile, "Strata");
    assert_eq!(parsed.model, format!("openai:Strata/{STRATA_MODEL}"));
    // @step And no session is created
    assert_eq!(manager.session_count().await, 0);
    assert_eq!(
        fixture.config()?,
        json!({"providers": {"openai": {"profiles": strata_profile()}}})
    );
    Ok(())
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_engine_that_reports_nothing_usable_goes_to_the_user() -> TestResult {
    // @step Given a TensorRT-LLM profile declares maxOutputTokens 4096 and no contextWindow
    let served = engine("/v1/models", json!({"data": [{"id": "trt"}]}))?;
    let fixture = Fixture::new(json!({"Trt": {
        "baseUrl": served.base_url,
        "apiKey": "offline",
        "engine": "tensorrt-llm",
        "maxOutputTokens": 4096,
        "customModels": [{"id": "trt"}]
    }}))?;
    let manager = SessionManager::new();

    // @step When a session is created on its model without the TUI
    let result = fixture.create(&manager, "openai:Trt/trt").await?;

    // @step Then creation is refused naming contextWindow and the profile
    let error = result.err().ok_or("creation is refused")?;
    let parsed = MissingProfileLimit::find_in(&error).ok_or("the refusal is recognised")?;
    assert_eq!(parsed.field, LimitField::ContextWindow);
    assert_eq!(parsed.profile, "Trt");
    assert!(
        error.contains("set contextWindow on the Trt profile"),
        "{error}"
    );
    assert_eq!(served.requests.load(Ordering::SeqCst), 0);
    assert_eq!(manager.session_count().await, 0);
    Ok(())
}

/// An engine value that is not a whole positive token count is not used.
#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unusable_engine_value_is_not_used() -> TestResult {
    let served = engine(
        "/props",
        json!({"default_generation_settings": {"n_ctx": 0}}),
    )?;
    let fixture = Fixture::new(json!({"Llama": {
        "baseUrl": served.base_url,
        "apiKey": "offline",
        "engine": "llama-cpp",
        "maxOutputTokens": 4096,
        "customModels": [{"id": "gguf"}]
    }}))?;
    let manager = SessionManager::new();
    let result = fixture.create(&manager, "openai:Llama/gguf").await?;
    let error = result.err().ok_or("creation is refused")?;
    let parsed = MissingProfileLimit::find_in(&error).ok_or("the refusal is recognised")?;
    assert_eq!(parsed.field, LimitField::ContextWindow);
    assert!(served.requests.load(Ordering::SeqCst) > 0);
    Ok(())
}
