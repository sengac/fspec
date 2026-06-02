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
use crate::components::create_session_dialog::CreateSessionOption;
use crate::components::help_dialog::HelpDialog;
use crate::views::agent::slash_commands::SlashCommandAction;

use codelet_rpc_types::CompactionResult;

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
                    self.compositor.push(Box::new(HelpDialog::new()));
                }
            }
            SlashCommandAction::Clear => {
                // RPC-046: handler body lives in dispatch_rpc046.rs.
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
                // RPC-022: open the ModelSelectorDialog + spawn list_providers.
                self.handle_open_model_dialog();
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
                // RPC-047: parity with the TS handleCompactCommand path
                // (src/tui/views/AgentView.tsx line ~2673). Wire the
                // slash action to backend.compact_session(session_id):
                //
                // 1. Bare /compact with no current session is a silent
                //    no-op (no backend call, no notice emitted).
                // 2. Spawn a tokio task that awaits the round-trip and
                //    routes the response into the originating session's
                //    scrollback via Action::EmitSessionNotice — so the
                //    notice lands on the right SessionContext even if
                //    the user switched tabs while the RPC was in
                //    flight (RPC-046 pattern).
                let Some(session_id) = self.agent_view_store.current_session().cloned() else {
                    return;
                };
                if tokio::runtime::Handle::try_current().is_err() {
                    return;
                }
                let backend = self.backend.clone();
                let action_tx = self.action_tx.clone();
                let handle = tokio::spawn(async move {
                    let text = match backend.compact_session(session_id.clone()).await {
                        Ok(result) => format_compaction_notice(&result),
                        Err(e) => format!("[error] /compact failed: {e}"),
                    };
                    let _ = action_tx.send(Action::EmitSessionNotice(session_id, text));
                });
                self.pending_tasks.push(handle);
            }
            SlashCommandAction::Detach => {
                // RPC-050: /detach handler — three documented paths
                // (no session / no binding / Ok-Err round-trip) live
                // in dispatch_rpc050.rs::handle_slash_detach.
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
                // RPC-055: handler body lives in dispatch_rpc055.rs.
                self.handle_slash_debug();
            }
            SlashCommandAction::Blocklist => {
                // RPC-056: open the BlocklistView. dispatch_rpc056.rs
                // handles the round-trip via Action::OpenBlocklistView
                // → backend.blocklist_list() → Action::BlocklistRulesLoaded.
                let _ = self.action_tx.send(Action::OpenBlocklistView);
            }
            SlashCommandAction::MergeWorktree => {
                // RPC-057: handler body lives in dispatch_rpc057.rs.
                self.handle_slash_merge_worktree();
            }
            SlashCommandAction::Schedule => {
                // RPC-058: handler body lives in dispatch_rpc058.rs.
                // Bare popup pick emits the static help notice.
                self.handle_slash_schedule_help();
            }
            SlashCommandAction::Loop => {
                // RPC-059: dispatch_rpc059.rs::handle_slash_loop_help.
                self.handle_slash_loop_help();
            }
            SlashCommandAction::Isolation => {
                // RPC-060: routed via try_dispatch_rpc060 in app/dispatch.rs.
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
                self.handle_open_model_dialog();
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
                // through the dedicated dispatcher in dispatch_rpc058.rs.
                let _ = self
                    .action_tx
                    .send(Action::ScheduleSubcommandParsed(sub));
                return;
            }
            SlashCommandParse::LoopSubcommand(sub) => {
                // RPC-059: route to dispatch_rpc059.rs.
                let _ = self.action_tx.send(Action::LoopSubcommandParsed(sub));
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
            self.navigator.agent.push_line(
                &mut self.agent_view_store,
                format!("You: {text}"),
            );
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
        // logged via tracing. Helper lives in dispatch_rpc052.rs.
        self.spawn_clear_pending_input(session.clone());
        // RPC-025: fire-and-forget persistence_add_history + reset the
        // per-session HistoryNavState so the next Shift+↑ pulls a fresh
        // snapshot from disk.
        self.handle_input_submitted_persistence(session, text);
    }
}

/// RPC-047: format a `CompactionResult` into the user-facing scrollback
/// notice line. Single-sourced so the `/compact` success branch AND the
/// `StreamChunk::CompactionComplete` handler (in `dispatch_rpc045.rs`)
/// produce byte-identical output.
///
/// Example output:
/// ```text
/// [compaction] 60.0% reduction (10000 → 4000 tokens, 12 turns summarised)
/// ```
pub(crate) fn format_compaction_notice(result: &CompactionResult) -> String {
    let reduction_pct = (1.0 - result.compression_ratio) * 100.0;
    format!(
        "[compaction] {reduction:.1}% reduction ({orig} \u{2192} {compacted} tokens, {turns} turns summarised)",
        reduction = reduction_pct,
        orig = result.original_tokens,
        compacted = result.compacted_tokens,
        turns = result.turns_summarized,
    )
}
