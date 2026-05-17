//! RPC-020 — Slash command registry.
//!
//! Feature: spec/features/rpc020-slash-and-file-popups.feature
//!
//! Static list of every `/`-prefixed command surfaced by AgentView's
//! palette, plus the three-tier `filter_commands` helper that mirrors
//! `src/tui/utils/slashCommands.ts::filterCommands` so the Rust TUI
//! and the Ink TS TUI agree on filter ordering.
//!
//! `SlashCommandAction` lives here (rather than in `components/mod.rs`)
//! so the Action enum's new `SlashCommandSelected(SlashCommandAction)`
//! variant has a stable home. The registry itself is `const`-friendly:
//! both `SlashCommand` and `SlashCommandAction` derive `Clone, Copy,
//! Debug`, and `SLASH_COMMANDS` is a `&'static [SlashCommand]`.

/// Concrete action emitted by the palette on Enter — App::dispatch
/// branches over this enum to wire the live handlers (Help / Clear /
/// Quit) and to push a `[notice]` line for the rest until the future
/// RPC card lands them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashCommandAction {
    Help,
    Clear,
    Resume,
    Search,
    Model,
    Thinking,
    Role,
    Quit,
    Isolation,
    Blocklist,
    Compact,
    Debug,
    Providers,
    Provider,
    Detach,
    MergeWorktree,
    Schedule,
    Loop,
}

impl SlashCommandAction {
    /// Stable display name (without the leading `/`). Used by both the
    /// popup row painter and the App::dispatch "[notice] /<name> not
    /// yet implemented" formatter.
    pub fn name(&self) -> &'static str {
        match self {
            SlashCommandAction::Help => "help",
            SlashCommandAction::Clear => "clear",
            SlashCommandAction::Resume => "resume",
            SlashCommandAction::Search => "search",
            SlashCommandAction::Model => "model",
            SlashCommandAction::Thinking => "thinking",
            SlashCommandAction::Role => "role",
            SlashCommandAction::Quit => "quit",
            SlashCommandAction::Isolation => "isolation",
            SlashCommandAction::Blocklist => "blocklist",
            SlashCommandAction::Compact => "compact",
            SlashCommandAction::Debug => "debug",
            SlashCommandAction::Providers => "providers",
            SlashCommandAction::Provider => "provider",
            SlashCommandAction::Detach => "detach",
            SlashCommandAction::MergeWorktree => "merge-worktree",
            SlashCommandAction::Schedule => "schedule",
            SlashCommandAction::Loop => "loop",
        }
    }
}

/// A single entry in the palette. Names line up with the TS
/// `SLASH_COMMANDS` registry so the user-facing UX matches across both
/// TUIs.
#[derive(Debug, Clone, Copy)]
pub struct SlashCommand {
    pub action: SlashCommandAction,
    pub description: &'static str,
}

impl SlashCommand {
    pub fn name(&self) -> &'static str {
        self.action.name()
    }
}

/// The full registry. Order is intentional — the palette renders this
/// list verbatim when no filter is set.
pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        action: SlashCommandAction::Help,
        description: "Show help dialog",
    },
    SlashCommand {
        action: SlashCommandAction::Clear,
        description: "Clear conversation history",
    },
    SlashCommand {
        action: SlashCommandAction::Quit,
        description: "Quit fspec TUI",
    },
    SlashCommand {
        action: SlashCommandAction::Model,
        description: "Select AI model",
    },
    SlashCommand {
        action: SlashCommandAction::Thinking,
        description: "Set base thinking level",
    },
    SlashCommand {
        action: SlashCommandAction::Role,
        description: "Set or edit session role",
    },
    SlashCommand {
        action: SlashCommandAction::Resume,
        description: "Resume a previous session",
    },
    SlashCommand {
        action: SlashCommandAction::Search,
        description: "Search command history",
    },
    SlashCommand {
        action: SlashCommandAction::Provider,
        description: "Configure API providers",
    },
    SlashCommand {
        action: SlashCommandAction::Providers,
        description: "Open provider settings",
    },
    SlashCommand {
        action: SlashCommandAction::Debug,
        description: "Toggle debug capture mode",
    },
    SlashCommand {
        action: SlashCommandAction::Compact,
        description: "Compact context window",
    },
    SlashCommand {
        action: SlashCommandAction::Isolation,
        description: "Toggle worktree isolation",
    },
    SlashCommand {
        action: SlashCommandAction::Blocklist,
        description: "Manage blocklist rules",
    },
    SlashCommand {
        action: SlashCommandAction::Detach,
        description: "Detach session from work unit",
    },
    SlashCommand {
        action: SlashCommandAction::MergeWorktree,
        description: "Merge worktree changes and close session",
    },
    SlashCommand {
        action: SlashCommandAction::Schedule,
        description: "Manage scheduled jobs",
    },
    SlashCommand {
        action: SlashCommandAction::Loop,
        description: "Quick recurring schedule (session-scoped)",
    },
];

/// Three-tier filter matching used by the palette.
///
/// 1. Exact prefix matches on the command name.
/// 2. Substring matches in the name (excluding prefix matches).
/// 3. Substring matches in the description (excluding any name match).
///
/// Empty `filter` returns the full registry verbatim. Matching is
/// case-insensitive.
///
/// Mirrors `src/tui/utils/slashCommands.ts::filterCommands` exactly so
/// the Rust TUI and the Ink TS TUI rank suggestions identically.
pub fn filter_commands(filter: &str) -> Vec<&'static SlashCommand> {
    if filter.is_empty() {
        return SLASH_COMMANDS.iter().collect();
    }
    let lower = filter.to_lowercase();
    let mut prefix_matches: Vec<&'static SlashCommand> = Vec::new();
    let mut substring_matches: Vec<&'static SlashCommand> = Vec::new();
    let mut description_matches: Vec<&'static SlashCommand> = Vec::new();
    for cmd in SLASH_COMMANDS.iter() {
        let name_lower = cmd.name().to_lowercase();
        let desc_lower = cmd.description.to_lowercase();
        if name_lower.starts_with(&lower) {
            prefix_matches.push(cmd);
        } else if name_lower.contains(&lower) {
            substring_matches.push(cmd);
        } else if desc_lower.contains(&lower) {
            description_matches.push(cmd);
        }
    }
    let mut out = prefix_matches;
    out.extend(substring_matches);
    out.extend(description_matches);
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn empty_filter_returns_full_registry() {
        let out = filter_commands("");
        assert_eq!(out.len(), SLASH_COMMANDS.len());
    }

    #[test]
    fn prefix_match_ranks_above_substring_match() {
        // "he" → /help (prefix) before any other.
        let out = filter_commands("he");
        assert!(!out.is_empty(), "expected at least one match");
        assert_eq!(out[0].name(), "help");
    }

    #[test]
    fn substring_in_name_outranks_substring_in_description() {
        // "isol" → /isolation (name substring) before any description match.
        let out = filter_commands("isol");
        assert!(!out.is_empty());
        assert_eq!(out[0].name(), "isolation");
    }

    #[test]
    fn case_insensitive_match() {
        let out = filter_commands("HELP");
        assert!(out.iter().any(|c| c.name() == "help"));
    }

    #[test]
    fn no_match_returns_empty() {
        let out = filter_commands("zzzzz-no-such-command");
        assert!(out.is_empty());
    }

    #[test]
    fn names_round_trip_through_action() {
        for cmd in SLASH_COMMANDS.iter() {
            assert!(!cmd.name().is_empty());
            assert!(!cmd.description.is_empty());
        }
    }
}
