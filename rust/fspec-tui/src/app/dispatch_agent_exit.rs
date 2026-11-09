//! App::dispatch routing for the close-dialog choices (RPC-098, TUI-124).
//!
//! Feature: spec/features/agentview-esc-exit-confirmation-dialog.feature
//! Feature: spec/features/agent-close-and-start-dialogs-say-exactly-what-will-happen.feature
//!
//! Routing decision tree for `Action::AgentExitChoice { choice }`:
//!
//! ```text
//! choice = Cancel        → the gathered close facts are dropped (the
//!                          dialog removed itself via its Callback)
//! choice = CloseSession  → take the facts the dialog stated
//!                          (`App::pending_close`) and spawn the close:
//!
//!                          1. A bot's sub-agents, one backend close
//!                             (`destroy_session`) at a time, in order
//!                             (product owner rule, 2026-11-07).
//!                          2. The session itself, only when every
//!                             sub-agent closed.
//!                          3. The first failure stops the close: the
//!                             rest stay open and the report names how
//!                             many sub-agents were closed and which
//!                             session could not be closed.
//!                          4. `Action::CloseFinished` folds every closed
//!                             session through the session-removed cleanup
//!                             (dispatch_agent_close.rs) — a tab is removed
//!                             only after its backend close succeeded.
//! ```
//!
//! The backend close stops the running turn and keeps the saved history
//! (resumable with /resume); an isolated session's worktree stays until
//! /worktrees prune removes it.

use codelet_rpc_types::SessionId;

use crate::components::exit_confirmation_dialog::{CloseFacts, ExitChoice};
use crate::components::Action;

use super::state::App;

impl App {
    /// Route `Action::AgentExitChoice { choice }` per the table above.
    pub(crate) fn handle_agent_exit_choice(&mut self, choice: ExitChoice) {
        let facts = self.pending_close.take();
        let (ExitChoice::CloseSession, Some(facts)) = (choice, facts) else {
            return;
        };
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            let (closed, failure) = close_in_order(backend.as_ref(), &facts).await;
            let _ = action_tx.send(Action::CloseFinished {
                facts,
                closed,
                failure,
            });
        });
        self.pending_tasks.push(handle);
    }
}

/// Close the bot's sub-agents and then the session, stopping at the first
/// failure. Returns the sessions closed, in order, and the failure report.
async fn close_in_order(
    backend: &dyn crate::transport::FspecBackend,
    facts: &CloseFacts,
) -> (Vec<SessionId>, Option<String>) {
    let word = facts.kind.word();
    let total = facts.closing_sub_agents.len();
    let mut closed = Vec::new();
    for sub in &facts.closing_sub_agents {
        if let Err(error) = backend.destroy_session(sub.id.clone()).await {
            let failure = format!(
                "The {word} was not closed: sub-agent {} could not be closed: {error:#}. {} of {total} sub-agents were closed.",
                sub.id.value,
                closed.len()
            );
            return (closed, Some(failure));
        }
        closed.push(sub.id.clone());
    }
    if let Err(error) = backend.destroy_session(facts.session.clone()).await {
        let failure = match closed.len() {
            0 => format!("The {word} was not closed: {error:#}"),
            1 => format!("The {word} was not closed: {error:#}. Its 1 sub-agent was closed."),
            count => {
                format!("The {word} was not closed: {error:#}. Its {count} sub-agents were closed.")
            }
        };
        return (closed, Some(failure));
    }
    closed.push(facts.session.clone());
    (closed, None)
}
