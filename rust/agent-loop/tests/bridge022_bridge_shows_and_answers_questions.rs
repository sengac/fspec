//! Feature: spec/features/the-bridge-shows-and-answers-questions.feature
//!
//! BRIDGE-022: a session's bridge event for its own pending question carries
//! the question text, and the bridge control action `question_response`
//! answers it as the user, naming its request id. The control envelope goes
//! through `handle_multiplexed_inbound`, which calls the session's control
//! handler inline, so when the call returns the handler has finished
//! (TEST-042 causality); a rejected reply is proved to answer nothing by the
//! first answer the wait gets afterwards.

use codelet_rpc_types::{RequestHolder, RequestRoute, RequestStatus, StreamChunk};
use codelet_sessions::background_session::{BackgroundSession, IncomingMessage};
use codelet_sessions::SessionManager;
use codelet_tools::request_user_input::{
    HitlAnswer, HitlOption, HitlQuestion, HitlRequest, HitlResponse,
};
use codelet_tools::tool_pause::{PauseKind, PauseState};
use std::sync::Arc;
use tokio::sync::oneshot;

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
/// share one, set once (BUG-406).
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
    release: Release,
    project: tempfile::TempDir,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        codelet_test_helpers::isolated_user_directory()?;
        data_directory()?;
        Ok(Self {
            manager: SessionManager::new(),
            release: Release(Vec::new()),
            project: codelet_test_helpers::setup_temp_directory()?,
        })
    }

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

fn jose() -> serde_json::Value {
    serde_json::json!({"answers": {"auth_library": {"selected": ["jose"], "other": null}}})
}

/// Raise the question on `session` and wait for its answer on a thread.
fn ask(
    session: &Arc<BackgroundSession>,
) -> TestResult<(RequestRoute, oneshot::Receiver<HitlResponse>)> {
    session.set_hitl_request(Some(question()));
    let (answered, answer) = oneshot::channel();
    let waiting = session.clone();
    std::thread::spawn(move || {
        let _ = answered.send(waiting.wait_for_hitl_response());
    });
    let route = session
        .pending_question_route()
        .ok_or("the session has no pending question")?;
    Ok((route, answer))
}

/// Send a bridge control envelope for `session`.
async fn control(session: &Arc<BackgroundSession>, data: serde_json::Value) -> TestResult {
    let envelope = serde_json::json!({
        "service": "session",
        "type": "control",
        "session_id": session.id.to_string(),
        "data": data,
    });
    let injector: codelet_tools::InputInjector = Arc::new(|_| {});
    codelet_tools::handle_multiplexed_inbound(
        &envelope.to_string(),
        session.id,
        injector,
        Some(codelet_agent_loop::bridge_control::control_handler(
            session.clone(),
        )),
        None,
        None,
    )
    .await?;
    Ok(())
}

async fn messages(session: &BackgroundSession) -> Vec<IncomingMessage> {
    let mut queued = Vec::new();
    let mut receiver = session.incoming_message_rx.lock().await;
    while let Ok(message) = receiver.try_recv() {
        queued.push(message);
    }
    queued
}

/// Scenario: A session's question event on the bridge carries the question text
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_question_event_on_the_bridge_carries_the_question_text() -> TestResult {
    let mut fixture = Fixture::new()?;
    let session = fixture.session("bridge022-event", None).await?;
    let mut events = session.supervisor_broadcast.subscribe();

    // @step Given a session with a bridge asks "Which auth library?" with options jose and jsonwebtoken
    let (route, _answer) = ask(&session)?;

    // @step When the bridge forwards its interactionRequestChanged Pending event
    let chunk = codelet_test_helpers::deadlock_guard("the Pending event arrives", async {
        loop {
            if let chunk @ StreamChunk::InteractionRequestChanged {
                status: RequestStatus::Pending,
                ..
            } = events.recv().await?
            {
                return Ok::<_, Box<dyn std::error::Error>>(chunk);
            }
        }
    })
    .await??;
    let event = codelet_agent_loop::bridge_control::bridge_event_json(&session, &chunk);

    // @step Then the event's request holds the question "Which auth library?" with the option labels jose and jsonwebtoken
    assert_eq!(event["type"], "interactionRequestChanged");
    assert_eq!(event["route"]["request_id"], route.request_id);
    let asked = &event["request"]["questions"][0];
    assert_eq!(asked["id"], "auth_library");
    assert_eq!(asked["question"], "Which auth library?");
    assert_eq!(asked["options"][0]["label"], "jose");
    assert_eq!(asked["options"][1]["label"], "jsonwebtoken");
    Ok(())
}

/// Scenario: question_response answers the question named by its request id
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn question_response_answers_the_question_named_by_its_id() -> TestResult {
    let mut fixture = Fixture::new()?;
    let session = fixture.session("bridge022-answer", None).await?;

    // @step Given a session waiting on its question
    let (route, answer) = ask(&session)?;

    // @step When the bridge sends question_response with that request id and auth_library selected jose
    control(
        &session,
        serde_json::json!({
            "action": "question_response",
            "request_id": route.request_id,
            "response": jose(),
        }),
    )
    .await?;

    // @step Then the question's wait returns the answer jose
    let response =
        codelet_test_helpers::deadlock_guard("the question is answered", answer).await??;
    let HitlResponse::Answered { answers } = response else {
        return Err(format!("expected answers, got {response:?}").into());
    };
    assert_eq!(
        answers.get("auth_library"),
        Some(&HitlAnswer {
            selected: vec!["jose".into()],
            other: None,
        })
    );
    assert!(session.pending_question_route().is_none());
    Ok(())
}

/// Scenario: A question_response with a wrong, missing or approval id answers nothing
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_wrong_missing_or_approval_id_answers_nothing() -> TestResult {
    let mut fixture = Fixture::new()?;
    let session = fixture.session("bridge022-reject", None).await?;

    // @step Given a session waiting on its question
    let (route, answer) = ask(&session)?;
    session.set_pause_state(Some(PauseState {
        kind: PauseKind::Triple,
        tool_name: "Bash".into(),
        message: "Command requires approval".into(),
        details: None,
        tool_call_id: None,
    }));
    let approval = session
        .pending_pause_route()
        .ok_or("the session has no pending approval")?;

    // @step When the bridge sends question_response with no request id, then a stale id, then the id of the session's approval, then an unparsable response
    let attempts = [
        serde_json::json!({"action": "question_response", "response": jose()}),
        serde_json::json!({"action": "question_response", "request_id": "req-stale", "response": jose()}),
        serde_json::json!({"action": "question_response", "request_id": approval.request_id, "response": jose()}),
        serde_json::json!({"action": "question_response", "request_id": route.request_id, "response": "jose"}),
    ];
    for data in attempts {
        control(&session, data).await?;
    }

    // @step Then the question is still pending with the same request id
    assert_eq!(session.pending_question_route(), Some(route));
    assert_eq!(session.pending_pause_route(), Some(approval));

    // @step And cancelling it then returns cancelled as its first answer
    session.cancel_pending_interactions();
    let response =
        codelet_test_helpers::deadlock_guard("the question is cancelled", answer).await??;
    assert_eq!(response, HitlResponse::Cancelled { cancelled: true });
    Ok(())
}

/// Scenario: The bridge answers a question its parent holds and the parent is told
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_bridge_answers_a_question_its_parent_holds() -> TestResult {
    let mut fixture = Fixture::new()?;
    let parent = fixture.session("bridge022-parent", None).await?;
    let child = fixture.session("bridge022-child", Some(&parent)).await?;

    // @step Given a sub-agent's question held by its parent session
    let (route, answer) = ask(&child)?;
    assert_eq!(
        route.holder,
        RequestHolder::Session(parent.id.to_string().into())
    );

    // @step When the bridge sends question_response for it
    control(
        &child,
        serde_json::json!({
            "action": "question_response",
            "request_id": route.request_id,
            "response": jose(),
        }),
    )
    .await?;

    // @step Then the sub-agent's tool receives the answer
    let response =
        codelet_test_helpers::deadlock_guard("the question is answered", answer).await??;
    assert!(matches!(response, HitlResponse::Answered { .. }));

    // @step And the parent is told that the user answered it
    let told = messages(&parent).await;
    let expected = format!("The user answered request {}", route.request_id);
    assert!(
        told.iter()
            .any(|message| message.message.starts_with(&expected)),
        "the parent was told: {:?}",
        told.iter()
            .map(|message| &message.message)
            .collect::<Vec<_>>()
    );
    Ok(())
}
