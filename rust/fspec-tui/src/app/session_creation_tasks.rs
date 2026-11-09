use std::sync::Arc;

use codelet_rpc_types::SessionId;
use tokio::sync::{mpsc::UnboundedSender, watch};

use codelet_sessions::profile_limits::MissingProfileLimit;

use crate::components::missing_profile_limit_dialog::MissingLimitRetry;
use crate::components::Action;
use crate::transport::FspecBackend;

/// What to create: isolated or not, the session role (TUI-114: the
/// orchestrator role for a bot), the selected workspace and the card the
/// new session is bound to.
pub(crate) struct CreationRequest {
    pub(crate) isolated: bool,
    pub(crate) role: Option<String>,
    pub(crate) workspace: Option<String>,
    pub(crate) card: Option<String>,
}

pub(crate) async fn run_session_creation(
    backend: Arc<dyn FspecBackend>,
    request: CreationRequest,
    active_session: Option<watch::Sender<Option<SessionId>>>,
    originating_session: Option<SessionId>,
    sender: UnboundedSender<Action>,
) {
    create_session_reporting(
        backend,
        request,
        active_session,
        originating_session,
        sender,
    )
    .await;
}

/// Create the session and report the outcome; the created session, if any
/// (BUG-576: no-session model selections apply to it).
pub(crate) async fn create_session_reporting(
    backend: Arc<dyn FspecBackend>,
    request: CreationRequest,
    active_session: Option<watch::Sender<Option<SessionId>>>,
    originating_session: Option<SessionId>,
    sender: UnboundedSender<Action>,
) -> Option<SessionId> {
    let seed_active = active_session.is_some();
    let CreationRequest {
        isolated,
        role,
        workspace,
        card,
    } = request;
    // BUG-547: kept to create the session again once a missing limit is given.
    let retry = MissingLimitRetry::SessionCreation {
        isolated,
        role: role.clone(),
        workspace: workspace.clone(),
        card: card.clone(),
        seed_active,
        originating_session: originating_session.clone(),
    };
    let bot = role
        .as_deref()
        .is_some_and(codelet_sessions::bot::is_bot_role);
    let result = if isolated {
        // BUG-427: an isolated session for a selected workspace is cut from
        // that workspace's repository.
        match workspace {
            Some(workspace) => {
                backend
                    .create_isolated_session_in_workspace(workspace, role)
                    .await
            }
            None => backend.create_isolated_session(role).await,
        }
        .map(|info| info.session_id)
    } else if let Some(workspace) = workspace {
        // RPC-445: a non-start workspace is selected on the board.
        backend.create_session_in_workspace(workspace, role).await
    } else {
        backend.create_session(role).await
    };
    let created = match result {
        Ok(session) => {
            if bot {
                // TUI-114: mark the chip before the session is folded in.
                let _ = sender.send(Action::BotSessionCreated(session.clone()));
            }
            if let Some(active_session) = active_session {
                let _ = active_session.send(Some(session.clone()));
            }
            let _ = sender.send(super::session_creation::post_create_session_with_work_unit(
                session.clone(),
                card,
            ));
            Some(session)
        }
        Err(error) => {
            // BUG-547: a profile model missing a limit asks the user.
            if let Some(limit) = MissingProfileLimit::find_in(&error.to_string()) {
                let _ = sender.send(Action::OpenMissingProfileLimitPrompt { limit, retry });
                super::session_storage::publish_manifest_issues(backend.as_ref(), &sender, false)
                    .await;
                return None;
            }
            if let Some(session) = originating_session {
                let operation = if isolated {
                    "create isolated session"
                } else {
                    "create session"
                };
                let _ = sender.send(Action::EmitSessionNotice(
                    session,
                    format!("[error] {operation}: {error}"),
                ));
            }
            let _ = sender.send(Action::SessionOperationFailed(format!(
                "Session creation failed: {error}"
            )));
            None
        }
    };
    super::session_storage::publish_manifest_issues(backend.as_ref(), &sender, false).await;
    created
}
