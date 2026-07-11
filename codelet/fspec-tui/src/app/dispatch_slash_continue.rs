//! CONT-002 — `/continue` dispatch handler.
//!
//! Feature: spec/features/continue-command-surface.feature
//!
//! Applies a parsed [`ContinueSubcommand`] to the focused session's cached
//! auto-continue state, prints the new state (or the rejection/error
//! notice) into the scrollback, and — when the state changed — round-trips
//! the absolute `(enabled, budget)` pair to the backend via
//! `FspecBackend::set_continue_state` (the `/compact` fire-and-forget
//! pattern: only the Err branch emits, via `Action::EmitSessionNotice` so
//! the error lands on the originating session even after a focus switch).
//!
//! `RejectZero` / `Invalid` never reach the backend — state unchanged.

use crate::components::Action;

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
        self.navigator.agent.push_line(
            &mut self.agent_view_store,
            format!("{prefix} {}", outcome.message),
        );

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

        // Backend round-trip (fire-and-forget; /compact pattern).
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let (new_enabled, new_budget) = (outcome.enabled, outcome.budget);
        let handle = tokio::spawn(async move {
            if let Err(e) = backend
                .set_continue_state(session_id.clone(), new_enabled, new_budget)
                .await
            {
                let _ = action_tx.send(Action::EmitSessionNotice(
                    session_id,
                    format!("[error] /continue failed: {e}"),
                ));
            }
        });
        self.pending_tasks.push(handle);
    }
}
