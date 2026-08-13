//! `remove-init-files` help configuration — Rust port of
//! `src/commands/remove-init-files-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js remove-init-files --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/remove-init-files.txt`.
//!
//! NOTE: the TS `typicalWorkflow` is a string[]; the help-formatter renders it
//! via template interpolation which joins array elements with `,` (no space).
//! The Rust `typical_workflow: Option<&str>` reproduces that comma-joined
//! string verbatim so the fixture matches byte-for-byte.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[];

const OPTS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-init-files",
        description: Some("Remove fspec init files for detected agent"),
        output: Some(
            "✓ Successfully removed fspec init files\n  - Removed spec/CLAUDE.md\n  - Removed .claude/commands/fspec.md\n  - Removed spec/fspec-config.json",
        ),
    },
    CommandExample {
        command: "fspec remove-init-files",
        description: Some("When no agent detected"),
        output: Some("✗ No fspec agent installation detected. Nothing to remove."),
    },
];

const COMMON_ERRORS: &[CommonError] = &[];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[];

const RELATED: &[&str] = &["init"];

const NOTES: &[&str] = &[
    "Auto-detects installed agent using spec/fspec-config.json or file detection",
    "Removes agent-specific files: spec/<AGENT>.md and slash command files",
    "Also removes spec/fspec-config.json",
    "Silently skips files that are already deleted (idempotent behavior)",
    "Does NOT remove spec/features/, spec/work-units.json, or other project files",
    "Use fspec init to reinstall after removal",
];

const PREREQS: &[&str] = &[
    "At least one agent must be installed (detected via spec/fspec-config.json or file detection)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-init-files",
    description: "Remove fspec initialization files for installed agents",
    usage: Some("fspec remove-init-files"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when you want to uninstall fspec or switch to a different agent. Auto-detects installed agents and removes their configuration files.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQS,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "detect-agent → remove-files → confirmation,Common pattern: fspec remove-init-files (then fspec init --agent=newagent to switch)",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
