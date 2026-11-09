//! Feature: spec/features/a-human-action-has-its-own-done-or-cancel-reply.feature
//!
//! AMGR-023: cancelling a human action gives the waiting tool the result an
//! interrupt gives. WebSearch's page wait and the /goal escalation call
//! `pause_for_user` exactly as they do in a turn; the session's pause handler
//! here does what the per-turn handler (agent_loop/handlers.rs) does: store
//! the pause, mark the session Paused and block in `wait_for_pause_response`.
//! The bridge answers a human action with `human_action_response`.
//!
//! HANG SAFETY: every session is released (its pending pause cancelled and
//! its pause handler cleared) before the test returns.

use codelet_cli::interactive::goal::{build_goal_blocked_message, raise_goal_escalation};
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_rpc_types::{HumanActionReply, RequestRoute, SessionId, SessionStatus};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use codelet_tools::tool_pause::{set_pause_handler, PauseHandler, PauseResponse, PauseState};
use std::sync::Arc;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Cancels whatever the session still waits on and clears its pause handler
/// so no waiting thread outlives the test.
struct Release(Arc<BackgroundSession>);

impl Drop for Release {
    fn drop(&mut self) {
        self.0.cancel_pending_interactions();
        set_pause_handler(self.0.id, None);
    }
}

struct Fixture {
    manager: Arc<SessionManager>,
    session: Arc<BackgroundSession>,
    _release: Release,
    _project: tempfile::TempDir,
}

/// The data directory is process-wide, so the tests (which run in parallel)
/// share one, set once and alive until the process exits (BUG-406).
fn data_directory() -> Result<(), String> {
    static DATA: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        let data = codelet_test_helpers::shared_temp_subdirectory()?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(data)?;
        // The session selects a model at creation; no request is ever sent.
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
        let project = codelet_test_helpers::setup_temp_directory()?;
        let manager = SessionManager::new();
        let id = uuid::Uuid::new_v4().to_string();
        manager
            .create_session_with_id(
                &id,
                "anthropic/claude-sonnet-4-5",
                &project.path().to_string_lossy(),
                "amgr023",
            )
            .await?;
        let session = manager.get_session(&id)?;
        let waiting = session.clone();
        let handler: PauseHandler = Arc::new(move |request| {
            waiting.set_pause_state(Some(PauseState::for_session(waiting.id, request)));
            waiting.set_status(SessionStatus::Paused);
            let response = waiting.wait_for_pause_response();
            waiting.resume_after_interaction();
            response
        });
        set_pause_handler(session.id, Some(handler));
        Ok(Self {
            manager,
            _release: Release(session.clone()),
            session,
            _project: project,
        })
    }

    fn sid(&self) -> SessionId {
        SessionId::from(self.session.id.to_string())
    }

    /// The human action the session waits on, once its pause is stored.
    async fn pending_route(&self) -> TestResult<RequestRoute> {
        let session = self.session.clone();
        let route = codelet_test_helpers::deadlock_guard("the human action is pending", async {
            let mut changes = session.subscribe_to_stream();
            loop {
                if let Some(route) = session.pending_pause_route() {
                    return route;
                }
                let _ = changes.recv().await;
            }
        })
        .await?;
        Ok(route)
    }

    async fn send_bridge(&self, data: serde_json::Value) -> TestResult {
        let envelope = serde_json::json!({
            "service": "session",
            "type": "control",
            "session_id": self.session.id.to_string(),
            "data": data,
        });
        let injector: codelet_tools::InputInjector = Arc::new(|_| {});
        codelet_tools::handle_multiplexed_inbound(
            &envelope.to_string(),
            self.session.id,
            injector,
            Some(codelet_agent_loop::bridge_control::control_handler(
                self.session.clone(),
            )),
            None,
            None,
        )
        .await?;
        Ok(())
    }
}

/// Run `wait` on its own thread (it blocks until the pause is answered).
fn waiting<T: Send + 'static>(
    wait: impl FnOnce() -> T + Send + 'static,
) -> tokio::sync::oneshot::Receiver<T> {
    let (done, result) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let _ = done.send(wait());
    });
    result
}

async fn returned<T>(result: tokio::sync::oneshot::Receiver<T>) -> TestResult<T> {
    Ok(codelet_test_helpers::deadlock_guard("the wait returns", result).await??)
}

fn screenshot_pause() -> TestResult<codelet_tools::tool_pause::PauseRequest> {
    let action = codelet_common::web_search::WebSearchAction::CaptureScreenshot {
        url: Some("https://example.com".into()),
        output_path: None,
        full_page: None,
        headless: false,
        pause: true,
    };
    Ok(codelet_tools::web_search::page_pause_request(&action)
        .ok_or("a paused screenshot asks for a human action")?)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_cancel_stops_websearch_with_the_result_an_interrupt_gives() -> TestResult {
    // @step Given a session whose WebSearch waits for the user to interact with the page before it captures the screenshot
    let fixture = Fixture::new().await?;
    let session_id = fixture.session.id;
    let pause = screenshot_pause()?;
    let cancelled =
        waiting(move || codelet_tools::web_search::wait_for_user(session_id, Some(pause)));
    let route = fixture.pending_route().await?;
    assert_eq!(route.kind, codelet_rpc_types::RequestKind::HumanAction);

    // @step When pause_human_action names the human action's request id with Cancel
    SessionManagerHandle::pause_human_action(
        &*fixture.manager,
        &fixture.sid(),
        &route.request_id,
        HumanActionReply::Cancel,
    )?;

    // @step Then the wait ends as an interrupt does, so WebSearch returns "Operation interrupted by user" and captures no screenshot
    assert!(
        returned(cancelled).await?,
        "a cancelled page wait reports an interrupt, so WebSearch stops before the screenshot"
    );
    let pause = screenshot_pause()?;
    let interrupted =
        waiting(move || codelet_tools::web_search::wait_for_user(session_id, Some(pause)));
    fixture.pending_route().await?;
    fixture.session.cancel_pending_interactions();
    assert!(
        returned(interrupted).await?,
        "an interrupted page wait reports the same result"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_cancel_ends_the_goal_escalation_and_the_goal_stays_active() -> TestResult {
    // @step Given a session with the goal "ship the release" whose "human review needed" escalation waits for the user
    let fixture = Fixture::new().await?;
    let session_id = fixture.session.id;
    codelet_tools::set_session_goal(
        session_id,
        Some(codelet_tools::GoalSpec {
            text: "ship the release".to_string(),
            verify: None,
        }),
    );
    let escalation =
        waiting(move || raise_goal_escalation(session_id, &build_goal_blocked_message()));
    let route = fixture.pending_route().await?;

    // @step When pause_human_action names the escalation's request id with Cancel
    SessionManagerHandle::pause_human_action(
        &*fixture.manager,
        &fixture.sid(),
        &route.request_id,
        HumanActionReply::Cancel,
    )?;

    // @step Then the escalation returns Interrupted
    assert_eq!(returned(escalation).await?, PauseResponse::Interrupted);

    // @step And the session's goal is still "ship the release"
    let goal = codelet_tools::get_session_goal(session_id).map(|goal| goal.text);
    codelet_tools::set_session_goal(session_id, None);
    assert_eq!(goal.as_deref(), Some("ship the release"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_bridge_answers_a_human_action_with_done_or_cancel() -> TestResult {
    // @step Given a session waiting on a WebSearch human action
    let fixture = Fixture::new().await?;
    let session_id = fixture.session.id;
    let pause = screenshot_pause()?;
    let answer = waiting(move || codelet_tools::tool_pause::pause_for_user(session_id, pause));
    let route = fixture.pending_route().await?;

    // @step When the bridge sends pause_response "allow_once" and then human_action_response "cancel" naming its request id
    fixture
        .send_bridge(serde_json::json!({
            "action": "pause_response",
            "response": "allow_once",
            "request_id": route.request_id,
        }))
        .await?;
    assert_eq!(fixture.session.pending_pause_route(), Some(route.clone()));
    fixture
        .send_bridge(serde_json::json!({
            "action": "human_action_response",
            "response": "cancel",
            "request_id": route.request_id,
        }))
        .await?;

    // @step Then the pause_response answers nothing and the tool receives Interrupted from the human_action_response
    // TEST-042 causality: the allow_once sent nothing, so the first response
    // the tool receives is the human_action_response's.
    assert_eq!(returned(answer).await?, PauseResponse::Interrupted);
    assert!(fixture.session.pending_pause_route().is_none());
    Ok(())
}
