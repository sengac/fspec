//! Feature: spec/features/a-pause-reply-answers-only-the-pause-kind-it-is-for.feature
//!
//! BUG-631: each pause reply action answers exactly one pause kind as the
//! wire shows it. AMGR-023: pause_human_action answers a human action
//! (PauseKind::Continue); pause_confirm answers a two-choice confirm
//! (PauseKind::Confirm) only; pause_triple answers a
//! three-choice approval (PauseKind::Triple). Any other reply is rejected and
//! answers nothing.
//!
//! Pauses are raised the way the per-turn pause handler raises them (store
//! the pause, then block in `wait_for_pause_response` on a thread). A
//! rejected reply is proved to have sent nothing by causality (TEST-042): the
//! pause is then cancelled, and the wait returns `Interrupted`, the first
//! response it received. HANG SAFETY: every session is released (its pending
//! pause cancelled) before the test returns.

use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{ApprovalChoice, HumanActionReply, RequestRoute, SessionId};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState};
use std::sync::Arc;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
        PauseKind::Continue => (
            "WebSearch",
            "Page loaded at https://example.com. Interact with the page, then press Enter to capture screenshot.",
        ),
        PauseKind::Confirm => ("Edit", "Apply the edit?"),
        PauseKind::Triple => ("Bash", "Command requires approval"),
    };
    PauseState {
        kind,
        tool_name: tool_name.into(),
        message: message.into(),
        details: Some("echo bug631 git reset --hard".into()),
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

/// A reply action as the handle sends it.
#[derive(Debug, Clone, Copy)]
enum Reply {
    HumanAction,
    Confirm(bool),
    Triple(ApprovalChoice),
}

const REPLIES: [Reply; 6] = [
    Reply::HumanAction,
    Reply::Confirm(true),
    Reply::Confirm(false),
    Reply::Triple(ApprovalChoice::Approve),
    Reply::Triple(ApprovalChoice::ApproveSession),
    Reply::Triple(ApprovalChoice::Deny),
];

fn send(
    manager: &SessionManager,
    session: &SessionId,
    request_id: &str,
    reply: Reply,
) -> Result<(), String> {
    match reply {
        Reply::HumanAction => SessionManagerHandle::pause_human_action(
            manager,
            session,
            request_id,
            HumanActionReply::Done,
        ),
        Reply::Confirm(accept) => {
            SessionManagerHandle::pause_confirm(manager, session, request_id, accept)
        }
        Reply::Triple(choice) => {
            SessionManagerHandle::pause_triple(manager, session, request_id, choice)
        }
    }
}

/// What the tool receives when `reply` answers a pause of `kind`, or `None`
/// when the reply is not for that kind.
fn expected(kind: PauseKind, reply: Reply) -> Option<PauseResponse> {
    match (kind, reply) {
        (PauseKind::Continue, Reply::HumanAction) => Some(PauseResponse::Resumed),
        (PauseKind::Confirm, Reply::Confirm(true)) => Some(PauseResponse::Approved),
        (PauseKind::Confirm, Reply::Confirm(false)) => Some(PauseResponse::Denied),
        (PauseKind::Triple, Reply::Triple(ApprovalChoice::Approve)) => {
            Some(PauseResponse::AllowOnce)
        }
        (PauseKind::Triple, Reply::Triple(ApprovalChoice::ApproveSession)) => {
            Some(PauseResponse::AllowSession)
        }
        (PauseKind::Triple, Reply::Triple(ApprovalChoice::Deny)) => Some(PauseResponse::Denied),
        _ => None,
    }
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_resume_sent_to_a_three_choice_approval_is_rejected() -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given a session waiting on a blocklist three-choice approval for "echo bug631 git reset --hard"
    let session = fixture.session("bug631").await?;
    let answer = hold(&session, pause(PauseKind::Triple));
    let route = pause_route(&session)?;
    // @step When pause_human_action names that approval's request id
    let reply = SessionManagerHandle::pause_human_action(
        &*fixture.manager,
        &sid(&session),
        &route.request_id,
        HumanActionReply::Done,
    );
    // @step Then it is rejected with an error naming the request id, the three-choice approval and pause_human_action
    let error = reply
        .err()
        .ok_or("pause_human_action answered a three-choice approval")?;
    assert!(error.contains(&route.request_id), "{error}");
    assert!(error.contains("three-choice approval"), "{error}");
    assert!(error.contains("pause_human_action"), "{error}");
    // @step And the tool is still waiting and the approval is still pending under the same id
    assert_eq!(pause_route(&session)?, route);
    assert_nothing_was_sent(&session, answer).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_every_reply_action_is_accepted_only_for_the_pause_kind_it_answers() -> TestResult
{
    let mut fixture = Fixture::new().await?;
    // @step Given a human action, a two-choice confirm and a three-choice approval, each pending on its own session
    let kinds = [PauseKind::Continue, PauseKind::Confirm, PauseKind::Triple];
    let mut sessions = Vec::new();
    for kind in kinds {
        sessions.push((kind, fixture.session(&format!("bug631-{kind:?}")).await?));
    }
    // @step When pause_human_action done, pause_confirm accept and deny, and pause_triple approve, approve for session and deny are each sent to each of them
    // @step Then pause_human_action is accepted only by the human action, pause_confirm only by the two-choice confirm, and pause_triple only by the three-choice approval
    // @step And each accepted reply gives the tool the response it names, and each rejected reply leaves the request pending under the same id with its tool still waiting
    for (kind, session) in &sessions {
        for reply in REPLIES {
            let answer = hold(session, pause(*kind));
            let route = pause_route(session)?;
            let sent = send(&fixture.manager, &sid(session), &route.request_id, reply);
            match expected(*kind, reply) {
                Some(response) => {
                    assert!(sent.is_ok(), "{reply:?} to {kind:?} was rejected: {sent:?}");
                    assert_eq!(returned(answer).await?, response, "{reply:?} to {kind:?}");
                    assert!(session.pending_pause_route().is_none());
                }
                None => {
                    assert!(sent.is_err(), "{reply:?} answered {kind:?}");
                    assert_eq!(pause_route(session)?, route, "{reply:?} to {kind:?}");
                    assert_nothing_was_sent(session, answer).await?;
                }
            }
        }
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_right_reply_still_answers_a_request_after_a_wrong_one_was_rejected(
) -> TestResult {
    let mut fixture = Fixture::new().await?;
    // @step Given a session waiting on a blocklist three-choice approval
    let session = fixture.session("bug631").await?;
    let answer = hold(&session, pause(PauseKind::Triple));
    let route = pause_route(&session)?;
    // @step When pause_confirm with accept true names its request id and then pause_triple with approve names the same id
    let wrong = SessionManagerHandle::pause_confirm(
        &*fixture.manager,
        &sid(&session),
        &route.request_id,
        true,
    );
    SessionManagerHandle::pause_triple(
        &*fixture.manager,
        &sid(&session),
        &route.request_id,
        ApprovalChoice::Approve,
    )?;
    // @step Then the pause_confirm is rejected and the tool receives allow once from the pause_triple
    let error = wrong
        .err()
        .ok_or("pause_confirm answered a three-choice approval")?;
    assert!(error.contains("pause_confirm"), "{error}");
    assert_eq!(returned(answer).await?, PauseResponse::AllowOnce);
    Ok(())
}
