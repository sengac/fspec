//! `add-hook` help configuration — Rust port of
//! `src/commands/add-hook-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-hook --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/add-hook.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "event",
        description:
            "Event name when hook should run (e.g., pre-update-work-unit-status, post-implementing)",
        required: true,
    },
    CommandArgument {
        name: "name",
        description: "Unique name for this hook (e.g., validate-feature, run-tests)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "--command <path>",
        description:
            "Path to hook script, relative to project root (e.g., spec/hooks/validate.sh)",
        default_value: None,
    },
    CommandOption {
        flag: "--blocking",
        description:
            "If set, hook failure prevents command execution (pre-hooks) or sets exit code to 1 (post-hooks)",
        default_value: None,
    },
    CommandOption {
        flag: "--timeout <seconds>",
        description: "Timeout in seconds (default: 60). Hook is killed if it exceeds this time.",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-hook pre-implementing validate-code --command spec/hooks/lint.sh --blocking",
        description: Some("Add blocking pre-hook to validate code before implementing"),
        output: Some("✓ Hook added: pre-implementing/validate-code"),
    },
    CommandExample {
        command: "fspec add-hook post-implementing run-tests --command spec/hooks/test.sh --timeout 300",
        description: Some("Add post-hook with custom timeout"),
        output: Some("✓ Hook added: post-implementing/run-tests"),
    },
    CommandExample {
        command: "fspec add-hook pre-update-work-unit-status check-ready --command spec/hooks/check.sh --blocking",
        description: Some("Add quality gate before status changes"),
        output: Some("✓ Hook added: pre-update-work-unit-status/check-ready"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Hook script does not exist",
        fix: "Create the hook script first, then add the hook. Make sure the script is executable (chmod +x).",
    },
    CommonError {
        error: "Hook name already exists for this event",
        fix: "Choose a unique name or remove the existing hook first with fspec remove-hook.",
    },
];

const RELATED: &[&str] = &["remove-hook", "list-hooks", "validate-hooks"];

const NOTES: &[&str] = &[
    "Creates spec/fspec-hooks.json if it does not exist",
    "Hook scripts must be executable (chmod +x script-path)",
    "Paths must be relative to project root, not to spec/ directory",
    "Event names follow pre-<command> and post-<command> pattern",
    "Blocking hooks emit <system-reminder> tags on failure (for AI agents)",
    "Use --blocking for quality gates, omit for notifications",
];

const PREREQUISITES: &[&str] = &[
    "Hook script exists at the specified path",
    "Hook script is executable (chmod +x)",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Quality Gate (Blocking Pre-Hook)",
        example:
            "fspec add-hook pre-implementing validate --command spec/hooks/lint.sh --blocking",
        description:
            "Prevents implementing phase if linting fails. AI agents see failure in <system-reminder>.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Automated Testing (Post-Hook)",
        example:
            "fspec add-hook post-implementing test --command spec/hooks/test.sh --timeout 300",
        description:
            "Runs tests after implementation completes. Failure sets exit code to 1.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Notification (Non-Blocking Post-Hook)",
        example: "fspec add-hook post-validating notify --command spec/hooks/notify-slack.sh",
        description:
            "Sends notifications after validation. Failure does not prevent completion.",
    }),
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-hook",
    description: "Add a lifecycle hook to the configuration",
    usage: Some("fspec add-hook <event> <name> --command <path> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to add a new lifecycle hook to your project. Hooks execute custom scripts at specific command events (pre-/post- pattern) and enable workflow automation like quality gates, testing, and notifications.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "Create hook script → Make executable (chmod +x) → fspec add-hook → fspec validate-hooks → Test execution",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
