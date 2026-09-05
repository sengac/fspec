//! RPC-022/BUG-169: `App::handle_input_submitted` — the AgentView
//! typed-submit path.
//!
//! Parses the submitted line with `parse_slash_command` (the
//! submit-time interceptor: family branches + the BUG-169
//! registry-driven `BareCommand` catch) and only falls through to
//! `backend.send_input` + the RPC-025/RPC-052 persistence tail for
//! `NotASlashCommand`.
//!
//! Factored out of `dispatch_slash_commands.rs` so both files stay
//! under the 300-LoC ceiling pinned by the source-shape cards
//! (rpc024 / rpc049 / rpc050 / rpc026).

use crate::components::Action;

use super::slash_parser::{parse_slash_command, SlashCommandParse};
use super::state::App;

impl App {
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
            SlashCommandParse::UpdateSubcommand(sub) => {
                // UPD-002: apply directly in this dispatch tick —
                // handler body lives in dispatch_slash_update.rs.
                self.handle_update_subcommand(sub);
                return;
            }
            SlashCommandParse::MuxCommand(line) => {
                // MUX-001: apply the /mux subcommand directly in this
                // dispatch tick (handler body lives in dispatch_mux.rs).
                self.handle_mux_subcommand(&line);
                return;
            }
            SlashCommandParse::BareCommand(action) => {
                // BUG-169: registry-driven bare-command catch. Route to
                // the SAME handler a popup pick uses (single source of
                // truth — AGENTS.md "Two Front Doors, One Source of
                // Truth") and RETURN immediately: no send_input, no
                // persistence_add_history (RPC-022 rule), no
                // pending-input draft clear.
                self.handle_slash_command(action);
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
