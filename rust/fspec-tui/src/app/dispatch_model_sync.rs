use codelet_rpc_types::{ModelInfo, SessionId};
use tokio::sync::oneshot;

use codelet_sessions::profile_limits::MissingProfileLimit;

use crate::components::missing_profile_limit_dialog::MissingLimitRetry;
use crate::components::Action;

use super::state::App;

/// BUG-576: the session's `(section, model)`, when listed, and its ModelInfo.
type ModelSelectionResult = Result<(Option<(String, String)>, ModelInfo), String>;

#[derive(Default)]
pub(crate) struct ModelSync {
    revision: u64,
    /// BUG-576: how many model selections were made for the session.
    selections: u64,
    tail: Option<oneshot::Receiver<()>>,
}

/// RPC-427: the `list_sessions` project path that lists every project.
const UNSCOPED_SESSION_LISTING: &str = "";

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
        if selection.is_some() {
            sync.selections = sync.selections.wrapping_add(1);
        }
        let revision = sync.revision;
        let selected = sync.selections;
        let (finished_tx, finished_rx) = oneshot::channel();
        let predecessor = sync.tail.replace(finished_rx);
        // RPC-455: the session is looked up by id in its own project. Until
        // that project is observed, the lookup uses RPC-427's unscoped
        // listing (the empty path = every project), which still finds the
        // session by id; it never stands in for a failed lookup.
        let project = match self.agent_view_store.session_project(&session) {
            Some(project) => project.to_owned(),
            None => UNSCOPED_SESSION_LISTING.to_owned(),
        };
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            if let Some(predecessor) = predecessor {
                let _ = predecessor.await;
            }
            if let Some((provider, model)) = selection {
                if let Err(error) = backend
                    .set_session_model(session.clone(), provider.clone(), model.clone())
                    .await
                {
                    let error = error.to_string();
                    // BUG-547: a profile model missing a limit asks the user.
                    let action = match MissingProfileLimit::find_in(&error) {
                        Some(limit) => Action::OpenMissingProfileLimitPrompt {
                            limit,
                            retry: MissingLimitRetry::ModelSelected {
                                session: Some(session.clone()),
                                provider,
                                model,
                                selection: selected,
                            },
                        },
                        None => Action::EmitSessionNotice(
                            session.clone(),
                            format!("[error] Model selection failed: {error}"),
                        ),
                    };
                    let _ = action_tx.send(action);
                }
            }
            let result = async {
                let sessions = backend.list_sessions(project).await?;
                let model = sessions
                    .into_iter()
                    .find(|info| info.id == session.value)
                    .and_then(|info| Some((info.provider_id?, info.model_id?)));
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
        result: &ModelSelectionResult,
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
                if let Some((section, model)) = model {
                    self.agent_view_store
                        .set_selected_model(session.clone(), (section.clone(), model.clone()));
                    self.navigator
                        .model_selector
                        .reconcile_current_model(session, (section, model));
                }
                self.dispatch(Action::ModelInfoLoaded(session.clone(), info.clone()));
            }
            Err(error) => self.handle_emit_session_notice(
                session,
                format!("[error] Model metadata refresh failed: {error}"),
            ),
        }
    }

    /// BUG-576: whether a model was selected for `session` after the
    /// selection numbered `selection`.
    pub(crate) fn selected_since(&self, session: &SessionId, selection: u64) -> bool {
        self.model_sync
            .get(session)
            .is_some_and(|sync| sync.selections != selection)
    }
}
