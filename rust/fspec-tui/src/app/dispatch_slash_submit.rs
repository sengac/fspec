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

use codelet_rpc_types::SessionId;
use ratatui::style::Color;

use crate::components::Action;

use super::slash_parser::{parse_slash_command, SlashCommandParse};
use super::state::App;

impl App {
    /// Spawn `backend.send_input` for `session`, the session whose
    /// composer produced the submission (BUG-552: never the focus at
    /// handling time). Handles the no-session-manager stub case by
    /// surfacing a notice line in the scrollback rather than dispatching
    /// the call.
    ///
    /// RPC-022 intercepts `/model`, `/thinking`, `/role …` via
    /// `parse_slash_command` before forwarding to `send_input`.
    /// RPC-078 deletes the sync `push_line("user> …")` so UserInput
    /// only lands via the `StreamChunk::UserInput` broadcast path.
    pub(crate) fn handle_input_submitted(&mut self, session: SessionId, text: String) {
        // BUG-549 rule 2: a submission ends the draft — whichever path
        // below handles it (model, local slash command, archive) — and
        // the immediate clear supersedes any draft save still pending.
        self.end_composer_draft(&session);

        if self.handle_archive_command(&session, &text) {
            return;
        }

        // RPC-022 slash-command interception.
        let parsed = parse_slash_command(&text);
        if self.refuses_unfocused_command(&session, &parsed, &text) {
            return;
        }
        match parsed {
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
                // RPC-048 / BUG-455: `/thinking default` clears the
                // per-session reasoning level inline without opening
                // the picker dialog. Routes through the
                // existing RPC-022 helper so backend.set_thinking_level
                // is awaited and a follow-up backend.get_thinking_level
                // refreshes AgentViewStore.thinking_level_for(session)
                // via Action::ThinkingLevelLoaded.
                self.handle_thinking_level_selected(session, level);
                return;
            }
            SlashCommandParse::ThinkingCommand(arg) => {
                // MODEL-011: resolved against the levels the session's model
                // offers; a refusal lands in the session's scrollback. NO
                // dialog is pushed and the text is not sent as input.
                self.handle_thinking_command(session, arg);
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
                // persistence_add_history (RPC-022 rule). The draft was
                // already ended above (BUG-549).
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
            if let Some(context) = self.agent_view_store.session_context_mut_for(&session) {
                context.push_line(format!("You: {text}"));
                context.push_line(
                    "[notice] no LLM session manager attached — input recorded but \
                     not sent to a model.",
                );
            }
            return;
        }
        let backend = self.backend.clone();
        let session_for_send = session.clone();
        let text_for_send = text.clone();
        // RIG-016 + TUI-115: a running turn is steered, a compaction is
        // queued behind, instead of either being sent a new turn.
        if let Some(mode) = self.busy_enter_mode(&session) {
            self.submit_busy_input(session.clone(), text.clone(), mode);
        // Guard sync test dispatchers (no tokio runtime).
        } else if tokio::runtime::Handle::try_current().is_ok() {
            let action_tx = self.action_tx.clone();
            let handle = tokio::spawn(async move {
                // BUG-328: a refused send is shown in the session.
                if let Err(error) = backend
                    .send_input(session_for_send.clone(), text_for_send)
                    .await
                {
                    tracing::warn!(session = %session_for_send.value, %error, "send_input failed");
                    let _ = action_tx.send(Action::EmitSessionNotice(
                        session_for_send,
                        format!("[error] send failed: {error}"),
                    ));
                }
            });
            // RPC-012 test-seam contract: park the
            // JoinHandle so `drain_pending` harnesses await it before
            // asserting on `MockBackend.send_input_calls()`. Unparked,
            // the spawned call raced the assertions (BUG-169 flaky
            // pair: registered_name_with_trailing_argument_goes_to_the_llm
            // / unknown_slash_lines_still_go_to_the_llm).
            self.pending_tasks.push(handle);
        }
        // RPC-025: fire-and-forget persistence_add_history + reset the
        // per-session HistoryNavState so the next Shift+↑ pulls a fresh
        // snapshot from disk.
        self.handle_input_submitted_persistence(session, text);
    }

    /// BUG-552: a slash command that does not name its session acts on
    /// the focused one, so it runs only while `session` (the session it
    /// was typed in) is still focused. Otherwise it is not run and
    /// `session` shows why. Sends, steers and queues (not slash commands)
    /// and the session-naming commands (/role, /thinking <level>; /archive
    /// is handled before) always go to `session`.
    fn refuses_unfocused_command(
        &mut self,
        session: &SessionId,
        parsed: &SlashCommandParse,
        text: &str,
    ) -> bool {
        let names_its_session = matches!(
            parsed,
            SlashCommandParse::NotASlashCommand
                | SlashCommandParse::ClearRole
                | SlashCommandParse::SetRole(_)
                | SlashCommandParse::SetThinkingLevel(_)
                | SlashCommandParse::ThinkingCommand(_)
        );
        if names_its_session || self.agent_view_store.current_session() == Some(session) {
            return false;
        }
        let command = text.split_whitespace().next().unwrap_or(text);
        self.push_command_not_run_notice(session, command);
        true
    }

    /// BUG-552/BUG-560: `command`, typed or picked in `session`, was not run
    /// because another session was focused before it was handled; `session`
    /// says so.
    pub(super) fn push_command_not_run_notice(&mut self, session: &SessionId, command: &str) {
        tracing::warn!(session = %session.value, command, "slash command for an unfocused session not run");
        if let Some(context) = self.agent_view_store.session_context_mut_for(session) {
            context.push_notice_line(
                format!(
                    "[notice] {command} was not run: another session was focused before it was handled"
                ),
                Color::Yellow,
            );
        }
    }
}
