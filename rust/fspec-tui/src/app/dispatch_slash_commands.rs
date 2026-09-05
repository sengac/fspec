//! App::dispatch routing for RPC-020 Action variants:
//! SlashCommandSelected, SearchFiles, FileSearchResults.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling. Routing is invoked from `App::dispatch`'s
//! match arm via these explicit helper methods.
//!
//! `handle_slash_command` (the registry handler behind the popup-pick
//! path) lives here — the source-shape cards (rpc026, source_shape_rpc020,
//! rpc054, source_shape_rpc060) pin its Resume/Search/Provider/Isolation
//! arms in this file. The typed-submit path (`handle_input_submitted`,
//! including the BUG-169 `BareCommand` catch) lives in
//! `dispatch_slash_submit.rs` so both files stay under the 300-LoC
//! ceiling.

use crate::components::create_session_dialog::CreateSessionOption;
use crate::components::help_dialog::HelpDialog;
use crate::components::Action;
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
                    self.compositor.push(Box::new(HelpDialog::for_agent()));
                }
            }
            SlashCommandAction::Clear => {
                // RPC-046: handler body lives in dispatch_slash_clear.rs.
                self.handle_slash_clear();
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
            SlashCommandAction::Model => {
                // RPC-337: open the full-screen ModelSelector mode-view.
                let _ = self.action_tx.send(Action::OpenModelSelectorView);
            }
            SlashCommandAction::Thinking => {
                // RPC-022: open the ThinkingLevelDialog seeded with the
                // cached level for the focused session.
                self.handle_open_thinking_dialog();
            }
            SlashCommandAction::Role => {
                // RPC-063: bare /role opens the RoleDialog (seeded from
                // AgentViewStore). Silent no-op with no active session.
                self.handle_open_role_dialog();
            }
            SlashCommandAction::Compact => {
                // RPC-047 wiring, amended by RPC-421:
                //
                // 1. Bare /compact with no current session is a silent
                //    no-op (no backend call, no notice emitted).
                // 2. Spawn a tokio task that awaits the round-trip
                //    (RPC-046 pattern). The Ok branch is SILENT — the
                //    RPC result is an acknowledgement whose numbers are
                //    measured before DAG injection; the `[compaction]`
                //    success notice is single-sourced from the
                //    StreamChunk::CompactionComplete handler in
                //    dispatch_stream_chunks.rs, which carries the honest
                //    post-injection numbers. Only the Err branch emits,
                //    via Action::EmitSessionNotice so the error lands on
                //    the originating session even after a focus switch.
                let Some(session_id) = self.agent_view_store.current_session().cloned() else {
                    return;
                };
                if tokio::runtime::Handle::try_current().is_err() {
                    return;
                }
                let backend = self.backend.clone();
                let action_tx = self.action_tx.clone();
                let handle = tokio::spawn(async move {
                    if let Err(e) = backend.compact_session(session_id.clone()).await {
                        let _ = action_tx.send(Action::EmitSessionNotice(
                            session_id,
                            format!("[error] /compact failed: {e}"),
                        ));
                    }
                });
                self.pending_tasks.push(handle);
            }
            SlashCommandAction::Detach => {
                // RPC-050: /detach handler — three documented paths
                // (no session / no binding / Ok-Err round-trip) live
                // in dispatch_work_unit_binding.rs::handle_slash_detach.
                self.handle_slash_detach();
            }
            SlashCommandAction::Provider => {
                // RPC-054: open the ProviderSettingsView. Singular
                // `/provider` only — the TypeScript Ink reference
                // (slashCommands.ts) defines exactly one entry whose
                // `name` is `'provider'`; no `/providers` alias.
                let _ = self.action_tx.send(Action::OpenProviderSettingsView);
            }
            SlashCommandAction::Debug => {
                // RPC-055: handler body lives in dispatch_slash_debug.rs.
                self.handle_slash_debug();
            }
            SlashCommandAction::Blocklist => {
                // RPC-056: open the BlocklistView. dispatch_blocklist.rs
                // handles the round-trip via Action::OpenBlocklistView
                // → backend.blocklist_list() → Action::BlocklistRulesLoaded.
                let _ = self.action_tx.send(Action::OpenBlocklistView);
            }
            SlashCommandAction::MergeWorktree => {
                // RPC-057: handler body lives in dispatch_merge_worktree.rs.
                self.handle_slash_merge_worktree();
            }
            SlashCommandAction::Schedule => {
                // RPC-058: handler body lives in dispatch_slash_schedule.rs.
                // Bare popup pick emits the static help notice.
                self.handle_slash_schedule_help();
            }
            SlashCommandAction::Loop => {
                // RPC-059: dispatch_slash_loop.rs::handle_slash_loop_help.
                self.handle_slash_loop_help();
            }
            SlashCommandAction::Continue => {
                // CONT-002: bare palette pick toggles auto-continue.
                // Handler body lives in dispatch_slash_continue.rs.
                self.handle_continue_subcommand(super::continue_parser::ContinueSubcommand::Toggle);
            }
            SlashCommandAction::Goal => {
                // CONT-003: bare palette pick shows the contract state.
                // Handler body lives in dispatch_slash_goal.rs.
                self.handle_goal_subcommand(super::goal_parser::GoalSubcommand::Show);
            }
            SlashCommandAction::Update => {
                // UPD-002: bare palette pick checks + installs the latest
                // release. Handler body lives in dispatch_slash_update.rs.
                self.handle_update_subcommand(super::update_parser::UpdateSubcommand::CheckAndUpdate);
            }
            SlashCommandAction::Mux => {
                // MUX-004: /mux (palette pick or bare /mux submit) opens
                // the MuxConfigDialog. Handler body lives in
                // dispatch_mux_config.rs.
                self.handle_open_mux_config_dialog();
            }
            SlashCommandAction::Isolation => {
                // RPC-060: routed via try_dispatch_create_session_dialog in app/dispatch.rs.
                let _ = self.action_tx.send(Action::OpenCreateSessionDialog {
                    preselect: Some(CreateSessionOption::Isolated),
                });
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
}
