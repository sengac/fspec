//! BUG-576: model selections made with no session open. The first one
//! saves the default model and creates a session; a selection made while
//! that session is being created waits for it and is then applied to it,
//! so only one session is created.
//!
//! Feature: spec/features/model-selector-keeps-its-place-across-loads.feature

use std::sync::Arc;

use codelet_rpc_types::SessionId;

use crate::components::Action;

use super::session_creation_tasks::{create_session_reporting, CreationRequest};
use super::state::App;

/// The session created for a burst of no-session selections, shared by
/// every selection task of the burst; `None` until it is created.
pub(crate) type NoSessionSelection = Arc<tokio::sync::Mutex<Option<SessionId>>>;

impl App {
    /// PROV-118 / MODEL-006: persist the choice as the DEFAULT model (the
    /// next `create_session` is otherwise declined, PROV-101), then create
    /// the session; a creation error opens the session failure dialog
    /// (BUG-410). BUG-576: a selection made while an earlier one is still
    /// creating its session is applied to that session instead.
    pub(crate) fn handle_model_selected_no_session(
        &mut self,
        provider_id: String,
        model_id: String,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            tracing::warn!(target: "model_select", "[MODEL-SELECT] no runtime -> skip default+retry");
            return;
        }
        let selection = self.no_session_selection();
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let active_session_tx = self.active_session_tx.clone();
        let workspace = self
            .board_store
            .selected_other_workspace()
            .map(str::to_owned);
        let model_string = format!("{provider_id}/{model_id}");
        self.pending_tasks.push(tokio::spawn(async move {
            let mut created = selection.lock().await;
            if let Some(session) = created.clone() {
                let _ = action_tx.send(Action::ModelSelected(Some(session), provider_id, model_id));
                return;
            }
            if let Err(e) = backend.set_default_model(model_string.clone()).await {
                tracing::error!(target: "model_select", error = %e, "[MODEL-SELECT] set_default_model FAILED");
                let _ = action_tx.send(Action::SessionOperationFailed(format!(
                    "Model selection failed: {e}"
                )));
                return;
            }
            tracing::info!(target: "model_select", model = %model_string, "[MODEL-SELECT] set_default_model OK");
            *created = create_session_reporting(
                backend,
                CreationRequest {
                    isolated: false,
                    role: None,
                    workspace,
                    card: None,
                },
                Some(active_session_tx),
                None,
                action_tx,
            )
            .await;
        }));
    }

    /// The burst still in flight (a task holds it), or a new one.
    fn no_session_selection(&mut self) -> NoSessionSelection {
        match &self.no_session_selection {
            Some(selection) if Arc::strong_count(selection) > 1 => selection.clone(),
            _ => {
                let selection = NoSessionSelection::default();
                self.no_session_selection = Some(selection.clone());
                selection
            }
        }
    }
}
