//! App::dispatch routing for RPC-052 — pending-input draft persistence,
//! reworked by BUG-549 (the composer-draft lifecycle in
//! `app/composer_draft.rs`).
//!
//! Feature: spec/features/pending-input-draft-persistence.feature
//! Feature: spec/features/composer-draft-belongs-to-one-session.feature
//!
//! Hosts the impl App helpers invoked from `app/dispatch_model_thinking_dialogs.rs`:
//!
//!   - `handle_pending_input_changed(text)`: the AgentView reports that the
//!     composer text changed. The composer is the truth for the session it
//!     is bound to, so the handler records the composer's CURRENT text
//!     through [`App::sync_composer_draft`] (in-memory mirror + per-session
//!     debounced daemon save) rather than the reported text, which can be
//!     stale once a later event moved focus. No-op without a session.
//!
//!   - `handle_seed_pending_input(session_id, text)`: code seeds a session's
//!     draft (the RPC-057 merge-conflict context). A draft change like
//!     typing: the in-memory draft, the live composer when it holds that
//!     session's draft, and a daemon save.
//!
//!   - `spawn_hydrate_pending_input(session_id)`: on attach / a new tab,
//!     read the daemon draft under the session's ordering lock — only when
//!     every scheduled write has settled — and report it as
//!     `Action::PendingInputHydrated` with the generation it was read at.
//!
//!   - `handle_pending_input_hydrated(..)`: applies the daemon draft only
//!     when no draft change was scheduled since the read (BUG-549 rule 4:
//!     text typed after focusing is never overwritten).

use std::sync::atomic::Ordering;
use std::sync::Arc;

use codelet_rpc_types::SessionId;

use crate::components::Action;
use crate::transport::FspecBackend;

use super::state::App;

impl App {
    /// RPC-052 / BUG-549: the composer text changed — record the
    /// composer's current text as the bound session's draft.
    pub(crate) fn handle_pending_input_changed(&mut self, _reported: String) {
        self.sync_composer_draft();
    }

    /// RPC-052 / BUG-549: seed `text` as `session_id`'s draft (a draft
    /// change like typing). Dropped when the session's tab is not open.
    pub(crate) fn handle_seed_pending_input(&mut self, session_id: SessionId, text: String) {
        let Some(context) = self.agent_view_store.session_context_mut_for(&session_id) else {
            return;
        };
        context.input_draft.clone_from(&text);
        if self.composer_drafts.bound() == Some(&session_id) {
            self.navigator.agent.input.set_value(&text);
            self.should_render = true;
        }
        self.schedule_draft_save(&session_id, text);
    }

    /// BUG-549: apply a hydrated daemon draft read at `generation`, unless a
    /// draft change was scheduled for the session since.
    pub(crate) fn handle_pending_input_hydrated(
        &mut self,
        session_id: SessionId,
        text: String,
        generation: u64,
    ) {
        let slot = self.composer_drafts.slot(&session_id);
        if slot.latest.load(Ordering::SeqCst) != generation {
            return;
        }
        let Some(context) = self.agent_view_store.session_context_mut_for(&session_id) else {
            return;
        };
        context.input_draft.clone_from(&text);
        if self.composer_drafts.bound() == Some(&session_id)
            && self.navigator.agent.input.value() != text
        {
            self.navigator.agent.input.set_value(&text);
            self.should_render = true;
        }
    }

    /// RPC-052 / BUG-549: read `session_id`'s daemon draft for the attach
    /// hydration. Skipped when a write is still pending (or failed): the
    /// local copy is then the newer one. Errors are logged via tracing.
    pub(crate) fn spawn_hydrate_pending_input(&mut self, session_id: SessionId) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let slot = self.composer_drafts.slot(&session_id);
        let generation = slot.latest.load(Ordering::SeqCst);
        let backend: Arc<dyn FspecBackend> = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let _order = slot.order.lock().await;
            if slot.latest.load(Ordering::SeqCst) != generation
                || slot.settled.load(Ordering::SeqCst) != generation
            {
                return;
            }
            match backend.get_pending_input(session_id.clone()).await {
                Ok(text) => {
                    let _ = action_tx.send(Action::PendingInputHydrated {
                        session_id,
                        text: text.unwrap_or_default(),
                        generation,
                    });
                }
                Err(err) => {
                    tracing::debug!(
                        target: "rpc052",
                        session = &session_id.value,
                        error = %err,
                        "get_pending_input failed (silently dropped)",
                    );
                }
            }
        });
        self.pending_tasks.push(handle);
    }
}
