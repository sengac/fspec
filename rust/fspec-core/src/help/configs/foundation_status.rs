//! `foundation-status` help configuration — Rust-only extension command
//! (DISC-003). Read-only progress report for the foundation discovery
//! workflow.

use super::super::{CommandExample, CommandHelpConfig, CommandOption, CommonError};

const EXAMPLE_1_OUTPUT: &str = "Foundation: DRAFT (spec/foundation.json.draft)\n\n\
Progress: 2/8 fields complete\n\n\
✓ project.name (complete) — fspec\n\
✗ project.vision (placeholder) — [QUESTION: What is the one-sentence vision?]\n\
...";

const EXAMPLE_2_OUTPUT: &str = "{\n\
  \"phase\": \"draft\",\n\
  \"progress\": {\"complete\": 2, \"total\": 8},\n\
  \"fields\": [...],\n\
  \"remaining\": [...],\n\
  \"nextAction\": \"Continue discovery. Run: fspec foundation-status\"\n\
}";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec foundation-status",
        description: Some("Show foundation phase, progress table, and remaining fields"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec foundation-status --json",
        description: Some(
            "Machine-readable envelope (phase, progress, fields, remaining, nextAction)",
        ),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--json",
    description: "Emit the machine-readable JSON envelope instead of the text report",
    default_value: None,
}];

const RELATED: &[&str] = &[
    "discover-foundation",
    "show-foundation",
    "update-foundation",
    "add-capability",
    "add-persona",
    "validate-foundation-schema",
];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "foundation.json not found",
    fix: "No foundation yet — run: fspec discover-foundation",
}];

const NOTES: &[&str] = &[
    "Read-only: never writes to spec/foundation.json or spec/foundation.json.draft",
    "Prefers the draft when present (draft-wins), matching the mutation commands",
    "Rust-only extension command (not in the 162 canonical TS list)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "foundation-status",
    description: "Read-only progress report for the foundation discovery workflow",
    usage: Some("fspec foundation-status [--json]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use at any point during foundation discovery to see which of the 8 draft fields remain, their fix commands with concrete examples, and the finalize next-action.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
