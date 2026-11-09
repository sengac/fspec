//! RPC-397 — View-specific help content for the board and agent variants.
//!
//! Feature: spec/features/view-specific-accurate-help-content-for-board-and-agent.feature
//!
//! Two content builders feed [`super::help_dialog::HelpDialog`]:
//!   * [`board_help_lines`] — accurate board keybindings ONLY (no slash
//!     commands). Its shortcut rows come from the menu registry
//!     (TUI-129), the rest from `views/board.rs` + `app/events.rs`.
//!   * [`agent_help_lines`] — accurate agent keybindings PLUS the full
//!     slash-command list, formatted directly from the single source of
//!     truth `views::agent::slash_commands::SLASH_COMMANDS` so the
//!     `/name    description` rows can never drift.
//!
//! Neither variant shows the old misleading `q       Quit` line; the
//! quit binding reads `Ctrl+D   Quit`.

use crate::components::menu_bar::items::CATEGORIES;
use crate::views::agent::slash_commands::SLASH_COMMANDS;

/// Board keybindings, each `KEY(s)    Explanation`. NO slash commands.
///
/// TUI-129: the shortcut rows are the menu registry's one-character rows
/// (`CATEGORIES`, the table the board key handler reads), with the same
/// key and label as the menu, so the help can never drift from the keys.
pub(crate) fn board_help_lines() -> Vec<String> {
    let mut lines: Vec<String> = [
        "Board keybindings:",
        "",
        "↑/k, ↓/j      Navigate work units",
        "←/h, →/l      Cycle columns → menu items → chips",
        "Enter         Choose agent or full details",
        "Enter (item)  Opens its dropdown menu",
        "Enter (chip)  Opens that session",
        "Shift+Right   Open agent view",
        "[ / ]         Reorder (up / down)",
        "PageUp/PageDn Scroll column",
        "Home / End    First / Last",
    ]
    .iter()
    .map(|l| (*l).to_string())
    .collect();
    for entry in CATEGORIES.iter().flat_map(|category| category.entries) {
        if entry.key.chars().count() == 1 {
            lines.push(format!("{:<14}{}", entry.key, entry.label));
        }
    }
    lines.extend(
        [
            "u             Menu bar help",
            "ESC           Exit (confirm)",
            "Ctrl+D        Quit",
        ]
        .iter()
        .map(|l| (*l).to_string()),
    );
    lines
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
        "Ctrl+J        Newline (universal)",
        "Shift+Enter   Newline (best-effort)",
        "Alt+Enter     Newline (queues the draft while busy)",
        "Ctrl+C        Interrupt turn",
        "PageUp/PageDn Scroll history",
        "Home / End    Top / Bottom (empty input)",
        "Shift+↑/↓     Input history (mux: pane above/below)",
        "Shift+←/→     Cycle sessions (mux: previous/next pane)",
        "← empty input Enter the menu bar",
        "Tab           Select turn",
        "Ctrl+R        Search history",
        "/             Slash commands",
        "@             File search",
        "ESC           Stop the run, clear the draft, or close the agent",
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
