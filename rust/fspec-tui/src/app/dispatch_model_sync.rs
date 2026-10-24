use codelet_rpc_types::{ModelInfo, SessionId};
use tokio::sync::oneshot;

use crate::components::Action;

use super::state::App;

#[derive(Default)]
pub(crate) struct ModelSync {
    revision: u64,
    tail: Option<oneshot::Receiver<()>>,
}

impl App {
    pub(crate) fn spawn_model_sync(
        &mut self,
        session: SessionId,
        selection: Option<(String, String)>,
    ) {
        if selection.is_some() {
            if let Some(context) = self.agent_view_store.session_context_mut_for(&session) {
                context.provider_waiting = None;
            }
        }
        if tokio::runtime::Handle::try_current().is_err() {
            if selection.is_some() {
                self.handle_emit_session_notice(
                    &session,
                    "[error] Model selection unavailable without an async runtime".into(),
                );
            }
            return;
        }
        let sync = self.model_sync.entry(session.clone()).or_default();
        sync.revision = sync.revision.wrapping_add(1);
        let revision = sync.revision;
        let (finished_tx, finished_rx) = oneshot::channel();
        let predecessor = sync.tail.replace(finished_rx);
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            if let Some(predecessor) = predecessor {
                let _ = predecessor.await;
            }
            if let Some((provider, model)) = selection {
                if let Err(error) = backend
                    .set_session_model(session.clone(), provider, model)
                    .await
                {
                    let _ = action_tx.send(Action::EmitSessionNotice(
                        session.clone(),
                        format!("[error] Model selection failed: {error}"),
                    ));
                }
            }
            let result = async {
                let sessions = backend.list_sessions(String::new()).await?;
                let model = sessions
                    .into_iter()
                    .find(|info| info.id == session.value)
                    .and_then(|info| info.model_id);
                let info = backend.get_model_info(session.clone()).await?;
                Ok::<_, anyhow::Error>((model, info))
            }
            .await
            .map_err(|error| error.to_string());
            let _ = action_tx.send(Action::ModelSelectionLoaded(session, revision, result));
            let _ = finished_tx.send(());
        }));
    }

    pub(crate) fn handle_model_selection_loaded(
        &mut self,
        session: &SessionId,
        revision: u64,
        result: &Result<(Option<String>, ModelInfo), String>,
    ) {
        if self
            .model_sync
            .get(session)
            .is_none_or(|sync| sync.revision != revision)
        {
            return;
        }
        match result {
            Ok((model, info)) => {
                if let Some(model) = model {
                    self.agent_view_store
                        .set_selected_model_id(session.clone(), model.clone());
                    self.navigator
                        .model_selector
                        .reconcile_current_model(session, model.clone());
                }
                self.dispatch(Action::ModelInfoLoaded(session.clone(), info.clone()));
            }
            Err(error) => self.handle_emit_session_notice(
                session,
                format!("[error] Model metadata refresh failed: {error}"),
            ),
        }
    }
}
