use codelet_rpc_types::SessionId;
use tokio::sync::oneshot;

use crate::components::Action;

use super::continue_parser::ContinueSubcommand;
use super::state::App;

#[derive(Default)]
pub(crate) struct ContinueSync {
    pub pending: Vec<ContinueSubcommand>,
    loading: bool,
    write_tail: Option<oneshot::Receiver<()>>,
}

impl App {
    pub(crate) fn try_dispatch_continue_sync(&mut self, action: &Action) -> bool {
        let Action::ContinueStateLoaded(session, revision, result) = action else {
            return false;
        };
        self.handle_continue_state_loaded(session, *revision, result.clone());
        true
    }

    pub(crate) fn spawn_hydrate_continue_state(&mut self, session: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let sync = self.continue_sync.entry(session.clone()).or_default();
        if sync.loading {
            return;
        }
        sync.loading = true;
        let (finished_tx, finished_rx) = oneshot::channel();
        let predecessor = sync.write_tail.replace(finished_rx);
        let revision = self.agent_view_store.continue_revision_for(&session);
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            if let Some(predecessor) = predecessor {
                let _ = predecessor.await;
            }
            let result = backend
                .get_continue_state(session.clone())
                .await
                .map(Some)
                .map_err(|error| error.to_string());
            let _ = action_tx.send(Action::ContinueStateLoaded(session, revision, result));
            let _ = finished_tx.send(());
        }));
    }

    pub(crate) fn handle_continue_state_loaded(
        &mut self,
        session: &SessionId,
        revision: u64,
        result: Result<Option<(bool, u32)>, String>,
    ) {
        let sync = self.continue_sync.entry(session.clone()).or_default();
        sync.loading = false;
        let pending = std::mem::take(&mut sync.pending);
        if self.agent_view_store.continue_revision_for(session) == revision {
            match result {
                Ok(Some((enabled, budget))) => {
                    self.agent_view_store
                        .set_continue_state(session.clone(), enabled, budget)
                }
                Ok(None) => self.agent_view_store.clear_continue_state(session),
                Err(error) => self.handle_emit_session_notice(
                    session,
                    format!("[error] /continue state unavailable: {error}"),
                ),
            }
        }
        if self
            .agent_view_store
            .continue_state_loaded_for(session)
            .is_some()
        {
            for sub in pending {
                self.apply_session_continue_subcommand(session.clone(), sub);
            }
        }
    }

    pub(crate) fn spawn_continue_write(
        &mut self,
        session: SessionId,
        state: (bool, u32),
        previous: Option<(bool, u32)>,
    ) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let revision = self.agent_view_store.continue_revision_for(&session);
        let (finished_tx, finished_rx) = oneshot::channel();
        let predecessor = self
            .continue_sync
            .entry(session.clone())
            .or_default()
            .write_tail
            .replace(finished_rx);
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            if let Some(predecessor) = predecessor {
                let _ = predecessor.await;
            }
            if let Err(error) = backend
                .set_continue_state(session.clone(), state.0, state.1)
                .await
            {
                let restored = backend
                    .get_continue_state(session.clone())
                    .await
                    .ok()
                    .or(previous);
                let _ = action_tx.send(Action::ContinueStateLoaded(
                    session.clone(),
                    revision,
                    Ok(restored),
                ));
                let _ = action_tx.send(Action::EmitSessionNotice(
                    session,
                    format!("[error] /continue failed: {error}"),
                ));
            }
            let _ = finished_tx.send(());
        }));
    }
}
