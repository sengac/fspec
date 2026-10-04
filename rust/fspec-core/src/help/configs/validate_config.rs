//! `validate-config` help configuration — Rust-only extension command
//! (CONFIG-009, DISC-003 pattern). Validates the user-scope
//! `~/.fspec/fspec-config.json` for project-scope-only keys (`tools`,
//! `agent`) that leaked there (the BUG-193 contamination class).

use super::super::{CommandExample, CommandHelpConfig, CommonError};

const EXAMPLE_1_OUTPUT: &str = "✓ User-scope config is clean (no project-scope keys found)";

const EXAMPLE_2_OUTPUT: &str = "\
⚠ User-scope config has 2 project-scope key(s):

  - `tools` is a project-scope key: Remove `tools` from your user-scope config; configure it per-project with `fspec configure-tools` (writes spec/fspec-config.json).
  - `agent` is a project-scope key: Remove `agent` from your user-scope config; it is set per-project by `fspec init`.";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec validate-config",
        description: Some("Validate the user-scope config for leaked project-scope keys"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec validate-config  # with tools+agent leaked into ~/.fspec/fspec-config.json",
        description: Some("Warnings with per-key remediation; still exit 0 (non-fatal)"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const RELATED: &[&str] = &[
    "configure-tools",
    "init",
    "check",
    "validate-tags",
    "validate-work-units",
];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "⚠ ~/.fspec/fspec-config.json contains malformed JSON (...)",
    fix: "Fix the JSON (or remove the file); validate-config reports it as a warning and never crashes",
}];

const NOTES: &[&str] = &[
    "Read-only: never writes to ~/.fspec/fspec-config.json",
    "Flags ONLY project-scope keys (tools, agent) in the USER-scope file; `research` is a legitimate user-scope key and is never flagged",
    "Warnings are non-fatal — the command exits 0 and reports them (hygiene check, not a correctness gate)",
    "User dir resolution: FSPEC_USER_DIR env override, then $HOME/.fspec (USERPROFILE on Windows)",
    "Rust-only extension command (not in the 162 canonical TS list)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "validate-config",
    description: "Validate the user-scope config for leaked project-scope keys (tools, agent)",
    usage: Some("fspec validate-config"),
    arguments: &[],
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use after a config-scope leak (or as hygiene) to check that ~/.fspec/fspec-config.json contains only user-scope keys.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
