//! CONT-003 — `/goal` dispatch handler.
//!
//! Feature: spec/features/goal-command-surface.feature
//!
//! Applies a parsed [`GoalSubcommand`] to the focused session's cached
//! goal state, prints the new state (or the rejection/error notice) into
//! the scrollback, and — when the state changed — round-trips the absolute
//! `(text, verify)` pair to the backend via `FspecBackend::set_goal_state`
//! (the `/continue` fire-and-forget pattern: only the Err branch emits,
//! via `Action::EmitSessionNotice` so the error lands on the originating
//! session even after a focus switch).
//!
//! `Show` and refused `Verify`/no-op `Clear` never reach the backend —
//! state unchanged.

use crate::components::Action;

use super::goal_parser::{apply_goal_subcommand, GoalSubcommand};
use super::state::App;

impl App {
    /// Apply a `/goal …` subcommand for the focused session. Bare palette
    /// picks route here with [`GoalSubcommand::Show`].
    pub(crate) fn handle_goal_subcommand(&mut self, sub: GoalSubcommand) {
        let Some(session_id) = self.agent_view_store.current_session().cloned() else {
            // No session: silently drop (matches the /continue arm).
            return;
        };

        let goal = self.agent_view_store.goal_state_for(&session_id);
        let (enabled, budget) = self.agent_view_store.continue_state_for(&session_id);
        // CONT-008: bare /goal shows the REAL per-turn counters from the
        // CONT-007 live snapshot cache ((0, 0) when no stream has pushed
        // one since the last slash change — truthful fallback).
        let live_counters = self
            .agent_view_store
            .continue_live_for(&session_id)
            .map(|l| (l.nudges_used, l.done_rejections))
            .unwrap_or((0, 0));
        let outcome = apply_goal_subcommand(goal, enabled, budget, live_counters, &sub);

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
        self.agent_view_store
            .set_goal_state(session_id.clone(), outcome.goal.clone());
        // CONT-007: drop the stale live counter — this change never flows
        // through an active stream; the next TurnStart emission re-syncs.
        self.agent_view_store.clear_continue_live(&session_id);

        // Backend round-trip (fire-and-forget; /continue pattern).
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let new_goal = outcome.goal;
        let handle = tokio::spawn(async move {
            if let Err(e) = backend.set_goal_state(session_id.clone(), new_goal).await {
                let _ = action_tx.send(Action::EmitSessionNotice(
                    session_id,
                    format!("[error] /goal failed: {e}"),
                ));
            }
        });
        self.pending_tasks.push(handle);
    }
}
