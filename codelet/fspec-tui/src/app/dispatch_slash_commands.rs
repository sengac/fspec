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

use crate::components::create_session_dialog::CreateSessionOption;
use crate::components::help_dialog::HelpDialog;
use crate::components::Action;
use crate::views::agent::slash_commands::SlashCommandAction;

use super::slash_parser::{parse_slash_command, SlashCommandParse};
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

    /// Spawn `backend.send_input` for the AgentViewStore's current
    /// session. Handles the no-session-manager stub case by surfacing a
    /// notice line in the scrollback rather than dispatching the call.
    ///
    /// RPC-022 intercepts `/model`, `/thinking`, `/role …` via
    /// `parse_slash_command` before forwarding to `send_input`.
    /// RPC-078 deletes the sync `push_line("user> …")` so UserInput
    /// only lands via the `StreamChunk::UserInput` broadcast path.
    pub(crate) fn handle_input_submitted(&mut self, text: String) {
        let Some(session) = self.agent_view_store.current_session().cloned() else {
            // No session: silently drop. The TS Ink TUI does the same.
            return;
        };

        // RPC-022 slash-command interception.
        match parse_slash_command(&text) {
            SlashCommandParse::OpenModelDialog => {
                let _ = self.action_tx.send(Action::OpenModelSelectorView); // RPC-337 mode-view
                return;
            }
            SlashCommandParse::OpenThinkingDialog => {
                self.handle_open_thinking_dialog();
                return;
            }
            SlashCommandParse::ClearRole => {
                self.handle_set_session_role(session, None);
                return;
            }
            SlashCommandParse::OpenRoleDialog => {
                // RPC-063: bare /role (or trailing-space empty arg)
                // opens the RoleDialog seeded from the AgentViewStore.
                self.handle_open_role_dialog();
                return;
            }
            SlashCommandParse::SetRole(role) => {
                self.handle_set_session_role(session, Some(role));
                return;
            }
            SlashCommandParse::SetThinkingLevel(level) => {
                // RPC-048: `/thinking off|low|med|medium|high` sets
                // the per-session reasoning level inline without
                // opening the picker dialog. Routes through the
                // existing RPC-022 helper so backend.set_thinking_level
                // is awaited and a follow-up backend.get_thinking_level
                // refreshes AgentViewStore.thinking_level_for(session)
                // via Action::ThinkingLevelLoaded.
                self.handle_thinking_level_selected(session, level);
                return;
            }
            SlashCommandParse::InvalidThinkingLevel(other) => {
                // RPC-048: `/thinking <unknown>` surfaces an `[error]`
                // notice into the focused session's scrollback. The
                // arg is already trimmed + lowercased by the parser so
                // the notice text is stable regardless of how the user
                // typed it. NO backend call fires and no dialog is
                // pushed.
                self.navigator.agent.push_line(
                    &mut self.agent_view_store,
                    format!("[error] unknown thinking level: {other}"),
                );
                return;
            }
            SlashCommandParse::ScheduleSubcommand(sub) => {
                // RPC-058: route the parsed `/schedule …` subcommand
                // through the dedicated dispatcher in dispatch_slash_schedule.rs.
                let _ = self.action_tx.send(Action::ScheduleSubcommandParsed(sub));
                return;
            }
            SlashCommandParse::LoopSubcommand(sub) => {
                // RPC-059: route to dispatch_slash_loop.rs.
                let _ = self.action_tx.send(Action::LoopSubcommandParsed(sub));
                return;
            }
            SlashCommandParse::ContinueSubcommand(sub) => {
                // CONT-002: apply directly in this dispatch tick —
                // handler body lives in dispatch_slash_continue.rs.
                self.handle_continue_subcommand(sub);
                return;
            }
            SlashCommandParse::GoalSubcommand(sub) => {
                // CONT-003: apply directly in this dispatch tick —
                // handler body lives in dispatch_slash_goal.rs.
                self.handle_goal_subcommand(sub);
                return;
            }
            SlashCommandParse::NotASlashCommand => {}
        }

        // RPC-078: the sync `push_line("user> …")` is deleted — the
        // user line lands via the `StreamChunk::UserInput` broadcast
        // path (background_session::send_input) so we never duplicate.
        // The stub `rpc-no-session-manager` session has no broadcaster
        // so it pushes a `You:` line + `[notice]` here manually.
        if session.value == "rpc-no-session-manager" {
            self.navigator
                .agent
                .push_line(&mut self.agent_view_store, format!("You: {text}"));
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
        // Guard sync test dispatchers (no tokio runtime).
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn(async move {
                let _ = backend.send_input(session_for_send, text_for_send).await;
            });
        }
        // RPC-052: clear the durable pending-input draft now that the
        // text has been submitted. Fire-and-forget — errors silently
        // logged via tracing. Helper lives in dispatch_pending_input.rs.
        self.spawn_clear_pending_input(session.clone());
        // RPC-025: fire-and-forget persistence_add_history + reset the
        // per-session HistoryNavState so the next Shift+↑ pulls a fresh
        // snapshot from disk.
        self.handle_input_submitted_persistence(session, text);
    }
}
