//! MODEL-011: App handlers for the thinking dialog and `/thinking` over the
//! levels the session's model offers (backend `get_thinking_options`): the
//! dialog opens from the fetched options, `/thinking <arg>` is resolved
//! against them, and a refusal lands in the session's scrollback.
//!
//! MODEL-012: the `(default)` marker is the current model's saved level.
//!
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature
//! Feature: spec/features/thinking-level-remembered-per-model.feature

use std::sync::Arc;

use codelet_rpc_types::{SessionId, ThinkingLevel, ThinkingOptions};
use codelet_sessions::default_thinking_level_persistence::load_default_thinking_level;
use tokio::sync::mpsc::UnboundedSender;

use crate::components::{
    thinking_level_dialog::{ThinkingLevelDialog, THINKING_LEVEL_DIALOG_ID},
    Action,
};

use super::state::App;

impl App {
    /// RPC-022 / MODEL-011: fetch the levels the focused session's model
    /// offers; `Action::ThinkingOptionsLoaded` then opens the dialog. A
    /// failed fetch is shown, never replaced by a fixed level list.
    pub(crate) fn handle_open_thinking_dialog(&mut self) {
        let session_id = match self.agent_view_store.current_session().cloned() {
            Some(sid) => sid,
            None => return,
        };
        if self.compositor.contains(THINKING_LEVEL_DIALOG_ID) {
            return;
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let action = match backend.get_thinking_options(session_id.clone()).await {
                Ok(options) => Action::ThinkingOptionsLoaded(session_id, options),
                Err(error) => Action::ThinkingLevelRefused(
                    session_id,
                    format!("cannot list the thinking levels: {error}"),
                ),
            };
            let _ = action_tx.send(action);
        });
        self.pending_tasks.push(handle);
    }

    /// RPC-022 / MODEL-011: push a ThinkingLevelDialog offering `options`,
    /// seeded with the session's level and the current model's saved level
    /// (TUI-094, MODEL-012). A saved level that cannot be read marks no row
    /// and is reported in the session's scrollback.
    pub(crate) fn handle_thinking_options_loaded(
        &mut self,
        session_id: SessionId,
        options: ThinkingOptions,
    ) {
        if self.compositor.contains(THINKING_LEVEL_DIALOG_ID) {
            return;
        }
        let current = self
            .agent_view_store
            .thinking_level_for(&session_id)
            .cloned()
            .unwrap_or(ThinkingLevel::ModelDefault);
        let saved = match load_default_thinking_level(options.model()) {
            Ok(saved) => saved,
            Err(error) => {
                self.handle_thinking_level_refused(
                    session_id.clone(),
                    format!(
                        "the saved thinking level for {} cannot be read: {error}",
                        options.model()
                    ),
                );
                None
            }
        };
        let dialog = ThinkingLevelDialog::new(session_id, current, options)
            .with_default_level(saved)
            .with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
    }

    /// MODEL-011: `/thinking <arg>` — resolve the argument against the
    /// levels the session's model offers, then set it; a refusal names the
    /// model and the levels it takes.
    pub(crate) fn handle_thinking_command(&mut self, session_id: SessionId, arg: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let resolved = match backend.get_thinking_options(session_id.clone()).await {
                Ok(options) => options.resolve_command(&arg),
                Err(error) => Err(format!("cannot list the thinking levels: {error}")),
            };
            match resolved {
                Ok(level) => set_level(backend, action_tx, session_id, level).await,
                Err(reason) => {
                    let _ = action_tx.send(Action::ThinkingLevelRefused(session_id, reason));
                }
            }
        });
        self.pending_tasks.push(handle);
    }

    /// MODEL-011: show a refused level in the session's scrollback.
    pub(crate) fn handle_thinking_level_refused(&mut self, session_id: SessionId, reason: String) {
        if let Some(context) = self.agent_view_store.session_context_mut_for(&session_id) {
            context.push_line(format!("[error] {reason}"));
        }
    }
}

/// RPC-022 / MODEL-011: store `level` through the backend, then re-fetch it
/// so the SessionHeader `[T:level]` badge repaints; a refusal (the daemon
/// checks the level against the model) is shown instead.
pub(super) async fn set_level(
    backend: Arc<dyn crate::transport::FspecBackend>,
    action_tx: UnboundedSender<Action>,
    session_id: SessionId,
    level: ThinkingLevel,
) {
    if let Err(error) = backend.set_thinking_level(session_id.clone(), level).await {
        let _ = action_tx.send(Action::ThinkingLevelRefused(session_id, error.to_string()));
        return;
    }
    if let Ok(fresh) = backend.get_thinking_level(session_id.clone()).await {
        let _ = action_tx.send(Action::ThinkingLevelLoaded(session_id, fresh));
    }
}
