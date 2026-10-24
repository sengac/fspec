use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use rig::prelude::CompletionClient;
use serde_json::{json, Value};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn cold_process_session() -> Result<Option<SessionId>, Box<dyn std::error::Error>> {
    let Some(root) = std::env::var_os("CMPCT_SESSION_COLD_ROOT") else {
        return Ok(None);
    };
    codelet_common::set_data_directory(root.clone().into())?;
    std::env::set_var("FSPEC_USER_DIR", root);
    Ok(Some(SessionId::new(std::env::var(
        "CMPCT_SESSION_COLD_ID",
    )?)))
}

pub async fn run_cold_process(
    root: &std::path::Path,
    session_id: &SessionId,
    scenario: &str,
) -> TestResult {
    codelet_core::persistence::reset_stores_for_tests();
    let mut child = tokio::process::Command::new(std::env::current_exe()?)
        .args(["--exact", scenario, "--nocapture"])
        .env("CMPCT_SESSION_COLD_ROOT", root)
        .env("CMPCT_SESSION_COLD_ID", &session_id.value)
        .kill_on_drop(true)
        .spawn()?;
    let status = tokio::time::timeout(std::time::Duration::from_secs(30), child.wait()).await??;
    if !status.success() {
        return Err(format!("fresh process restore failed: {status}").into());
    }
    Ok(())
}

pub struct TestDirectory {
    directory: tempfile::TempDir,
    previous_key: Option<std::ffi::OsString>,
    previous_user_dir: Option<std::ffi::OsString>,
}

pub fn cold_resume(session_id: &SessionId) -> Result<SessionManager, String> {
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    manager.resume_session(session_id)?;
    Ok(manager)
}

impl TestDirectory {
    pub fn path(&self) -> &std::path::Path {
        self.directory.path()
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        for (key, previous) in [
            ("ANTHROPIC_API_KEY", &self.previous_key),
            ("FSPEC_USER_DIR", &self.previous_user_dir),
        ] {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        codelet_core::persistence::reset_stores_for_tests();
    }
}

pub async fn manager(
) -> Result<(TestDirectory, SessionManager, SessionId, usize), Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let directory = TestDirectory {
        directory,
        previous_key: std::env::var_os("ANTHROPIC_API_KEY"),
        previous_user_dir: std::env::var_os("FSPEC_USER_DIR"),
    };
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    std::env::set_var("ANTHROPIC_API_KEY", "test-restore");
    std::env::set_var("FSPEC_USER_DIR", directory.path());
    std::fs::create_dir_all(directory.path().join("cache"))?;
    std::fs::write(
        directory.path().join("cache/models.json"),
        include_str!("../fixtures/prov123_models.json"),
    )?;
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    let session_id = SessionId::new(
        manager
            .create_session(
                "anthropic/claude-sonnet-4",
                &directory.path().to_string_lossy(),
            )
            .await?,
    );
    let baseline = manager
        .get_session(&session_id.value)?
        .inner
        .lock()
        .await
        .messages
        .len();
    Ok((directory, manager, session_id, baseline))
}

pub fn envelope(content: Value) -> Value {
    json!({"message": {"role": "user", "content": content}})
}

pub async fn next_wire(
    manager: &SessionManager,
    session_id: &SessionId,
    _baseline: usize,
) -> Result<Value, Box<dyn std::error::Error>> {
    let server = MockServer::start().await;
    let response = json!({
        "id":"resp_test","object":"response","created_at":1,
        "status":"completed","model":"test","output":[],
        "usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2,
            "input_tokens_details":{"cached_tokens":0},
            "output_tokens_details":{"reasoning_tokens":0}}
    });
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            format!(
                "data: {}\n\n",
                json!({"type":"response.completed","sequence_number":1,"response":response})
            ),
            "text/event-stream",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let client: rig::providers::openai::Client = rig::providers::openai::Client::builder()
        .api_key("test")
        .base_url(server.uri())
        .build()?;
    stream_session(manager, session_id, client.completion_model("test")).await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("capture unavailable")?;
    Ok(requests.first().ok_or("no provider request")?.body_json()?)
}

async fn stream_session<Model>(
    manager: &SessionManager,
    session_id: &SessionId,
    model: Model,
) -> TestResult
where
    Model: rig::completion::CompletionModel + 'static,
    Model::StreamingResponse: Send + rig::completion::GetTokenUsage,
{
    let background = manager.get_session(&session_id.value)?;
    let mut inner = background.inner.lock().await;
    let output = codelet_agent_loop::BackgroundOutput::with_provider(
        background.clone(),
        inner.current_provider_name().to_owned(),
        None,
    );
    let agent = codelet_core::RigAgent::new(
        rig::agent::AgentBuilder::new(model).max_tokens(100).build(),
        1,
    );
    inner.bind_provider_persistence(background.id)?;
    inner.begin_provider_turn();
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        codelet_cli::interactive::run_agent_stream(
            agent,
            "next",
            &mut inner,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            std::sync::Arc::new(tokio::sync::Notify::new()),
            &output,
            background.id,
        ),
    )
    .await??;
    Ok(())
}

pub async fn next_anthropic_wire(
    manager: &SessionManager,
    session_id: &SessionId,
    _baseline: usize,
) -> Result<Value, Box<dyn std::error::Error>> {
    let server = MockServer::start().await;
    let start = json!({"type":"message_start","message":{
        "id":"msg_test","type":"message","role":"assistant","model":"test",
        "content":[],"usage":{"input_tokens":1,"output_tokens":0}
    }});
    let delta = json!({"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},
        "usage":{"output_tokens":1}});
    Mock::given(wiremock::matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            format!("event: message_start\ndata: {start}\n\nevent: message_delta\ndata: {delta}\n\nevent: message_stop\ndata: {{\"type\":\"message_stop\"}}\n\n"),
            "text/event-stream",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let client: rig::providers::anthropic::Client = rig::providers::anthropic::Client::builder()
        .api_key("test")
        .base_url(server.uri())
        .build()?;
    stream_session(
        manager,
        session_id,
        client.completion_model("claude-sonnet-4"),
    )
    .await?;
    let requests = server
        .received_requests()
        .await
        .ok_or("capture unavailable")?;
    Ok(requests.first().ok_or("no provider request")?.body_json()?)
}
