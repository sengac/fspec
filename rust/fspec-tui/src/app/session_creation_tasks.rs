use std::sync::Arc;

use codelet_rpc_types::SessionId;
use tokio::sync::{mpsc::UnboundedSender, watch};

use crate::components::Action;
use crate::transport::FspecBackend;

pub(crate) async fn run_session_creation(
    backend: Arc<dyn FspecBackend>,
    isolated: bool,
    card: Option<String>,
    active_session: Option<watch::Sender<Option<SessionId>>>,
    originating_session: Option<SessionId>,
    sender: UnboundedSender<Action>,
) {
    let result = if isolated {
        backend
            .create_isolated_session(None)
            .await
            .map(|info| info.session_id)
    } else {
        backend.create_session(None).await
    };
    let refresh = match result {
        Ok(session) => {
            let created = !session.value.trim().is_empty();
            if created {
                if let Some(active_session) = active_session {
                    let _ = active_session.send(Some(session.clone()));
                }
            }
            let _ = sender.send(super::session_creation::post_create_session_with_work_unit(
                session, card,
            ));
            created
        }
        Err(error) => {
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
            true
        }
    };
    if refresh {
        super::session_storage::publish_manifest_issues(backend.as_ref(), &sender, false).await;
    }
}
