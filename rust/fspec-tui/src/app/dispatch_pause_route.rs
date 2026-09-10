//! Catch-all routing + small RPC-406 prompt helpers for the
//! RPC-053 / RPC-406 Action variants.
//!
//! Features: spec/features/pause-and-hitl-dialogs.feature,
//! spec/features/inline-tool-approval-pause-prompt.feature
//!
//! Factored out of `app/dispatch_pause_hitl.rs` so that file stays
//! under the 300-LoC ceiling pinned by `pause_hitl_rpc053`. The pinned
//! helper bodies (`handle_pause_chunk`, `handle_pause_confirmed`,
//! `handle_pause_triple`, `handle_pause_resumed`, `handle_hitl_submitted`,
//! `handle_pause_cleared`) stay in `dispatch_pause_hitl.rs`; this file
//! hosts the state-fold / navigation / Enter-mapping helpers and the
//! Action router (falling through to `try_dispatch_hitl_prompt`).

use codelet_rpc_types::{ApprovalChoice, PauseState, SessionId};

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-406: fold a fetched PauseState into the per-session store
    /// slot. The AgentView paints the inline prompt from this slot on
    /// the next frame (only when the paused session is focused).
    pub(crate) fn handle_pause_state_fetched(&mut self, session_id: SessionId, state: PauseState) {
        self.agent_view_store.set_pause_state(session_id, state);
        self.should_render = true;
    }

    /// RPC-406: cycle the triple-prompt selection with wraparound.
    pub(crate) fn handle_pause_prompt_nav(&mut self, session_id: &SessionId, delta: i32) {
        self.agent_view_store
            .cycle_triple_pause_selection(session_id, delta);
        self.should_render = true;
    }

    /// RPC-406: Enter on the triple prompt — read the authoritative
    /// selection from the store, map onto an ApprovalChoice, and route
    /// through `handle_pause_triple` (which clears the slot).
    pub(crate) fn handle_pause_prompt_enter(&mut self, session_id: SessionId) {
        let choice = match self
            .agent_view_store
            .triple_pause_selection_for(&session_id)
        {
            0 => ApprovalChoice::Approve,
            1 => ApprovalChoice::ApproveSession,
            _ => ApprovalChoice::Deny,
        };
        self.handle_pause_triple(session_id, choice);
    }

    /// Route the RPC-053 / RPC-406 Action variants through their
    /// helpers. Called from the catch-all arm of `App::dispatch`.
    pub(crate) fn try_dispatch_pause_hitl(&mut self, action: &Action) -> bool {
        match action {
            Action::PauseChunkReceived(sid) => {
                self.handle_pause_chunk(sid.clone());
            }
            Action::PauseCleared(sid) => {
                self.handle_pause_cleared(sid.clone());
            }
            Action::PauseStateFetched { session_id, state } => {
                self.handle_pause_state_fetched(session_id.clone(), state.clone());
            }
            Action::PausePromptNav { session_id, delta } => {
                self.handle_pause_prompt_nav(session_id, *delta);
            }
            Action::PausePromptEnter { session_id } => {
                self.handle_pause_prompt_enter(session_id.clone());
            }

            Action::PauseConfirmed { session_id, accept } => {
                self.handle_pause_confirmed(session_id.clone(), *accept);
            }
            Action::PauseTriple { session_id, choice } => {
                self.handle_pause_triple(session_id.clone(), *choice);
            }
            Action::PauseResumed { session_id } => {
                self.handle_pause_resumed(session_id.clone());
            }
            Action::HitlSubmitted {
                session_id,
                response,
            } => {
                self.handle_hitl_submitted(session_id.clone(), response.clone());
            }
            _ => return self.try_dispatch_hitl_prompt(action),
        }
        true
    }
}
