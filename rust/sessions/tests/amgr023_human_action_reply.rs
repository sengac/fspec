//! Feature: spec/features/a-human-action-has-its-own-done-or-cancel-reply.feature
//!
//! AMGR-023: a human action (WebSearch 'interact with the page', /goal
//! 'human review needed') has its own reply, pause_human_action with Done or
//! Cancel, and the wire shows it as PauseKind::HumanAction. pause_confirm
//! answers a two-choice confirm only, and pause_human_action answers a human
//! action only.
//!
//! Pauses are raised the way the per-turn pause handler raises them (store
//! the pause, then block in `wait_for_pause_response` on a thread). A
//! rejected reply is proved to have sent nothing by causality (TEST-042): the
//! pause is then cancelled, and the wait returns `Interrupted`, the first
//! response it received. HANG SAFETY: every session is released (its pending
//! pause cancelled) before the test returns.

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{
    HumanActionReply, RequestHolder, RequestRoute, RequestStatus, SessionId, SessionStatus,
    StreamChunk,
};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
use std::sync::Arc;
use tokio::sync::broadcast;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SCREENSHOT_PROMPT: &str =
    "Page loaded at https://example.com. Interact with the page, then press Enter to capture screenshot.";

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

struct Fixture {
    manager: Arc<SessionManager>,
    release: Release,
    project: tempfile::TempDir,
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
            release: Release(Vec::new()),
            project: codelet_test_helpers::setup_temp_directory()?,
        })
    }

    async fn session(&mut self, name: &str) -> TestResult<Arc<BackgroundSession>> {
        let id = uuid::Uuid::new_v4().to_string();
        self.manager
            .create_session_with_id(
                &id,
                "anthropic/claude-sonnet-4",
                &self.project.path().to_string_lossy(),
                name,
            )
            .await?;
        let session = self.manager.get_session(&id)?;
        self.release.0.push(session.clone());
        Ok(session)
    }
}

fn sid(session: &BackgroundSession) -> SessionId {
    SessionId::from(session.id.to_string())
}

fn pause(kind: PauseKind) -> PauseState {
    let (tool_name, message) = match kind {
        PauseKind::Continue => ("WebSearch", SCREENSHOT_PROMPT),
        PauseKind::Confirm => ("Edit", "Apply the edit?"),
        PauseKind::Triple => ("Bash", "Command requires approval"),
    };
    PauseState {
        kind,
        tool_name: tool_name.into(),
        message: message.into(),
        details: None,
        tool_call_id: None,
    }
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

async fn returned(
    answer: tokio::sync::oneshot::Receiver<PauseResponse>,
) -> TestResult<PauseResponse> {
    Ok(codelet_test_helpers::deadlock_guard("the wait returns", answer).await??)
}

fn pause_route(session: &BackgroundSession) -> TestResult<RequestRoute> {
    Ok(session
        .pending_pause_route()
        .ok_or("the session has no pending pause")?)
}

/// TEST-042 causality: a rejected reply sent nothing, so cancelling the
/// pause makes the wait return `Interrupted` as the first response it got.
async fn assert_nothing_was_sent(
    session: &BackgroundSession,
    answer: tokio::sync::oneshot::Receiver<PauseResponse>,
) -> TestResult {
    session.cancel_pending_interactions();
    assert_eq!(returned(answer).await?, PauseResponse::Interrupted);
    Ok(())
}

/// The next request event for `request_id` on `chunks`, skipping others.
async fn request_status(
    chunks: &mut broadcast::Receiver<StreamChunk>,
    request_id: &str,
) -> TestResult<RequestStatus> {
    let status = codelet_test_helpers::deadlock_guard("the request event arrives", async {
        loop {
            match chunks.recv().await {
                Ok(StreamChunk::InteractionRequestChanged { route, status })
                    if route.request_id == request_id =>
                {
                    return Ok(status);
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return Err("stream closed"),
            }
        }
    })
    .await??;
    Ok(status)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_done_answers_a_human_action_and_the_tool_goes_on() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given a session waiting on a WebSearch human action "Interact with the page, then press Enter to capture screenshot."
    let session = fixture.session("amgr023").await?;
    let mut chunks = session.subscribe_to_stream();
    let answer = hold(&session, pause(PauseKind::Continue));
    let route = pause_route(&session)?;
    assert_eq!(
        request_status(&mut chunks, &route.request_id).await?,
        RequestStatus::Pending
    );
    // @step When pause_human_action names its request id with Done
    SessionManagerHandle::pause_human_action(
        &*fixture.manager,
        &sid(&session),
        &route.request_id,
        HumanActionReply::Done,
    )?;
    // @step Then the waiting tool receives Resumed and the session is Running
    assert_eq!(returned(answer).await?, PauseResponse::Resumed);
    assert_eq!(session.get_status(), SessionStatus::Running);
    assert!(session.pending_pause_route().is_none());
    // @step And the request is announced Answered by the user
    assert_eq!(
        request_status(&mut chunks, &route.request_id).await?,
        RequestStatus::Answered {
            by: RequestHolder::User
        }
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_pause_confirm_does_not_answer_a_human_action() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given a session waiting on a WebSearch human action
    let session = fixture.session("amgr023").await?;
    let answer = hold(&session, pause(PauseKind::Continue));
    let route = pause_route(&session)?;
    // @step When pause_confirm with accept true names its request id
    let reply = SessionManagerHandle::pause_confirm(
        &*fixture.manager,
        &sid(&session),
        &route.request_id,
        true,
    );
    // @step Then it is rejected with an error naming the request id, the human action and pause_confirm
    let error = reply.err().ok_or("pause_confirm answered a human action")?;
    assert!(error.contains(&route.request_id), "{error}");
    assert!(error.contains("human action"), "{error}");
    assert!(error.contains("pause_confirm"), "{error}");
    // @step And the tool is still waiting and the human action is still pending under the same id
    assert_eq!(pause_route(&session)?, route);
    assert_nothing_was_sent(&session, answer).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_pause_human_action_answers_a_human_action_only() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given a two-choice confirm and a three-choice approval, each pending on its own session
    let mut pending = Vec::new();
    for kind in [PauseKind::Confirm, PauseKind::Triple] {
        let session = fixture.session(&format!("amgr023-{kind:?}")).await?;
        let answer = hold(&session, pause(kind));
        let route = pause_route(&session)?;
        pending.push((session, answer, route));
    }
    // @step When pause_human_action with Done names each request id
    let mut replies = Vec::new();
    for (session, _, route) in &pending {
        replies.push(SessionManagerHandle::pause_human_action(
            &*fixture.manager,
            &sid(session),
            &route.request_id,
            HumanActionReply::Done,
        ));
    }
    // @step Then both are rejected with an error naming pause_human_action
    for reply in &replies {
        let error = reply
            .as_ref()
            .err()
            .ok_or("pause_human_action answered an approval")?;
        assert!(error.contains("pause_human_action"), "{error}");
    }
    // @step And both requests are still pending under the same ids with their tools waiting
    for (session, answer, route) in pending {
        assert_eq!(pause_route(&session)?, route);
        assert_nothing_was_sent(&session, answer).await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_wire_shows_a_human_action_as_its_own_pause_kind() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given a session waiting on a WebSearch human action and another waiting on a two-choice confirm
    let human = fixture.session("amgr023-human").await?;
    let confirm = fixture.session("amgr023-confirm").await?;
    let _human_answer = hold(&human, pause(PauseKind::Continue));
    let _confirm_answer = hold(&confirm, pause(PauseKind::Confirm));
    // @step When get_pause_state reads each session
    let human_state = SessionManagerHandle::get_pause_state(&*fixture.manager, &sid(&human))?
        .ok_or("the human action is not pending")?;
    let confirm_state = SessionManagerHandle::get_pause_state(&*fixture.manager, &sid(&confirm))?
        .ok_or("the confirm is not pending")?;
    // @step Then the human action's kind is HumanAction and the confirm's kind is Confirm
    assert_eq!(
        human_state.payload.kind,
        codelet_rpc_types::PauseKind::HumanAction
    );
    assert_eq!(
        confirm_state.payload.kind,
        codelet_rpc_types::PauseKind::Confirm
    );
    Ok(())
}
