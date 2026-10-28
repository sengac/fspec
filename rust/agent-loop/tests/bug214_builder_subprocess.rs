#[path = "../../sessions/tests/cmpct060/support.rs"]
pub mod session_support;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use codelet_agent_loop::generate_compaction_handler::register_compactor_sub_agent_handler;
use codelet_agent_loop::BackgroundOutput;
use codelet_cli::compaction_operation::CompactionFailure;
use codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler;
use codelet_cli::interactive::run_agent_stream;
use codelet_core::{
    persistence::{append_message, load_session},
    RigAgent,
};
use codelet_rpc_types::StreamChunk;
use codelet_tools::unified_exec::{global_store, ChildHandle, UnifiedExecArgs, UnifiedExecTool};
use codelet_tools::BashTool;
use rig::{client::CompletionClient, message::Message, tool::Tool};
use serde_json::json;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

struct Cleanup {
    environment: Vec<(&'static str, Option<std::ffi::OsString>)>,
    children: Vec<ChildHandle>,
    session: uuid::Uuid,
}

async fn process_alive(pid: &str) -> Result<bool, std::io::Error> {
    Ok(tokio::process::Command::new("kill")
        .args(["-0", pid.trim()])
        .stderr(std::process::Stdio::null())
        .status()
        .await?
        .success())
}

impl Cleanup {
    fn new(session: uuid::Uuid) -> Self {
        Self {
            environment: ["OPENAI_API_KEY", "OPENAI_BASE_URL", "OPENAI_MODEL"]
                .into_iter()
                .map(|key| (key, std::env::var_os(key)))
                .collect(),
            children: Vec::new(),
            session,
        }
    }

    async fn track(&mut self, process: &str) -> session_support::TestResult {
        self.children.push(
            global_store()
                .kill_handle(process)
                .await
                .ok_or("process kill handle")?,
        );
        Ok(())
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        for child in &self.children {
            child.kill();
        }
        set_compactor_sub_agent_handler(self.session, None);
        for (key, previous) in &self.environment {
            match previous {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_original_session_cancellation_terminates_only_its_compaction_builder_subprocesses(
) -> session_support::TestResult {
    // @step Given automatic compaction has a builder running a long-lived subprocess under an ephemeral identity
    let (directory, manager, session_id, _) = session_support::manager().await?;
    let session = manager.get_session(&session_id.value)?;
    let mut cleanup = Cleanup::new(session.id);
    let server = MockServer::start().await;
    std::env::set_var("OPENAI_API_KEY", "offline-builder");
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    std::env::set_var("OPENAI_MODEL", "o3");
    let marker = format!("b214-{}", uuid::Uuid::new_v4());
    let pid_file = directory.path().join("builder.pid");
    let quoted_path = pid_file.to_string_lossy().replace('\'', "'\\''");
    let command = format!(": {marker}; printf '%s' \"$$\" > '{quoted_path}'; exec sleep 60");
    let arguments = serde_json::to_string(&json!({"command":command}))?;
    let requests = Arc::new(AtomicUsize::new(0));
    let received = requests.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            received.fetch_add(1, Ordering::AcqRel);
            ResponseTemplate::new(200).set_body_json(json!({
                "id":"builder-bash", "object":"chat.completion", "created":1,
                "model":"o3", "usage":{"prompt_tokens":100,"completion_tokens":20,"total_tokens":120},
                "choices":[{"index":0,"finish_reason":"tool_calls","message":{
                    "role":"assistant","content":null,"tool_calls":[{
                        "id":"pending-command","type":"function",
                        "function":{"name":BashTool::NAME,"arguments":arguments}
                    }]
                }}]
            }))
        })
        .mount(&server).await;
    let mut manifest = load_session(session.id)?;
    append_message(&mut manifest, "user", "unfinished task")?;
    append_message(&mut manifest, "assistant", "working")?;
    let durable = serde_json::to_value(&manifest)?;
    {
        let mut inner = session.inner.lock().await;
        inner.messages = vec![
            Message::user("unfinished task"),
            Message::assistant("working"),
        ];
        inner.provider_manager_mut().set_model_direct(
            "openai",
            "o3",
            Some(16_000),
            Some(4096),
            None,
        )?;
        inner.token_tracker.input_tokens = 15_000;
        register_compactor_sub_agent_handler(
            session.id,
            &inner,
            directory.path().to_owned(),
            session.compaction_in_progress.clone(),
        );
    }
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .base_url(server.uri())
            .build()?;
    let agent = RigAgent::new(client.agent("o3").max_tokens(4096).build(), 1);
    let background = session.clone();
    let operation = tokio::spawn(async move {
        let output = BackgroundOutput::with_provider(background.clone(), "openai".into(), None);
        let mut inner = background.inner.lock().await;
        run_agent_stream(
            agent,
            "resume original task",
            &mut inner,
            background.is_interrupted.clone(),
            background.compaction_in_progress.clone(),
            background.interrupt_notify.clone(),
            &output,
            background.id,
        )
        .await
    });
    let builder_process = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(process) = global_store()
                .list_sessions()
                .await
                .into_iter()
                .find(|process| process.command.contains(&marker))
            {
                break process.session_id;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    cleanup.track(&builder_process).await?;
    let builder_pid = std::fs::read_to_string(pid_file)?;
    assert!(process_alive(&builder_pid).await?);
    assert!(matches!(
        global_store().try_wait(&builder_process).await,
        Some(None)
    ));
    // @step And an unrelated owner has a running subprocess
    let unrelated = UnifiedExecTool::new(uuid::Uuid::new_v4())
        .call(UnifiedExecArgs(json!({
            "action":"run", "command":["sleep","60"], "yield_time_ms":250, "skip_blocklist":true
        })))
        .await?;
    let unrelated_process = unrelated.session_id.ok_or("unrelated subprocess session")?;
    cleanup.track(&unrelated_process).await?;
    // @step When the original session is interrupted while its builder is pending
    session.interrupt();
    let outcome = tokio::time::timeout(Duration::from_secs(3), operation).await;
    let terminated = tokio::time::timeout(Duration::from_secs(2), async {
        while process_alive(&builder_pid).await.unwrap_or(true) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .is_ok();
    let unrelated_running = matches!(
        global_store().try_wait(&unrelated_process).await,
        Some(None)
    );
    global_store().close_session(&builder_process).await?;
    global_store().close_session(&unrelated_process).await?;
    cleanup.children.clear();
    // @step Then the compaction operation is cancelled and its builder subprocess terminates promptly
    let result = outcome??;
    let failure = result.err().ok_or("compaction must be cancelled")?;
    assert!(
        matches!(
            failure.downcast_ref::<CompactionFailure>(),
            Some(CompactionFailure::Cancelled)
        ),
        "{failure}"
    );
    assert!(terminated, "cancelled builder left its ephemeral subprocess alive; explicit test teardown was required");
    // @step And the unrelated owner's subprocess remains running
    assert!(unrelated_running);
    // @step And no compaction success or continuation inference occurs
    assert_eq!(requests.load(Ordering::Acquire), 1);
    assert!(!session
        .get_buffered_output(1000)
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    assert_eq!(serde_json::to_value(load_session(session.id)?)?, durable);
    Ok(())
}
