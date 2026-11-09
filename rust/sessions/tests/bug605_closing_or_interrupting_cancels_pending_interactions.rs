//! Feature: spec/features/closing-or-interrupting-a-session-cancels-its-pending-question-or-approval.feature
//!
//! BUG-605: a session waiting on a question (request_user_input) or an
//! approval (a blocklist pause) blocks its turn until an answer arrives.
//! Interrupting or closing the session must answer that wait with its exact
//! cancelled outcome so the turn finishes.
//!
//! The first three scenarios wait the way the per-turn handlers do (store
//! the request, then block in wait_for_*_response on a thread); the last
//! three drive a real turn: a SessionManager with FspecAgentHooks runs the
//! agent loop against an offline OpenAI-compatible endpoint (see [`reply`]).
//! Reproduced before the fix: the turn ends on interrupt/close (the stream
//! loop drops the running tool), but the tool's wait is never answered and
//! the question or approval stays pending. HANG SAFETY: a blocked turn holds
//! a tokio worker in a std mpsc recv, and runtime shutdown would wait on it
//! forever, so every scenario keeps a [`Release`] that answers whatever is
//! still pending (cancelling it) before the test returns.

use codelet_rpc_types::SessionStatus;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::request_user_input::{HitlAnswer, HitlResponse};
use codelet_tools::request_user_input::{HitlOption, HitlQuestion, HitlRequest};
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const QUESTION_CALL: &str = "bug605-question";
const BASH_CALL: &str = "bug605-bash";
/// Matches the default blocklist's `git-hard-reset-prompt` rule; harmless if run.
const PROMPTED_COMMAND: &str = "echo bug605 git reset --hard";

fn question_call() -> Value {
    let arguments = json!({"work_unit_id": "NONE", "questions": [{
        "id": "auth_library",
        "header": "Auth",
        "question": "Which auth library?",
        "options": [
            {"label": "jose", "description": "Use jose."},
            {"label": "jsonwebtoken", "description": "Use jsonwebtoken."}
        ]
    }]});
    tool_call(QUESTION_CALL, "request_user_input", arguments)
}

fn bash_call() -> Value {
    let arguments = json!({"work_unit_id": "NONE", "command": PROMPTED_COMMAND});
    tool_call(BASH_CALL, "Bash", arguments)
}

fn tool_call(id: &str, name: &str, arguments: Value) -> Value {
    json!({"role": "assistant", "tool_calls": [{"index": 0, "id": id, "type": "function",
        "function": {"name": name, "arguments": arguments.to_string()}}]})
}

fn sse(delta: Value) -> ResponseTemplate {
    let finish = if delta.get("tool_calls").is_some() {
        "tool_calls"
    } else {
        "stop"
    };
    let chunk = json!({"id": "bug605", "object": "chat.completion.chunk", "created": 1,
        "model": "o3", "choices": [{"index": 0, "delta": delta, "finish_reason": finish}],
        "usage": {"prompt_tokens": 50, "completion_tokens": 5, "total_tokens": 55}});
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
}

/// The scripted provider: a turn asked to "pick a library" calls
/// request_user_input, one asked to "reset the tree" runs the prompted Bash
/// command, and a request carrying a tool result gets plain text.
fn reply(body: &[u8]) -> ResponseTemplate {
    let body: Value = serde_json::from_slice(body).unwrap_or_default();
    let messages = body["messages"].as_array().cloned().unwrap_or_default();
    if messages.last().is_some_and(|last| last["role"] == "tool") {
        return sse(json!({"role": "assistant", "content": "finished"}));
    }
    let asked = messages
        .iter()
        .rev()
        .filter(|message| message["role"] == "user")
        .map(|message| message["content"].to_string())
        .find(|text| text.contains("pick a library") || text.contains("reset the tree"));
    match asked {
        Some(text) if text.contains("pick a library") => sse(question_call()),
        Some(_) => sse(bash_call()),
        None => sse(json!({"role": "assistant", "content": "finished"})),
    }
}

/// Restores the environment variables a scenario changed.
struct Environment(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Environment {
    fn set(values: &[(&'static str, std::ffi::OsString)]) -> Self {
        let previous = values
            .iter()
            .map(|(name, _)| (*name, std::env::var_os(name)))
            .collect();
        for (name, value) in values {
            std::env::set_var(name, value);
        }
        Self(previous)
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (name, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// Answers whatever the session still waits on, so a blocked turn never
/// keeps the runtime from shutting down (see the module docs).
struct Release(Arc<BackgroundSession>);

impl Drop for Release {
    fn drop(&mut self) {
        self.0.cancel_pending_interactions();
    }
}

struct Fixture {
    manager: Arc<SessionManager>,
    session: Arc<BackgroundSession>,
    chunks: broadcast::Receiver<StreamChunk>,
    _release: Release,
    _server: MockServer,
    _environment: Environment,
    _home: tempfile::TempDir,
    _project: tempfile::TempDir,
    _data: tempfile::TempDir,
}

impl Fixture {
    /// A live session over the scripted endpoint (see [`reply`]).
    async fn new() -> TestResult<Self> {
        // BUG-347 / BUG-449: never consult the host's ~/.fspec or RLCD.
        codelet_test_helpers::isolated_user_directory()?;
        let data = codelet_test_helpers::setup_temp_directory()?;
        std::fs::create_dir_all(data.path().join("cache"))?;
        std::fs::write(
            data.path().join("cache/models.json"),
            include_str!("fixtures/rpc422_models.json"),
        )?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(data.path().to_owned())?;
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(|request: &wiremock::Request| reply(&request.body))
            .mount(&server)
            .await;
        // The default blocklist is installed into this HOME's ~/.fspec.
        let home = codelet_test_helpers::setup_temp_directory()?;
        let environment = Environment::set(&[
            ("ANTHROPIC_API_KEY", "offline".into()),
            ("OPENAI_API_KEY", "offline".into()),
            ("OPENAI_BASE_URL", server.uri().into()),
            ("HOME", home.path().as_os_str().to_owned()),
        ]);
        let project = codelet_test_helpers::setup_temp_directory()?;
        let manager = SessionManager::new();
        manager.set_hooks(Arc::new(codelet_agent_loop::FspecAgentHooks::new()));
        let id = uuid::Uuid::new_v4().to_string();
        manager
            .create_session_with_id(
                &id,
                "anthropic/claude-sonnet-4",
                &project.path().to_string_lossy(),
                "bug605",
            )
            .await?;
        let session = manager.get_session(&id)?;
        session
            .inner
            .lock()
            .await
            .provider_manager_mut()
            .set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
        let chunks = session.subscribe_to_stream();
        Ok(Self {
            manager,
            _release: Release(session.clone()),
            session,
            chunks,
            _server: server,
            _environment: environment,
            _home: home,
            _project: project,
            _data: data,
        })
    }

    async fn wait_for_question(&self) -> TestResult {
        let session = self.session.clone();
        codelet_test_helpers::wait_until("the session waits on the question", move || {
            std::future::ready(session.get_hitl_request().is_some())
        })
        .await?;
        Ok(())
    }

    async fn wait_for_approval(&self) -> TestResult {
        let session = self.session.clone();
        codelet_test_helpers::wait_until("the session waits on the approval", move || {
            std::future::ready(session.get_pause_state().is_some())
        })
        .await?;
        Ok(())
    }

    /// Chunks until the turn's Done; fails when the turn never finishes.
    async fn until_done(&mut self) -> TestResult<Vec<StreamChunk>> {
        let chunks = &mut self.chunks;
        let seen = codelet_test_helpers::deadlock_guard("the turn finishes", async {
            let mut seen = Vec::new();
            loop {
                match chunks.recv().await {
                    Ok(StreamChunk::Done) => return seen,
                    Ok(chunk) => seen.push(chunk),
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return seen,
                }
            }
        })
        .await?;
        Ok(seen)
    }
}

/// The result content of tool call `id` among `chunks`.
fn tool_result(chunks: &[StreamChunk], id: &str) -> Option<String> {
    chunks.iter().find_map(|chunk| match chunk {
        StreamChunk::ToolResult { tool_result, .. } if tool_result.tool_call_id == id => {
            Some(tool_result.content.clone())
        }
        _ => None,
    })
}

fn json_result(chunks: &[StreamChunk], id: &str) -> TestResult<Value> {
    let content = tool_result(chunks, id).ok_or_else(|| format!("no result for {id}"))?;
    // The tool's JSON output reaches the stream as a JSON string.
    match serde_json::from_str(&content)? {
        Value::String(output) => Ok(serde_json::from_str(&output)?),
        value => Ok(value),
    }
}

fn question() -> HitlRequest {
    HitlRequest {
        questions: vec![HitlQuestion {
            id: "auth_library".into(),
            header: "Auth".into(),
            question: "Which auth library?".into(),
            options: Some(vec![
                HitlOption {
                    label: "jose".into(),
                    description: "Use jose.".into(),
                },
                HitlOption {
                    label: "jsonwebtoken".into(),
                    description: "Use jsonwebtoken.".into(),
                },
            ]),
        }],
    }
}

fn approval() -> PauseState {
    PauseState {
        kind: PauseKind::Triple,
        tool_name: "Bash".into(),
        message: "Command requires approval".into(),
        details: Some(PROMPTED_COMMAND.into()),
        tool_call_id: None,
    }
}

/// Raise `question` the way the per-turn HITL handler does and wait for it
/// on its own thread; the receiver yields what the wait returned.
fn ask_question(session: &Arc<BackgroundSession>) -> tokio::sync::oneshot::Receiver<HitlResponse> {
    session.set_hitl_request(Some(question()));
    session.set_status(SessionStatus::Paused);
    let (answered, answer) = tokio::sync::oneshot::channel();
    let waiting = session.clone();
    std::thread::spawn(move || {
        let _ = answered.send(waiting.wait_for_hitl_response());
    });
    answer
}

/// Raise `approval` the way the per-turn pause handler does and wait for it
/// on its own thread; the receiver yields what the wait returned.
fn ask_approval(session: &Arc<BackgroundSession>) -> tokio::sync::oneshot::Receiver<PauseResponse> {
    session.set_pause_state(Some(approval()));
    let (answered, answer) = tokio::sync::oneshot::channel();
    let waiting = session.clone();
    std::thread::spawn(move || {
        let _ = answered.send(waiting.wait_for_pause_response());
    });
    answer
}

async fn returned<T>(answer: tokio::sync::oneshot::Receiver<T>) -> TestResult<T> {
    Ok(codelet_test_helpers::deadlock_guard("the wait returns", answer).await??)
}

fn idle_after_turn(session: &BackgroundSession) -> bool {
    !matches!(
        session.get_status(),
        SessionStatus::Paused | SessionStatus::Running
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn interrupting_a_session_waiting_on_a_question_answers_it_cancelled() -> TestResult {
    // @step Given a session waiting for the answer to a question
    let fixture = Fixture::new().await?;
    let answer = ask_question(&fixture.session);

    // @step When the session is interrupted
    fixture.session.interrupt();

    // @step Then the wait returns {"cancelled": true}
    assert_eq!(
        returned(answer).await?,
        HitlResponse::Cancelled { cancelled: true }
    );

    // @step And the session has no pending question
    assert_eq!(fixture.session.get_hitl_request(), None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn interrupting_a_session_waiting_on_an_approval_answers_it_interrupted() -> TestResult {
    // @step Given a session waiting for the answer to an approval
    let fixture = Fixture::new().await?;
    let answer = ask_approval(&fixture.session);

    // @step When the session is interrupted
    fixture.session.interrupt();

    // @step Then the wait returns Interrupted
    assert_eq!(returned(answer).await?, PauseResponse::Interrupted);

    // @step And the session has no pending approval
    assert!(fixture.session.get_pause_state().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn a_question_or_approval_raised_after_the_session_was_interrupted_resolves_at_once(
) -> TestResult {
    // @step Given a session that was interrupted and has not started a new turn
    let fixture = Fixture::new().await?;
    fixture.session.interrupt();

    // @step When it asks a question
    let answer = ask_question(&fixture.session);

    // @step Then the wait returns {"cancelled": true} without anyone answering
    assert_eq!(
        returned(answer).await?,
        HitlResponse::Cancelled { cancelled: true }
    );

    // @step When it asks for an approval
    let answer = ask_approval(&fixture.session);

    // @step Then the wait returns Interrupted without anyone answering
    assert_eq!(returned(answer).await?, PauseResponse::Interrupted);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn closing_a_session_whose_turn_waits_on_an_approval_ends_the_turn_with_nothing_pending(
) -> TestResult {
    // @step Given a session whose turn runs Bash "echo bug605 git reset --hard" and is waiting for the blocklist approval
    let mut fixture = Fixture::new().await?;
    fixture.session.send_input("reset the tree".into(), None)?;
    fixture.wait_for_approval().await?;

    // @step When the session is closed
    fixture
        .manager
        .destroy_session(&fixture.session.id.to_string())?;

    // @step Then the session's turn finishes
    fixture.until_done().await?;

    // @step And the session has no pending approval and is neither paused nor running
    assert!(fixture.session.get_pause_state().is_none());
    assert!(
        idle_after_turn(&fixture.session),
        "{:?}",
        fixture.session.get_status()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn interrupting_a_session_whose_turn_waits_on_a_question_ends_the_turn_with_nothing_pending(
) -> TestResult {
    // @step Given a session whose turn called request_user_input and is waiting for the answer
    let mut fixture = Fixture::new().await?;
    fixture.session.send_input("pick a library".into(), None)?;
    fixture.wait_for_question().await?;

    // @step When the session is interrupted
    fixture.session.interrupt();

    // @step Then the session's turn finishes
    fixture.until_done().await?;

    // @step And the session has no pending question and is neither paused nor running
    assert_eq!(fixture.session.get_hitl_request(), None);
    assert!(
        idle_after_turn(&fixture.session),
        "{:?}",
        fixture.session.get_status()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn interrupting_a_session_that_is_not_waiting_leaves_no_answer_behind() -> TestResult {
    // @step Given a session that was interrupted while it was not waiting on a question or approval
    let mut fixture = Fixture::new().await?;
    fixture.session.interrupt();

    // @step When its next turn calls request_user_input
    fixture.session.send_input("pick a library".into(), None)?;

    // @step Then the session waits for the answer
    fixture.wait_for_question().await?;
    let request_id = fixture
        .session
        .pending_question_route()
        .ok_or("no question id")?
        .request_id;

    // @step And the answer the user gives is the request_user_input tool result
    let answer = HitlResponse::Answered {
        answers: HashMap::from([(
            "auth_library".to_string(),
            HitlAnswer {
                selected: vec!["jose".into()],
                other: None,
            },
        )]),
    };
    fixture.session.answer_question(
        &request_id,
        &codelet_rpc_types::RequestHolder::User,
        answer,
    )?;
    let chunks = fixture.until_done().await?;
    assert_eq!(
        json_result(&chunks, QUESTION_CALL)?,
        json!({"answers": {"auth_library": {"selected": ["jose"], "other": null}}})
    );
    Ok(())
}
