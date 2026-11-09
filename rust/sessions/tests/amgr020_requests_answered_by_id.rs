//! Feature: spec/features/questions-and-approvals-are-requests-answered-by-id.feature
//!
//! AMGR-020: every question and approval a session raises is a request with
//! an id, an origin, a kind and the parent chain read from the spawner links
//! when it is raised. Its holder is the origin's parent (the user for a root
//! session or a human action). Replies name the request id and are accepted
//! only from the holder or the user; every session on the chain is told
//! when the request is raised and when it is answered.
//!
//! Requests are raised the way the per-turn handlers raise them (store the
//! request, then block in `wait_for_*_response` on a thread). HANG SAFETY:
//! every scenario keeps a [`Release`] that cancels whatever is still pending
//! (through `cancel_pending_interactions`) before the test returns.

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{
    RequestHolder, RequestKind, RequestRoute, RequestStatus, SessionId, SessionStatus, StreamChunk,
};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::request_user_input::HitlResponse;
use codelet_tools::request_user_input::{HitlOption, HitlQuestion, HitlRequest};
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
use std::sync::Arc;
use tokio::sync::broadcast;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Cancels whatever a session still waits on so no waiting thread outlives
/// the test (see the module docs).
struct Release(Vec<Arc<BackgroundSession>>);

impl Drop for Release {
    fn drop(&mut self) {
        for session in &self.0 {
            session.cancel_pending_interactions();
        }
    }
}

struct Fixture {
    manager: Arc<SessionManager>,
    _release: Release,
    _project: tempfile::TempDir,
}

/// The data directory is process-wide, so the tests (which run in parallel)
/// share one, set once and alive until the process exits (BUG-406).
fn data_directory() -> Result<(), String> {
    static DATA: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let data = codelet_test_helpers::shared_temp_subdirectory()?;
        std::fs::create_dir_all(data.join("cache")).map_err(|e| e.to_string())?;
        std::fs::write(
            data.join("cache/models.json"),
            include_str!("fixtures/rpc422_models.json"),
        )
        .map_err(|e| e.to_string())?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(data)?;
        // Sessions select a model at creation; no request is ever sent.
        std::env::set_var("ANTHROPIC_API_KEY", "offline");
        Ok(())
    })
    .clone()
}

impl Fixture {
    async fn new() -> TestResult<Self> {
        // BUG-347 / BUG-449: never consult the host's ~/.fspec.
        codelet_test_helpers::isolated_user_directory()?;
        data_directory()?;
        Ok(Self {
            manager: SessionManager::new(),
            _release: Release(Vec::new()),
            _project: codelet_test_helpers::setup_temp_directory()?,
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
                "anthropic/claude-sonnet-4",
                &self._project.path().to_string_lossy(),
                name,
            )
            .await?;
        if let Some(parent) = parent {
            self.manager.add_supervisor(id, parent.id)?;
        }
        let session = self.manager.get_session(&id.to_string())?;
        self._release.0.push(session.clone());
        Ok(session)
    }
}

fn sid(session: &BackgroundSession) -> SessionId {
    SessionId::from(session.id.to_string())
}

fn question(text: &str) -> HitlRequest {
    HitlRequest {
        questions: vec![HitlQuestion {
            id: "auth_library".into(),
            header: "Auth".into(),
            question: text.into(),
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

fn pause(kind: PauseKind, tool_name: &str, message: &str) -> PauseState {
    PauseState {
        kind,
        tool_name: tool_name.into(),
        message: message.into(),
        details: Some("echo amgr020 git reset --hard".into()),
        tool_call_id: None,
    }
}

fn approval() -> PauseState {
    pause(PauseKind::Triple, "Bash", "Command requires approval")
}

/// Raise `request` the way the per-turn HITL handler does and wait for it on
/// its own thread; the receiver yields what the wait returned.
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

fn jose() -> codelet_rpc_types::HitlResponse {
    codelet_rpc_types::HitlResponse {
        cancelled: false,
        answers: vec![codelet_rpc_types::HitlAnswer {
            id: "auth_library".into(),
            selected: vec!["jose".into()],
            other: None,
        }],
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_sub_agents_question_is_a_request_held_by_its_parent() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given bot R spawned sub-agent A
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    // @step When A asks the question "Which auth library?"
    let _answer = ask(&a, question("Which auth library?"));
    // @step Then get_hitl_request for A returns a request of kind question with origin A, chain [A, R] and holder R
    let pending = SessionManagerHandle::get_hitl_request(&*fixture.manager, &sid(&a))?
        .ok_or("no pending question")?;
    assert_eq!(pending.payload.questions[0].question, "Which auth library?");
    assert_eq!(pending.route.kind, RequestKind::Question);
    assert_eq!(pending.route.origin, sid(&a));
    assert_eq!(pending.route.chain, vec![sid(&a), sid(&r)]);
    assert_eq!(pending.route.holder, RequestHolder::Session(sid(&r)));
    assert_eq!(pending.route, question_route(&a)?);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_session_with_no_parent_sends_its_question_to_the_user() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given a session with no parent
    let root = fixture.session("root", None).await?;
    // @step When it asks a question
    let _answer = ask(&root, question("Which auth library?"));
    // @step Then the request's chain is that session alone and its holder is the user
    let route = question_route(&root)?;
    assert_eq!(route.chain, vec![sid(&root)]);
    assert_eq!(route.holder, RequestHolder::User);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_nested_sub_agents_approval_is_held_by_its_own_parent() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given R spawned A and A spawned B
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let b = fixture.session("B", Some(&a)).await?;
    // @step When the blocklist holds B's Bash command for approval
    let _answer = hold(&b, approval());
    // @step Then get_pause_state for B returns a request of kind approval with origin B, chain [B, A, R] and holder A
    let pending = SessionManagerHandle::get_pause_state(&*fixture.manager, &sid(&b))?
        .ok_or("no pending approval")?;
    assert_eq!(pending.payload.prompt, "Bash: Command requires approval");
    assert_eq!(pending.route.kind, RequestKind::Approval);
    assert_eq!(pending.route.origin, sid(&b));
    assert_eq!(pending.route.chain, vec![sid(&b), sid(&a), sid(&r)]);
    assert_eq!(pending.route.holder, RequestHolder::Session(sid(&a)));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_human_action_is_always_held_by_the_user() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given bot R spawned sub-agent A
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    // @step When A's WebSearch asks for the page to be used and Enter pressed
    let _answer = hold(
        &a,
        pause(
            PauseKind::Continue,
            "WebSearch",
            "Page loaded at https://example.com. Interact with the page, then press Enter to capture screenshot.",
        ),
    );
    // @step Then the request has kind human action and its holder is the user
    let route = pause_route(&a)?;
    assert_eq!(route.kind, RequestKind::HumanAction);
    assert_eq!(route.chain, vec![sid(&a), sid(&r)]);
    assert_eq!(route.holder, RequestHolder::User);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_user_answers_a_sub_agents_question_by_its_id_and_every_session_on_the_chain_is_told(
) -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given A's question is held by R
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let mut r_chunks = r.subscribe_to_stream();
    let mut a_chunks = a.subscribe_to_stream();
    let answer = ask(&a, question("Which auth library?"));
    let route = question_route(&a)?;
    assert_eq!(route.holder, RequestHolder::Session(sid(&r)));
    // @step When the user answers it with send_hitl_response naming its id
    SessionManagerHandle::send_hitl_response(
        &*fixture.manager,
        &sid(&a),
        &route.request_id,
        jose(),
    )?;
    // @step Then A's tool receives the answer, and A and R each received an event for that id when it was raised and one saying the user answered it
    let HitlResponse::Answered { answers } = returned(answer).await? else {
        return Err("the question was not answered".into());
    };
    let chosen = answers
        .get("auth_library")
        .ok_or("no answer for auth_library")?;
    assert_eq!(chosen.selected, vec!["jose".to_string()]);
    assert_eq!(chosen.other, None);
    for chunks in [&mut a_chunks, &mut r_chunks] {
        assert_eq!(
            request_event(chunks, &route.request_id).await?,
            (route.clone(), RequestStatus::Pending)
        );
        assert_eq!(
            request_event(chunks, &route.request_id).await?.1,
            RequestStatus::Answered {
                by: RequestHolder::User
            }
        );
    }
    assert!(a.pending_question_route().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_holder_answers_its_sub_agents_approval() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given A's approval is held by R
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let mut r_chunks = r.subscribe_to_stream();
    let mut a_chunks = a.subscribe_to_stream();
    let answer = hold(&a, approval());
    let route = pause_route(&a)?;
    // @step When R replies Deny naming its id
    a.answer_pause(
        &route.request_id,
        &RequestHolder::Session(sid(&r)),
        codelet_sessions::background_session::PauseReply::Triple(
            codelet_rpc_types::ApprovalChoice::Deny,
        ),
    )?;
    // @step Then A's approval is answered Deny, and A and R each received an event saying R answered it
    assert_eq!(returned(answer).await?, PauseResponse::Denied);
    for chunks in [&mut a_chunks, &mut r_chunks] {
        assert_eq!(
            request_event(chunks, &route.request_id).await?.1,
            RequestStatus::Pending
        );
        assert_eq!(
            request_event(chunks, &route.request_id).await?.1,
            RequestStatus::Answered {
                by: RequestHolder::Session(sid(&r))
            }
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_reply_from_a_session_that_is_not_the_holder_is_rejected() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given A's question is held by R
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let s = fixture.session("S", Some(&r)).await?;
    let _answer = ask(&a, question("Which auth library?"));
    let route = question_route(&a)?;
    // @step When A's sibling sub-agent S replies to it naming its id
    let reply = a.answer_question(
        &route.request_id,
        &RequestHolder::Session(sid(&s)),
        HitlResponse::Cancelled { cancelled: true },
    );
    // @step Then the reply is rejected as not from the holder and the question stays pending
    let error = reply.err().ok_or("the sibling's reply was accepted")?;
    assert_eq!(
        error,
        format!(
            "Session {} does not hold request {}; its holder is session {}",
            s.id, route.request_id, r.id
        )
    );
    assert_eq!(question_route(&a)?, route);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_reply_naming_a_replaced_request_is_rejected() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given A asked a question and then a new one
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    a.set_hitl_request(Some(question("Which auth library?")));
    let first = question_route(&a)?;
    let _answer = ask(&a, question("Which token lifetime?"));
    let second = question_route(&a)?;
    // @step When a reply names the first question's id
    let reply = SessionManagerHandle::send_hitl_response(
        &*fixture.manager,
        &sid(&a),
        &first.request_id,
        jose(),
    );
    // @step Then the reply is rejected and the new question stays pending
    assert_eq!(
        reply.err().as_deref(),
        Some("Stale or already answered interaction")
    );
    assert_ne!(first.request_id, second.request_id);
    assert_eq!(question_route(&a)?, second);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_reply_of_the_wrong_kind_is_rejected() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given A waits on a blocklist approval
    let r = fixture.session("R", None).await?;
    let a = fixture.session("A", Some(&r)).await?;
    let _answer = hold(&a, approval());
    let route = pause_route(&a)?;
    // @step When a question answer is sent naming the approval's id
    let reply = SessionManagerHandle::send_hitl_response(
        &*fixture.manager,
        &sid(&a),
        &route.request_id,
        jose(),
    );
    // @step Then the reply is rejected and the approval stays pending
    assert_eq!(
        reply.err(),
        Some(format!(
            "Request {} is an approval, not a question",
            route.request_id
        ))
    );
    assert_eq!(pause_route(&a)?, route);
    Ok(())
}

/// Production sources (comments stripped) of the crates on every reply path.
fn sources() -> TestResult<Vec<(String, String)>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for dir in [
        "sessions/src",
        "core/src",
        "rpc/src",
        "agent-loop/src",
        "tools/src",
        "fspec-tui/src",
        "fspec/src",
    ] {
        let mut pending = vec![root.join(dir)];
        while let Some(path) = pending.pop() {
            for entry in std::fs::read_dir(&path)? {
                let path = entry?.path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let text = std::fs::read_to_string(&path)?
                        .lines()
                        .filter(|line| !line.trim_start().starts_with("//"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    files.push((path.display().to_string(), text));
                }
            }
        }
    }
    Ok(files)
}

#[test]
fn scenario_no_unchecked_reply_path_is_left() -> TestResult {
    // @step Given the daemon, RPC, TUI, bridge and client MCP sources
    let files = sources()?;
    // @step When they are searched for the ways to answer a question or approval
    // reply_interaction_checked / get_interaction_snapshot keep their
    // frozen tarpc slots (RPC-449) and go through the same checked replies.
    let gone = [
        "fn interaction_snapshot",
        "fn send_pause_response",
        "fn send_hitl_response_checked",
        "fn get_pause_response_tx",
        "request_bound",
    ];
    let reply_methods = [
        "fn pause_human_action(",
        "fn pause_confirm(",
        "fn pause_triple(",
        "fn send_hitl_response(",
    ];
    // @step Then every one takes a request id, and the unchecked session replies and the client MCP snapshot-then-reply wrapper are gone
    let mut found = Vec::new();
    for (path, text) in &files {
        for needle in gone {
            if text.contains(needle) {
                found.push(format!("{path}: {needle}"));
            }
        }
        for method in reply_methods {
            let mut rest = text.as_str();
            while let Some(at) = rest.find(method) {
                let signature: String = rest[at..].chars().take_while(|c| *c != ')').collect();
                if !signature.contains("request_id") {
                    found.push(format!("{path}: {method} without request_id"));
                }
                rest = &rest[at + method.len()..];
            }
        }
    }
    assert!(found.is_empty(), "unchecked reply paths remain: {found:#?}");
    Ok(())
}
