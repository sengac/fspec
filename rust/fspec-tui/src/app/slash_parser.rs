//! Slash command parser invoked by `handle_input_submitted` BEFORE
//! the text is forwarded to `backend.send_input`. Extracted from
//! `dispatch_model_thinking_dialogs.rs` to keep that file under the 300-LoC ceiling
//! after RPC-027 added `Action::SetThinkingLevelDefault` routing.
//!
//! RPC-048 widens this module with the `/thinking <level>` inline-arg
//! branch — bare `/thinking` still opens the ThinkingLevelDialog
//! (RPC-022), but `/thinking off|low|med|medium|high` now resolves to
//! `SetThinkingLevel(level)` and `/thinking <other>` to
//! `InvalidThinkingLevel(other)` so the dispatcher can dispatch the
//! backend write OR emit `[error] unknown thinking level: {other}`
//! without round-tripping through the picker dialog.

use codelet_rpc_types::ThinkingLevel;

use super::continue_parser::{parse_continue_command, ContinueSubcommand};
use super::goal_parser::{parse_goal_command, GoalSubcommand};
use super::loop_parser::{parse_loop_command, LoopSubcommand};
use super::schedule_parser::{parse_schedule_command, ScheduleSubcommand};

/// Outcome of parsing a single submitted input line. The
/// `handle_input_submitted` arm in `dispatch_slash_commands.rs` branches over
/// this enum BEFORE forwarding plain text to `backend.send_input`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandParse {
    /// `/model` — open the full-screen ModelSelector mode-view.
    OpenModelDialog,
    /// `/thinking` (bare) — open the ThinkingLevelDialog.
    OpenThinkingDialog,
    /// `/thinking <off|low|med|medium|high>` — set the per-session
    /// thinking level inline without opening the picker dialog.
    /// RPC-048 — mirrors the TS `AgentView.tsx` slash branch.
    SetThinkingLevel(ThinkingLevel),
    /// `/thinking <other>` — the dispatcher emits
    /// `[error] unknown thinking level: {other}` into the focused
    /// session's scrollback. The captured arg is lowercased + trimmed
    /// so the error notice is stable regardless of how the user typed.
    /// RPC-048.
    InvalidThinkingLevel(String),
    /// `/role` or `/role ` (bare or trailing-space empty arg) — open
    /// the RoleDialog. RPC-063: previously `ClearRole`; users must now
    /// type `/role clear` (or press Ctrl+D inside the dialog) to clear.
    OpenRoleDialog,
    /// `/role clear` — clear the session role.
    ClearRole,
    /// `/role <text>` — set the session role to `text`.
    SetRole(String),
    /// `/schedule …` — RPC-058. Carries the parsed [`ScheduleSubcommand`]
    /// (Add / List / Pause / Resume / Remove / Help) so the dispatcher
    /// can fan out to the matching handle_schedule_* helper without
    /// re-parsing.
    ScheduleSubcommand(ScheduleSubcommand),
    /// `/loop …` — RPC-059. Carries the parsed [`LoopSubcommand`]
    /// (Add / Cancel / List / Help) so the dispatcher can fan out to
    /// the matching handle_loop_* helper without re-parsing.
    LoopSubcommand(LoopSubcommand),
    /// `/continue …` — CONT-002. Carries the parsed
    /// [`ContinueSubcommand`] (Toggle / On / Off / SetBudget /
    /// RejectZero / Invalid) so the dispatcher can apply the toggle and
    /// round-trip the new state to the backend without re-parsing.
    ContinueSubcommand(ContinueSubcommand),
    /// `/goal …` — CONT-003. Carries the parsed [`GoalSubcommand`]
    /// (Set / Show / Verify / Clear) so the dispatcher can apply the
    /// goal state and round-trip it to the backend without re-parsing.
    GoalSubcommand(GoalSubcommand),
    /// Anything else — forward to `backend.send_input` as before.
    NotASlashCommand,
}

/// Inspect the submitted input text and return the slash command it
/// represents, if any. Public so unit tests can exercise the parser
/// without spinning up an App.
///
/// Trimming rules:
///   - "/model" → `OpenModelDialog`
///   - "/thinking" → `OpenThinkingDialog`
///   - "/thinking off|low|med|medium|high" (case-insensitive) →
///     `SetThinkingLevel(ThinkingLevel)`
///   - "/thinking <anything-else>" → `InvalidThinkingLevel(lowercased)`
///   - "/role" (bare) → `OpenRoleDialog`
///   - "/role clear" (any case after trimming) → `ClearRole`
///   - "/role <text>" → `SetRole(text.trim())`
///   - "/role " (trailing space, empty arg) → `OpenRoleDialog`
///   - everything else → `NotASlashCommand`
pub fn parse_slash_command(text: &str) -> SlashCommandParse {
    let trimmed = text.trim();
    if trimmed == "/model" {
        return SlashCommandParse::OpenModelDialog;
    }
    if trimmed == "/thinking" {
        return SlashCommandParse::OpenThinkingDialog;
    }
    if let Some(rest) = trimmed.strip_prefix("/thinking ") {
        // RPC-048: `/thinking <arg>` inline-arg parsing. The captured
        // arg is trimmed + lowercased so `/thinking HIGH` and
        // `/thinking  high  ` both resolve identically. A
        // trailing-space-only `/thinking ` mirrors the bare command
        // (the picker dialog is the right UX, not an error notice).
        let arg = rest.trim().to_ascii_lowercase();
        if arg.is_empty() {
            return SlashCommandParse::OpenThinkingDialog;
        }
        let level = match arg.as_str() {
            "off" => ThinkingLevel::Off,
            "low" => ThinkingLevel::Low,
            "med" | "medium" => ThinkingLevel::Medium,
            "high" => ThinkingLevel::High,
            _ => return SlashCommandParse::InvalidThinkingLevel(arg),
        };
        return SlashCommandParse::SetThinkingLevel(level);
    }
    if trimmed == "/role" {
        return SlashCommandParse::OpenRoleDialog;
    }
    if let Some(rest) = trimmed.strip_prefix("/role ") {
        let arg = rest.trim();
        if arg.is_empty() {
            return SlashCommandParse::OpenRoleDialog;
        }
        if arg.eq_ignore_ascii_case("clear") {
            return SlashCommandParse::ClearRole;
        }
        return SlashCommandParse::SetRole(arg.to_string());
    }
    // RPC-058: route the entire `/schedule …` family through the
    // dedicated parser. Bare `/schedule` resolves to
    // `ScheduleSubcommand::Help` so the dispatcher always sees a
    // structured variant.
    if trimmed == "/schedule" || trimmed.starts_with("/schedule ") {
        return SlashCommandParse::ScheduleSubcommand(parse_schedule_command(trimmed));
    }
    // RPC-059: route the entire `/loop …` family through the dedicated
    // parser. Bare `/loop` resolves to `LoopSubcommand::Help`.
    if trimmed == "/loop" || trimmed.starts_with("/loop ") {
        return SlashCommandParse::LoopSubcommand(parse_loop_command(trimmed));
    }
    // CONT-002: route the entire `/continue …` family through the
    // dedicated parser. Bare `/continue` resolves to
    // `ContinueSubcommand::Toggle`.
    if trimmed == "/continue" || trimmed.starts_with("/continue ") {
        return SlashCommandParse::ContinueSubcommand(parse_continue_command(trimmed));
    }
    // CONT-003: route the entire `/goal …` family through the dedicated
    // parser. Bare `/goal` resolves to `GoalSubcommand::Show`.
    if trimmed == "/goal" || trimmed.starts_with("/goal ") {
        return SlashCommandParse::GoalSubcommand(parse_goal_command(trimmed));
    }
    SlashCommandParse::NotASlashCommand
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn parse_slash_command_recognises_model_thinking_and_role_variants() {
        assert_eq!(
            parse_slash_command("/model"),
            SlashCommandParse::OpenModelDialog
        );
        assert_eq!(
            parse_slash_command("/thinking"),
            SlashCommandParse::OpenThinkingDialog
        );
        assert_eq!(
            parse_slash_command("/role"),
            SlashCommandParse::OpenRoleDialog
        );
        assert_eq!(
            parse_slash_command("/role clear"),
            SlashCommandParse::ClearRole
        );
        assert_eq!(
            parse_slash_command("/role CLEAR"),
            SlashCommandParse::ClearRole
        );
        assert_eq!(
            parse_slash_command("/role You are a security reviewer"),
            SlashCommandParse::SetRole("You are a security reviewer".to_string())
        );
        assert_eq!(
            parse_slash_command("/role  leading space ok"),
            SlashCommandParse::SetRole("leading space ok".to_string())
        );
        assert_eq!(
            parse_slash_command("hello world"),
            SlashCommandParse::NotASlashCommand
        );
        assert_eq!(
            parse_slash_command("/unknown anything"),
            SlashCommandParse::NotASlashCommand
        );
    }
}
