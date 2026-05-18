//! App::dispatch routing for RPC-020 Action variants:
//! SlashCommandSelected, SearchFiles, FileSearchResults.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. Routing is invoked from `App::dispatch`'s
//! match arm via these explicit helper methods.
//!
//! Also hosts `handle_input_submitted` (the AgentView submit handler)
//! to keep the main `dispatch.rs` orchestrator under the 300-LoC
//! ceiling — both files cluster App::dispatch helper impls that touch
//! the AgentView path.

use crate::components::Action;
use crate::components::help_dialog::HelpDialog;
use crate::views::agent::slash_commands::SlashCommandAction;

use super::state::App;

impl App {
    /// Route a `SlashCommandSelected(action)` press. `Help` pushes the
    /// HelpDialog onto the compositor; `Clear` resets the AgentView's
    /// scrollback + input; `Quit` flips `should_quit`. Every other
    /// variant surfaces a `[notice]` scrollback line so the user knows
    /// the command will land in a future RPC card.
    pub(crate) fn handle_slash_command(&mut self, action: SlashCommandAction) {
        match action {
            SlashCommandAction::Help => {
                if !self.compositor.contains("help-dialog") {
                    self.compositor.push(Box::new(HelpDialog::new()));
                }
            }
            SlashCommandAction::Clear => {
                self.navigator
                    .agent
                    .reset_scrollback(&mut self.agent_view_store);
            }
            SlashCommandAction::Quit => {
                self.should_quit = true;
            }
            SlashCommandAction::Resume => {
                // RPC-026: route into the resume mode-view helper. Direct
                // invocation rather than self.action_tx.send(...) so the
                // view state lands in this dispatch tick.
                self.handle_open_resume_view();
            }
            SlashCommandAction::Search => {
                // RPC-026: route into the search mode-view helper.
                self.handle_open_search_view();
            }
            other => {
                let name = other.name();
                self.navigator.agent.push_line(
                    &mut self.agent_view_store,
                    format!("[notice] /{name} not yet implemented in Rust TUI"),
                );
            }
        }
    }

    /// Spawn a `backend.search_files(prefix, 20)` task and dispatch
    /// `Action::FileSearchResults(matches)` on success. Uses
    /// `Handle::try_current` so synchronous unit tests (which dispatch
    /// without a Tokio runtime) get a graceful no-op rather than a
    /// panic.
    pub(crate) fn handle_search_files(&mut self, prefix: String) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        let backend = self.backend.clone();
        let action_tx = self.action_tx.clone();
        let handle = tokio::spawn(async move {
            if let Ok(matches) = backend.search_files(prefix, 20).await {
                let _ = action_tx.send(Action::FileSearchResults(matches));
            }
        });
        self.pending_tasks.push(handle);
    }

    /// Fold a backend file-search result into the open popup.
    pub(crate) fn handle_file_search_results(&mut self, matches: Vec<String>) {
        self.navigator.agent.set_file_search_results(matches);
    }

    /// Spawn `backend.send_input` for the AgentViewStore's current
    /// session. Handles the no-session-manager stub case by surfacing a
    /// notice line in the scrollback rather than dispatching the call.
    ///
    /// Moved out of `app/dispatch.rs` as part of RPC-020 to keep the
    /// orchestrator under the 300-LoC ceiling.
    pub(crate) fn handle_input_submitted(&mut self, text: String) {
        self.navigator
            .agent
            .push_line(&mut self.agent_view_store, format!("user> {text}"));
        let Some(session) = self.agent_view_store.current_session().cloned() else {
            return;
        };
        if session.value == "rpc-no-session-manager" {
            self.navigator.agent.push_line(
                &mut self.agent_view_store,
                "[notice] no LLM session manager attached — input recorded but \
                 not sent to a model.",
            );
            return;
        }
        let backend = self.backend.clone();
        let session_for_send = session.clone();
        let text_for_send = text.clone();
        tokio::spawn(async move {
            let _ = backend.send_input(session_for_send, text_for_send).await;
        });
        // RPC-025: fire-and-forget persistence_add_history + reset the
        // per-session HistoryNavState so the next Shift+↑ pulls a fresh
        // snapshot from disk.
        self.handle_input_submitted_persistence(session, text);
    }
}
