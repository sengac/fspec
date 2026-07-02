//! RPC-397 — View-specific help content for the board and agent variants.
//!
//! Feature: spec/features/view-specific-accurate-help-content-for-board-and-agent.feature
//!
//! Two content builders feed [`super::help_dialog::HelpDialog`]:
//!   * [`board_help_lines`] — accurate board keybindings ONLY (no slash
//!     commands). Sourced from `views/board.rs` + `app/events.rs`.
//!   * [`agent_help_lines`] — accurate agent keybindings PLUS the full
//!     slash-command list, formatted directly from the single source of
//!     truth `views::agent::slash_commands::SLASH_COMMANDS` so the
//!     `/name    description` rows can never drift.
//!
//! Neither variant shows the old misleading `q       Quit` line; the
//! quit binding reads `Ctrl+D   Quit`.

use crate::views::agent::slash_commands::SLASH_COMMANDS;

/// Board keybindings, each `KEY(s)    Explanation`. NO slash commands.
///
/// Source: `spec/attachments/RPC-397/ast-research-help-content.md`
/// (`views/board.rs` handle_event + `app/events.rs` app shortcuts).
pub(crate) fn board_help_lines() -> Vec<String> {
    [
        "Board keybindings:",
        "",
        "↑/k, ↓/j      Navigate work units",
        "←/h, →/l      Switch column",
        "Enter         Work / open focused unit",
        "Shift+Right   Open agent view",
        ".             New Agent",
        "[ / ]         Reorder (up / down)",
        "PageUp/PageDn Scroll column",
        "Home / End    First / Last",
        "f             Changed Files",
        "c             Checkpoints",
        "d             FOUNDATION.md",
        "a             Attachments",
        "?             Show this help",
        "ESC           Exit (confirm)",
        "Ctrl+D        Quit",
    ]
    .iter()
    .map(|l| (*l).to_string())
    .collect()
}

/// Agent keybindings PLUS the full slash-command list.
///
/// The slash-command rows are derived from `SLASH_COMMANDS` so
/// `/compact` always pairs with "Compact context window" and `/model`
/// with "Select AI model".
pub(crate) fn agent_help_lines() -> Vec<String> {
    let mut lines: Vec<String> = [
        "Agent keybindings:",
        "",
        "Enter         Send message",
        "Shift+Enter   Newline",
        "Alt+Enter     Newline (legacy terminals)",
        "Ctrl+C        Interrupt turn",
        "PageUp/PageDn Scroll history",
        "Home          Top",
        "End           Bottom",
        "Shift+↑/↓     Input history",
        "Shift+←/→     Cycle sessions",
        "Tab           Select turn",
        "Ctrl+R        Search history",
        "/             Slash commands",
        "@             File search",
        "ESC           Back",
        "Ctrl+D        Quit",
        "",
        "Slash commands:",
        "",
    ]
    .iter()
    .map(|l| (*l).to_string())
    .collect();

    for cmd in SLASH_COMMANDS.iter() {
        lines.push(format!("/{:<14}{}", cmd.name(), cmd.description));
    }

    lines
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn board_lines_contain_board_hints_and_no_slash_commands() {
        let text = board_help_lines().join("\n");
        assert!(text.contains("New Agent"));
        assert!(text.contains("Reorder"));
        assert!(text.contains("Ctrl+D"));
        assert!(text.contains("Quit"));
        for slash in &["/help", "/model", "/compact"] {
            assert!(!text.contains(slash));
        }
        assert!(!text.contains("q       Quit"));
    }

    #[test]
    fn agent_lines_contain_agent_hints_and_the_full_slash_list() {
        let text = agent_help_lines().join("\n");
        assert!(text.contains("Send"));
        assert!(text.contains("Interrupt"));
        assert!(text.contains("/compact"));
        assert!(text.contains("Compact context window"));
        assert!(text.contains("/model"));
        assert!(text.contains("Select AI model"));
        assert!(!text.contains("q       Quit"));
        assert!(!text.contains("Quit fspec-tui"));
    }
}
