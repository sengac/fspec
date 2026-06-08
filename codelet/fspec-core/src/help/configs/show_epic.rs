//! `show-epic` help configuration — Rust port of
//! `src/commands/show-epic-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js show-epic --help`.
//! The captured fixture lives at
//! `codelet/fspec/tests/fixtures/help/show-epic.txt`; the scenario
//! `scenario_show_epic_help_matches_ts_formatcommandhelp_reference`
//! locks every byte of this output against the fixture.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const EXAMPLE_OUTPUT: &str = "user-management\nTitle: User Management Features\nDescription: Handles user accounts\n\nWork Units:\n  AUTH-001 - User login feature\n  AUTH-002 - Password reset";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec show-epic user-management",
    description: Some("Show epic details"),
    output: Some(EXAMPLE_OUTPUT),
}];

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "epicId",
    description: "Epic ID (e.g., user-management)",
    required: true,
}];

// Parity with TS `show-epic-help.ts`: although `src/commands/show-epic.ts:141`
// registers `--format` on the Commander.js surface, the TS help doc omits
// the option entirely (no OPTIONS row is rendered). The Rust port mirrors
// the TS help-doc exactly so `fspec show-epic --help` is byte-for-byte
// identical to `node dist/index.js show-epic --help`. The `--format` flag
// is still wired up at runtime via the clap Mode variant.
const OPTIONS: &[CommandOption] = &[];

const RELATED: &[&str] = &["list-epics", "create-epic", "list-work-units"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-epic",
    description: "Display details of an epic including associated work units",
    usage: Some("fspec show-epic <epicId>"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: None,
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
