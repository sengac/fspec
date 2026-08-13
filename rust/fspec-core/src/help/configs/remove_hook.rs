//! `remove-hook` help configuration — Rust port of
//! `src/commands/remove-hook-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js remove-hook --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/remove-hook.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "event",
        description:
            "Event name of the hook to remove (e.g., pre-update-work-unit-status, post-implementing)",
        required: true,
    },
    CommandArgument {
        name: "name",
        description: "Name of the hook to remove (e.g., validate-feature, run-tests)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-hook pre-implementing validate-code",
        description: Some("Remove a pre-hook"),
        output: Some("✓ Hook removed: pre-implementing/validate-code"),
    },
    CommandExample {
        command: "fspec remove-hook post-implementing run-tests",
        description: Some("Remove a post-hook"),
        output: Some("✓ Hook removed: post-implementing/run-tests"),
    },
    CommandExample {
        command: "fspec remove-hook pre-update-work-unit-status check-ready",
        description: Some("Remove a quality gate hook"),
        output: Some("✓ Hook removed: pre-update-work-unit-status/check-ready"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Hook not found",
        fix: "Verify the event and hook name are correct using fspec list-hooks. Event and name must match exactly.",
    },
    CommonError {
        error: "Hook configuration file not found",
        fix: "No hooks are configured. Nothing to remove.",
    },
];

const RELATED: &[&str] = &["add-hook", "list-hooks", "validate-hooks"];

const NOTES: &[&str] = &[
    "Removes hook from spec/fspec-hooks.json",
    "Does NOT delete the hook script file",
    "Empty event array is retained (key is NOT deleted)",
    "Use fspec list-hooks to see available hooks",
    "Case-sensitive: event and name must match exactly",
];

const PREREQUISITES: &[&str] = &[
    "Hook configuration file exists (spec/fspec-hooks.json)",
    "Hook exists with the specified event and name",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-hook",
    description: "Remove a lifecycle hook from the configuration",
    usage: Some("fspec remove-hook <event> <name>"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to remove a hook from your configuration. Useful when you no longer need a specific automation or want to temporarily disable a hook.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "fspec list-hooks → Identify hook to remove → fspec remove-hook <event> <name> → Verify with list-hooks",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
