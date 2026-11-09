//! Catch-all routing + small RPC-406 prompt helpers for the
//! RPC-053 / RPC-406 Action variants.
//!
//! Features: spec/features/pause-and-hitl-dialogs.feature,
//! spec/features/inline-tool-approval-pause-prompt.feature
//!
//! Factored out of `app/dispatch_pause_hitl.rs` so that file stays
//! under the 300-LoC ceiling pinned by `pause_hitl_rpc053`. The pinned
//! helper bodies (`handle_pause_chunk`, `handle_pause_confirmed`,
//! `handle_pause_triple`, `handle_pause_human_action`, `handle_hitl_submitted`,
//! `handle_pause_cleared`) stay in `dispatch_pause_hitl.rs`; this file
//! hosts the state-fold / navigation / Enter-mapping helpers and the
//! Action router (falling through to `try_dispatch_hitl_prompt`).

use codelet_rpc_types::{ApprovalChoice, PauseState, SessionId};
use ratatui::style::Color;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-406: fold a fetched PauseState into the per-session store
    /// slot. The AgentView paints the inline prompt from this slot on
    /// the next frame (only when the paused session is focused).
    pub(crate) fn handle_pause_state_fetched(
        &mut self,
        session_id: SessionId,
        route: codelet_rpc_types::RequestRoute,
        state: PauseState,
    ) {
        self.agent_view_store
            .set_pause_state(session_id, route, state);
        self.should_render = true;
    }

    /// BUG-394: a Paused probe read failed — log it and show it in the
    /// session scrollback so a stuck session never looks idle.
    pub(crate) fn handle_paused_read_failed(
        &mut self,
        session_id: &SessionId,
        read: &str,
        error: &str,
    ) {
        tracing::error!(
            target: "bug394",
            session = &session_id.value,
            read,
            error,
            "Paused probe read failed",
        );
        let notice = format!("[error] {read} failed while the session is paused: {error}");
        match self.agent_view_store.session_context_mut_for(session_id) {
            Some(context) => {
                context.push_notice_line(notice, Color::Red);
                self.should_render = true;
            }
            None => tracing::error!(
                target: "bug394",
                session = &session_id.value,
                "no open session to show the failed Paused probe read",
            ),
        }
    }

    /// BUG-395: a paused-prompt write failed in its task — report it and
    /// read the prompt back so a still-paused session shows it again.
    pub(crate) fn handle_paused_write_failed(
        &mut self,
        session_id: &SessionId,
        write: &str,
        error: &str,
    ) {
        self.report_paused_write_failed(session_id, write, error);
        self.handle_pause_chunk(session_id.clone());
    }

    /// BUG-395: log a failed paused-prompt write at error level and show
    /// it as a red notice in the session scrollback.
    pub(crate) fn report_paused_write_failed(
        &mut self,
        session_id: &SessionId,
        write: &str,
        error: &str,
    ) {
        tracing::error!(
            target: "bug395",
            session = &session_id.value,
            write,
            error,
            "paused prompt write failed",
        );
        let notice = format!("[error] {write} failed: {error}");
        match self.agent_view_store.session_context_mut_for(session_id) {
            Some(context) => {
                context.push_notice_line(notice, Color::Red);
                self.should_render = true;
            }
            None => tracing::error!(
                target: "bug395",
                session = &session_id.value,
                "no open session to show the failed paused prompt write",
            ),
        }
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
            Action::PauseStateFetched {
                session_id,
                route,
                state,
            } => {
                self.handle_pause_state_fetched(session_id.clone(), route.clone(), state.clone());
            }
            Action::PausedReadFailed {
                session_id,
                read,
                error,
            } => self.handle_paused_read_failed(session_id, read, error),
            Action::PausedWriteFailed {
                session_id,
                write,
                error,
            } => self.handle_paused_write_failed(session_id, write, error),
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
            Action::PauseHumanAction { session_id, reply } => {
                self.handle_pause_human_action(session_id.clone(), *reply);
            }
            _ => return self.try_dispatch_hitl_prompt(action),
        }
        true
    }
}
