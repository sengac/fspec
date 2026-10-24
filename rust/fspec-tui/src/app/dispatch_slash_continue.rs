//! CONT-002 — `/continue` dispatch handler.
//!
//! Feature: spec/features/continue-command-surface.feature
//!
//! Applies a parsed [`ContinueSubcommand`] to the focused session's cached
//! auto-continue state, prints the new state (or the rejection/error
//! notice) into the scrollback, and — when the state changed — round-trips
//! the absolute `(enabled, budget)` pair to the backend via
//! `FspecBackend::set_continue_state`. Unknown relative commands wait for
//! hydration; ordered writes reconcile failures using revision-guarded
//! responses bound to the originating session.
//!
//! `RejectZero` / `Invalid` never reach the backend — state unchanged.

use super::continue_parser::{apply_continue_subcommand, ContinueSubcommand};
use super::state::App;

impl App {
    /// Apply a `/continue …` subcommand for the focused session. Bare
    /// palette picks route here with [`ContinueSubcommand::Toggle`].
    pub(crate) fn handle_continue_subcommand(&mut self, sub: ContinueSubcommand) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            // No session: silently drop (matches the /compact arm).
            return;
        };

        let pending = !self
            .continue_sync
            .entry(session_id.clone())
            .or_default()
            .pending
            .is_empty();
        let unknown_relative = self
            .agent_view_store
            .continue_state_loaded_for(&session_id)
            .is_none()
            && matches!(sub, ContinueSubcommand::Toggle | ContinueSubcommand::Off);
        if pending || unknown_relative {
            self.continue_sync
                .entry(session_id.clone())
                .or_default()
                .pending
                .push(sub);
            self.spawn_hydrate_continue_state(session_id);
            return;
        }
        self.apply_session_continue_subcommand(session_id, sub);
    }

    pub(crate) fn apply_session_continue_subcommand(
        &mut self,
        session_id: codelet_rpc_types::SessionId,
        sub: ContinueSubcommand,
    ) {
        let previous = self.agent_view_store.continue_state_loaded_for(&session_id);
        let (enabled, budget) = self.agent_view_store.continue_state_for(&session_id);
        // CONT-003: /continue off is refused while a goal is active.
        let goal_active = self.agent_view_store.goal_state_for(&session_id).is_some();
        let outcome = apply_continue_subcommand(enabled, budget, goal_active, &sub);

        // Always print the new state / error notice.
        let prefix = if outcome.changed {
            "[notice]"
        } else {
            "[error]"
        };
        self.handle_emit_session_notice(&session_id, format!("{prefix} {}", outcome.message));

        if !outcome.changed {
            return;
        }

        // Cache the new state so the status-bar indicator updates this frame.
        self.agent_view_store.set_continue_state(
            session_id.clone(),
            outcome.enabled,
            outcome.budget,
        );
        // CONT-007: drop the stale live counter — this change never flows
        // through an active stream; the next TurnStart emission re-syncs.
        self.agent_view_store.clear_continue_live(&session_id);

        self.spawn_continue_write(session_id, (outcome.enabled, outcome.budget), previous);
    }
}
