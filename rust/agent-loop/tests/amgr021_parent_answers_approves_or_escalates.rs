//! Feature: spec/features/a-parent-answers-approves-or-escalates-its-sub-agents-request.feature
//!
//! AMGR-021: a sub-agent's question or approval is held by its parent, which
//! learns about it from AgentManager await_idle (needs_input) or, when it is
//! not waiting, from a message that starts its next turn. The parent answers,
//! approves or escalates it with AgentManager; a holder that is interrupted
//! or closed passes it up the chain, ending with the user; the user can
//! always answer, and the holder is then told.
//!
//! AgentManager is driven the way the tool drives it: an action parsed from
//! the tool's JSON arguments, run by the session's sync or async handler,
//! and the result read as the JSON the model receives. Requests are raised
//! the way the per-turn handlers raise them (store the request, then block
//! in `wait_for_*_response` on a thread). No turn ever runs (no agent loop is
//! spawned), so a message to a parent stays queued where the test reads it.
//! HANG SAFETY: every session's pending request is cancelled before the test
//! returns.

use codelet_agent_loop::agent_manager_handler::{create_async_handler, create_handler};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{
    RequestHolder, RequestRoute, RequestStatus, SessionId, SessionStatus, StreamChunk,
};
use codelet_sessions::background_session::{BackgroundSession, IncomingMessage};
use codelet_sessions::SessionManager;
use codelet_tools::agent_manager::{
    AgentManagerAction, AgentManagerAsyncHandler, AgentManagerHandler,
};
use codelet_tools::request_user_input::{HitlOption, HitlQuestion, HitlRequest, HitlResponse};
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const MODEL: &str = "anthropic/claude-sonnet-4-5";

/// Cancels whatever a session still waits on so no waiting thread outlives
/// the test.
struct Release(Vec<Arc<BackgroundSession>>);

impl Drop for Release {
    fn drop(&mut self) {
        for session in &self.0 {
            session.cancel_pending_interactions();
        }
    }
}

/// The data directory is process-wide, so the tests (which run in parallel)
/// share one, set once and alive until the process exits (BUG-406).
fn data_directory() -> Result<(), String> {
    static DATA: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let data = codelet_test_helpers::shared_temp_subdirectory()?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(data)?;
        // Sessions select a model at creation; no request is ever sent.
        std::env::set_var("ANTHROPIC_API_KEY", "offline");
        Ok(())
    })
    .clone()
}

struct Fixture {
    manager: Arc<SessionManager>,
    sync: AgentManagerHandler,
    asynchronous: AgentManagerAsyncHandler,
    release: Release,
    project: tempfile::TempDir,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        // BUG-347 / BUG-449: never consult the host's ~/.fspec.
        codelet_test_helpers::isolated_user_directory()?;
        data_directory()?;
        let manager = SessionManager::new();
        let project = codelet_test_helpers::setup_temp_directory()?;
        Ok(Self {
            sync: create_handler(
                Some(manager.clone()),
                project.path().to_string_lossy().into_owned(),
                Some(MODEL.to_string()),
                None,
                None,
            ),
            asynchronous: create_async_handler(Some(manager.clone())),
            manager,
            release: Release(Vec::new()),
            project,
        })
    }

    /// A live session named `name`, spawned by `parent` when given (the
    /// spawner link AgentManager spawn registers).
    async fn session(
        &mut self,
        name: &str,
        parent: Option<&Arc<BackgroundSession>>,
    ) -> TestResult<Arc<BackgroundSession>> {
        let id = uuid::Uuid::new_v4();
        self.manager
            .create_session_with_id(
                &id.to_string(),
                MODEL,
                &self.project.path().to_string_lossy(),
                name,
            )
            .await?;
        if let Some(parent) = parent {
            self.manager.add_supervisor(id, parent.id)?;
        }
        let session = self.manager.get_session(&id.to_string())?;
        self.release.0.push(session.clone());
        Ok(session)
    }

    /// Run a sync AgentManager action given as the tool's JSON arguments,
    /// for `caller`; returns the JSON result the model receives.
    fn act(&self, caller: &BackgroundSession, arguments: Value) -> TestResult<Value> {
        let action: AgentManagerAction = serde_json::from_value(arguments)?;
        Ok(serde_json::to_value((self.sync)(action, caller.id))?)
    }

    /// Start await_idle for `caller` on `sessions` on its own task.
    fn await_idle(
        &self,
        caller: &BackgroundSession,
        sessions: &[&BackgroundSession],
        timeout: Option<u64>,
    ) -> TestResult<tokio::task::JoinHandle<Result<Value, String>>> {
        let ids: Vec<String> = sessions.iter().map(|s| s.id.to_string()).collect();
        let mut arguments = json!({"action": "await_idle", "session_id": ids});
        if let Some(timeout) = timeout {
            arguments["timeout"] = json!(timeout);
        }
        let action: AgentManagerAction = serde_json::from_value(arguments)?;
        let future = (self.asynchronous)(action, caller.id);
        Ok(tokio::spawn(async move {
            serde_json::to_value(future.await).map_err(|error| error.to_string())
        }))
    }
}

async fn awaited(handle: tokio::task::JoinHandle<Result<Value, String>>) -> TestResult<Value> {
    Ok(codelet_test_helpers::deadlock_guard("await_idle returns", handle).await???)
}

fn sid(session: &BackgroundSession) -> SessionId {
    SessionId::from(session.id.to_string())
}

fn auth_question() -> HitlRequest {
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

fn bash_approval() -> PauseState {
    PauseState {
        kind: PauseKind::Triple,
        tool_name: "Bash".into(),
        message: "Command requires approval".into(),
        details: Some("echo amgr021 git reset --hard".into()),
        tool_call_id: None,
    }
}

fn web_search_action() -> PauseState {
    PauseState {
        kind: PauseKind::Continue,
        tool_name: "WebSearch".into(),
        message: "Page loaded at https://example.com. Interact with the page, then press Enter to capture screenshot.".into(),
        details: None,
        tool_call_id: None,
    }
}

/// Raise `request` the way the per-turn HITL handler does and wait for it on
/// its own thread.
fn ask(
    session: &Arc<BackgroundSession>,
    request: HitlRequest,
) -> tokio::sync::oneshot::Receiver<HitlResponse> {
    session.set_hitl_request(Some(request));
    session.set_status(SessionStatus::Paused);
    let (answered, answer) = tokio::sync::oneshot::channel();
    let waiting = session.clone();
    std::thread::spawn(move || {
        let _ = answered.send(waiting.wait_for_hitl_response());
    });
    answer
}

/// Raise `state` the way the per-turn pause handler does and wait for it on
/// its own thread.
fn hold(
    session: &Arc<BackgroundSession>,
    state: PauseState,
) -> tokio::sync::oneshot::Receiver<PauseResponse> {
    session.set_pause_state(Some(state));
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

fn question_route(session: &BackgroundSession) -> TestResult<RequestRoute> {
    Ok(session
        .pending_question_route()
        .ok_or("the session has no pending question")?)
}

fn pause_route(session: &BackgroundSession) -> TestResult<RequestRoute> {
    Ok(session
        .pending_pause_route()
        .ok_or("the session has no pending approval")?)
}

/// The awaited session `session`'s entry in an await_idle result.
fn entry<'a>(result: &'a Value, session: &BackgroundSession) -> TestResult<&'a Value> {
    let id = session.id.to_string();
    Ok(result["results"]
        .as_array()
        .ok_or_else(|| format!("not an await result: {result}"))?
        .iter()
        .find(|entry| entry["session_id"] == json!(id))
        .ok_or_else(|| format!("no entry for {id} in {result}"))?)
}

fn holder(session: &BackgroundSession) -> Value {
    json!(session.id.to_string())
}

/// Every message queued for `session` (each would start one turn).
async fn messages(session: &BackgroundSession) -> Vec<IncomingMessage> {
    let mut queued = Vec::new();
    let mut receiver = session.incoming_message_rx.lock().await;
    while let Ok(message) = receiver.try_recv() {
        queued.push(message);
    }
    queued
}

/// The next request event for `request_id` on `chunks`, skipping others.
async fn request_event(
    chunks: &mut broadcast::Receiver<StreamChunk>,
    request_id: &str,
) -> TestResult<(RequestRoute, RequestStatus)> {
    let event = codelet_test_helpers::deadlock_guard("the request event arrives", async {
        loop {
            match chunks.recv().await {
                Ok(StreamChunk::InteractionRequestChanged { route, status })
                    if route.request_id == request_id =>
                {
                    return Ok((route, status));
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return Err("stream closed"),
            }
        }
    })
    .await??;
    Ok(event)
}

fn assert_rejected(result: &Value, what: &str) {
    assert_eq!(
        result["error"],
        json!(true),
        "{what} was not rejected: {result}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_await_idle_reports_a_sub_agents_question_and_the_parent_answers_it() -> TestResult
{
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" spawned sub-agent "A" and is awaiting it with no timeout
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    a.set_status(SessionStatus::Running);
    let waiting = fixture.await_idle(&r, &[&a], None)?;
    // @step When "A" asks "Which auth library?" with options "jose" and "jsonwebtoken"
    let answer = ask(&a, auth_question());
    let route = question_route(&a)?;
    // @step Then R's await_idle returns at once with "A" needs_input naming the request id, origin "A", kind question and the question with its options
    let result = awaited(waiting).await?;
    let a_entry = entry(&result, &a)?;
    assert_eq!(a_entry["status"], json!("needs_input"), "{result}");
    let request = &a_entry["request"];
    assert_eq!(request["request_id"], json!(route.request_id));
    assert_eq!(request["origin_session_id"], json!(a.id.to_string()));
    assert_eq!(request["kind"], json!("question"));
    assert_eq!(request["holder"], holder(&r));
    assert_eq!(
        request["questions"][0]["question"],
        json!("Which auth library?")
    );
    assert_eq!(
        request["questions"][0]["options"][0]["label"],
        json!("jose")
    );
    assert_eq!(
        request["questions"][0]["options"][1]["label"],
        json!("jsonwebtoken")
    );
    // @step When R calls AgentManager answer with that request id selecting "jose"
    let answered = fixture.act(
        &r,
        json!({
            "action": "answer",
            "request_id": route.request_id,
            "answers": [{"id": "auth_library", "selected": ["jose"]}],
        }),
    )?;
    assert_eq!(answered["answered"], json!(true), "{answered}");
    // @step Then A's tool receives the answer "jose"
    let HitlResponse::Answered { answers } = returned(answer).await? else {
        return Err("the question was not answered".into());
    };
    let chosen = answers
        .get("auth_library")
        .ok_or("no answer for auth_library")?;
    assert_eq!(chosen.selected, vec!["jose".to_string()]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_await_idle_reports_an_approval_and_the_rest_as_pending_and_the_parent_denies_it(
) -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" spawned sub-agents "A" and "S", "S" is running and R awaits both
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let s = fixture.session("S", Some(&r)).await?;
    a.set_status(SessionStatus::Running);
    s.set_status(SessionStatus::Running);
    let waiting = fixture.await_idle(&r, &[&a, &s], None)?;
    // @step When the blocklist holds A's Bash command "echo amgr021 git reset --hard" for approval
    let answer = hold(&a, bash_approval());
    let route = pause_route(&a)?;
    // @step Then R's await_idle returns "A" needs_input with kind approval, the tool, message and command, and "S" pending
    let result = awaited(waiting).await?;
    let a_entry = entry(&result, &a)?;
    assert_eq!(a_entry["status"], json!("needs_input"), "{result}");
    let request = &a_entry["request"];
    assert_eq!(request["request_id"], json!(route.request_id));
    assert_eq!(request["kind"], json!("approval"));
    assert_eq!(request["approval"]["tool_name"], json!("Bash"));
    assert_eq!(
        request["approval"]["message"],
        json!("Command requires approval")
    );
    assert_eq!(
        request["approval"]["details"],
        json!("echo amgr021 git reset --hard")
    );
    assert_eq!(entry(&result, &s)?["status"], json!("pending"), "{result}");
    // @step When R calls AgentManager approve with that request id and choice deny
    let approved = fixture.act(
        &r,
        json!({"action": "approve", "request_id": route.request_id, "choice": "deny"}),
    )?;
    assert_eq!(approved["answered"], json!(true), "{approved}");
    // @step Then A's command is denied
    assert_eq!(returned(answer).await?, PauseResponse::Denied);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_a_request_below_an_awaited_sub_agent_reaches_the_parent_when_it_is_escalated_to_it(
) -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" spawned "A", "A" spawned "B", and R awaits "A"
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let b = fixture.session("B", Some(&a)).await?;
    a.set_status(SessionStatus::Running);
    let mut a_chunks = a.subscribe_to_stream();
    let waiting = fixture.await_idle(&r, &[&a], None)?;
    // @step When "B" asks a question, which "A" holds
    let _answer = ask(&b, auth_question());
    let route = question_route(&b)?;
    assert_eq!(route.holder, RequestHolder::Session(sid(&a)));
    // @step Then R's await_idle has not returned when "A" escalates the question
    request_event(&mut a_chunks, &route.request_id).await?;
    assert!(
        !waiting.is_finished(),
        "await_idle returned for a request A holds"
    );
    let escalated = fixture.act(
        &a,
        json!({"action": "escalate", "request_id": route.request_id}),
    )?;
    assert_eq!(escalated["holder"], holder(&r), "{escalated}");
    // @step And R's await_idle then returns "A" needs_input for that request with origin "B" and holder R
    let result = awaited(waiting).await?;
    let a_entry = entry(&result, &a)?;
    assert_eq!(a_entry["status"], json!("needs_input"), "{result}");
    assert_eq!(a_entry["request"]["request_id"], json!(route.request_id));
    assert_eq!(
        a_entry["request"]["origin_session_id"],
        json!(b.id.to_string())
    );
    assert_eq!(a_entry["request"]["holder"], holder(&r));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_escalating_at_the_root_hands_the_request_to_the_user() -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given root bot "R" holds its sub-agent "A"'s question
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let mut r_chunks = r.subscribe_to_stream();
    let mut a_chunks = a.subscribe_to_stream();
    let _answer = ask(&a, auth_question());
    let route = question_route(&a)?;
    assert_eq!(route.holder, RequestHolder::Session(sid(&r)));
    // @step When R calls AgentManager escalate with its request id
    let escalated = fixture.act(
        &r,
        json!({"action": "escalate", "request_id": route.request_id}),
    )?;
    assert_eq!(escalated["holder"], json!("user"), "{escalated}");
    // @step Then the request's holder is the user and every session on its chain is told
    assert_eq!(question_route(&a)?.holder, RequestHolder::User);
    for chunks in [&mut a_chunks, &mut r_chunks] {
        let (raised, _) = request_event(chunks, &route.request_id).await?;
        assert_eq!(raised.holder, RequestHolder::Session(sid(&r)));
        let (moved, status) = request_event(chunks, &route.request_id).await?;
        assert_eq!(status, RequestStatus::Pending);
        assert_eq!(moved.holder, RequestHolder::User);
    }
    // @step And R's AgentManager answer naming it is then rejected as not from the holder
    let answered = fixture.act(
        &r,
        json!({"action": "answer", "request_id": route.request_id, "cancelled": true}),
    )?;
    assert_rejected(&answered, "R's answer after escalating");
    assert_eq!(question_route(&a)?.request_id, route.request_id);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_replies_that_are_not_the_holders_of_the_wrong_kind_or_stale_are_rejected(
) -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" holds sub-agent "A"'s question and "S" is A's sibling
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let s = fixture.session("S", Some(&r)).await?;
    let _answer = ask(&a, auth_question());
    let route = question_route(&a)?;
    // @step When S calls answer for it, R calls approve for it and R calls answer naming an unknown request id
    let sibling = fixture.act(
        &s,
        json!({"action": "answer", "request_id": route.request_id, "cancelled": true}),
    )?;
    let wrong_kind = fixture.act(
        &r,
        json!({"action": "approve", "request_id": route.request_id, "choice": "approve"}),
    )?;
    let unknown = fixture.act(
        &r,
        json!({"action": "answer", "request_id": "amgr021-unknown", "cancelled": true}),
    )?;
    // @step Then each call is rejected and A's question is still pending with holder R
    assert_rejected(&sibling, "the sibling's answer");
    assert_rejected(&wrong_kind, "an approval for a question");
    assert_rejected(&unknown, "an answer naming an unknown id");
    assert_eq!(question_route(&a)?, route);
    assert_eq!(route.holder, RequestHolder::Session(sid(&r)));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_a_human_action_is_held_by_the_user_and_the_parent_is_told_once() -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" spawned sub-agent "A" and is awaiting it
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    a.set_status(SessionStatus::Running);
    let waiting = fixture.await_idle(&r, &[&a], None)?;
    // @step When A's WebSearch asks for the page to be used and Enter pressed
    let answer = hold(&a, web_search_action());
    let route = pause_route(&a)?;
    // @step Then R's await_idle returns "A" waiting_on_user with the request id and holder the user
    let result = awaited(waiting).await?;
    let a_entry = entry(&result, &a)?;
    assert_eq!(a_entry["status"], json!("waiting_on_user"), "{result}");
    assert_eq!(a_entry["request"]["request_id"], json!(route.request_id));
    assert_eq!(a_entry["request"]["kind"], json!("human_action"));
    assert_eq!(a_entry["request"]["holder"], json!("user"));
    // @step And R's approve naming it is rejected
    let approved = fixture.act(
        &r,
        json!({"action": "approve", "request_id": route.request_id, "choice": "approve"}),
    )?;
    assert_rejected(&approved, "a parent's approve of a human action");
    assert_eq!(pause_route(&a)?, route);
    // @step And R's next await_idle on "A" with timeout 1 times out instead of reporting it again
    let result = awaited(fixture.await_idle(&r, &[&a], Some(1))?).await?;
    assert_eq!(
        entry(&result, &a)?["status"],
        json!("timed_out"),
        "{result}"
    );
    assert_eq!(pause_route(&a)?, route);
    a.cancel_pending_interactions();
    assert_eq!(returned(answer).await?, PauseResponse::Interrupted);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_an_interrupted_or_closed_holder_passes_its_requests_up_the_chain() -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" spawned "A", "A" spawned "B", and "A" holds "B"'s question
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let b = fixture.session("B", Some(&a)).await?;
    let _answer = ask(&b, auth_question());
    let route = question_route(&b)?;
    assert_eq!(route.holder, RequestHolder::Session(sid(&a)));
    assert!(
        messages(&r).await.is_empty(),
        "R was told about a request A holds"
    );
    // @step When "A" is interrupted
    a.interrupt();
    // @step Then R holds the question and is told by a message naming the request
    assert_eq!(question_route(&b)?.holder, RequestHolder::Session(sid(&r)));
    assert_eq!(question_route(&b)?.request_id, route.request_id);
    let told = messages(&r).await;
    assert_eq!(told.len(), 1, "R was not told once");
    assert_eq!(told[0].source_session_id, b.id.to_string());
    assert!(
        told[0].message.contains(&route.request_id),
        "{}",
        told[0].message
    );
    // @step When R is closed
    fixture.manager.destroy_session(&r.id.to_string())?;
    // @step Then the user holds the question
    assert_eq!(question_route(&b)?.holder, RequestHolder::User);
    assert_eq!(question_route(&b)?.request_id, route.request_id);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_a_parent_that_is_not_waiting_is_told_by_a_message_and_still_holds_the_request(
) -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" spawned sub-agent "A" and R is idle, not in await_idle
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    // @step When "A" asks "Which auth library?"
    let _answer = ask(&a, auth_question());
    let route = question_route(&a)?;
    // @step Then R receives a message from "A" that starts its next turn, naming the request id, the question and the AgentManager answer and escalate actions
    let told = messages(&r).await;
    assert_eq!(told.len(), 1, "R was not told once");
    let message = &told[0];
    assert_eq!(message.source_session_id, a.id.to_string());
    assert!(
        message.message.contains(&route.request_id),
        "{}",
        message.message
    );
    assert!(
        message.message.contains("Which auth library?"),
        "{}",
        message.message
    );
    assert!(
        message.message.contains("AgentManager answer"),
        "{}",
        message.message
    );
    assert!(message.message.contains("escalate"), "{}", message.message);
    // @step And R still holds the request
    assert_eq!(question_route(&a)?, route);
    assert_eq!(route.holder, RequestHolder::Session(sid(&r)));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_the_user_takes_a_request_over_and_the_holder_is_told() -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" holds sub-agent "A"'s question "Which auth library?"
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let answer = ask(&a, auth_question());
    let route = question_route(&a)?;
    // @step When the user answers it "jose" from A's tab
    SessionManagerHandle::send_hitl_response(
        &*fixture.manager,
        &sid(&a),
        &route.request_id,
        codelet_rpc_types::HitlResponse {
            cancelled: false,
            answers: vec![codelet_rpc_types::HitlAnswer {
                id: "auth_library".into(),
                selected: vec!["jose".into()],
                other: None,
            }],
        },
    )?;
    // @step Then A's tool receives "jose"
    let HitlResponse::Answered { answers } = returned(answer).await? else {
        return Err("the question was not answered".into());
    };
    let chosen = answers
        .get("auth_library")
        .ok_or("no answer for auth_library")?;
    assert_eq!(chosen.selected, vec!["jose".to_string()]);
    // @step And R receives a message saying the user answered the request with "jose"
    let told = messages(&r).await;
    let answered = told
        .iter()
        .find(|message| message.message.contains("user answered"))
        .ok_or("R was not told that the user answered")?;
    assert!(
        answered.message.contains(&route.request_id),
        "{}",
        answered.message
    );
    assert!(answered.message.contains("jose"), "{}", answered.message);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_a_sub_agent_with_no_live_parent_sends_its_request_to_the_user() -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given session "A" whose spawner link is gone, as after a daemon restart
    let a = fixture.session("A", None).await?;
    let gone = uuid::Uuid::new_v4();
    fixture.manager.add_supervisor(a.id, gone)?;
    // @step When "A" asks a question
    let _answer = ask(&a, auth_question());
    // @step Then the user holds the request
    assert_eq!(question_route(&a)?.holder, RequestHolder::User);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_get_status_and_list_show_a_sessions_pending_request() -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" holds sub-agent "A"'s question
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let _answer = ask(&a, auth_question());
    let route = question_route(&a)?;
    // @step When R calls AgentManager get_status for "A" and list
    let status = fixture.act(
        &r,
        json!({"action": "get_status", "session_id": a.id.to_string()}),
    )?;
    let listed = fixture.act(&r, json!({"action": "list"}))?;
    // @step Then both show A's pending request with its id, kind question, origin "A" and holder R
    let listed_a = listed["sessions"]
        .as_array()
        .ok_or_else(|| format!("not a list: {listed}"))?
        .iter()
        .find(|entry| entry["session_id"] == json!(a.id.to_string()))
        .ok_or_else(|| format!("A is not listed: {listed}"))?;
    for request in [&status["pending_request"], &listed_a["pending_request"]] {
        assert_eq!(
            request["request_id"],
            json!(route.request_id),
            "{status} {listed}"
        );
        assert_eq!(request["kind"], json!("question"));
        assert_eq!(request["origin_session_id"], json!(a.id.to_string()));
        assert_eq!(request["holder"], holder(&r));
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_an_await_idle_started_after_the_request_was_raised_reports_needs_input_not_timed_out(
) -> TestResult {
    let mut fixture = Fixture::new()?;
    // @step Given bot "R" holds sub-agent "A"'s question, raised while R was not in await_idle
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let _answer = ask(&a, auth_question());
    let route = question_route(&a)?;
    // @step When R calls await_idle on "A" with timeout 1
    let result = awaited(fixture.await_idle(&r, &[&a], Some(1))?).await?;
    // @step Then the result is "A" needs_input for that request, not timed_out
    let a_entry = entry(&result, &a)?;
    assert_eq!(a_entry["status"], json!("needs_input"), "{result}");
    assert_eq!(a_entry["request"]["request_id"], json!(route.request_id));
    Ok(())
}

#[test]
fn scenario_the_bot_role_answers_approves_or_escalates_instead_of_messaging_a_paused_sub_agent() {
    // @step Given the orchestrator bot role text
    // @step When a new bot is created with it
    let role = codelet_sessions::bot::ORCHESTRATOR_ROLE;
    // @step Then it tells the bot to use AgentManager answer, approve or escalate when await_idle reports needs_input
    for needed in ["needs_input", "AgentManager answer", "approve", "escalate"] {
        assert!(role.contains(needed), "the bot role does not name {needed}");
    }
    // @step And it no longer tells the bot to answer a sub-agent with AgentManager message
    assert!(
        !role.contains("message to steer or answer"),
        "the bot role still tells the bot to answer with message"
    );
}
