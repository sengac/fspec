//! `add-virtual-hook` help configuration — Rust port of
//! `src/commands/add-virtual-hook-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-virtual-hook --help`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-virtual-hook AUTH-001 post-implementing \"<quality-check-commands>\" --blocking",
        description: Some(
            "Add blocking lint check after implementing (prevents validating if lint fails)",
        ),
        output: Some("✓ Virtual hook added to AUTH-001\n  Total virtual hooks: 1"),
    },
    CommandExample {
        command: "fspec add-virtual-hook BUG-042 pre-validating \"<quality-check-commands>\" --blocking",
        description: Some("Add type check before validation phase"),
        output: Some("✓ Virtual hook added to BUG-042\n  Total virtual hooks: 1"),
    },
    CommandExample {
        command:
            "fspec add-virtual-hook FEAT-123 post-implementing \"eslint src/\" --git-context --blocking",
        description: Some("Lint only changed files using git context (more efficient)"),
        output: Some("✓ Virtual hook added to FEAT-123\n  Total virtual hooks: 1"),
    },
    CommandExample {
        command: "fspec add-virtual-hook AUTH-001 post-validating \"npm audit\"",
        description: Some(
            "Run security audit (non-blocking - shows output but allows completion)",
        ),
        output: Some("✓ Virtual hook added to AUTH-001\n  Total virtual hooks: 2"),
    },
];

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID to attach hook to (e.g., AUTH-001, BUG-042)",
        required: true,
    },
    CommandArgument {
        name: "event",
        description: "Hook event when command should run (e.g., post-implementing, pre-validating)",
        required: true,
    },
    CommandArgument {
        name: "command",
        description: "Command to execute (e.g., \"<quality-check-commands>\", \"eslint src/\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--blocking",
        description:
            "If set, hook failure prevents workflow transition. AI sees failure in <system-reminder> tags.",
        default_value: None,
    },
    CommandOption {
        flag: "--git-context",
        description:
            "Provide git context (staged/unstaged files). Generates script in spec/hooks/.virtual/ that processes changed files only.",
        default_value: None,
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        fix: "Create the work unit first: fspec create-story AUTH \"Title\" (or create-bug/create-task)",
    },
    CommonError {
        error: "Hook command not found: eslint",
        fix: "Install the tool first (npm install -D eslint) or use a different command.",
    },
];

const RELATED: &[&str] = &[
    "list-virtual-hooks",
    "remove-virtual-hook",
    "clear-virtual-hooks",
    "copy-virtual-hooks",
    "update-work-unit-status",
];

const NOTES: &[&str] = &[
    "Virtual hooks are stored in spec/work-units.json under workUnit.virtualHooks array",
    "Virtual hooks run BEFORE global hooks (from spec/fspec-hooks.json)",
    "Multiple hooks can be added to the same event - they execute sequentially",
    "Hook name is auto-generated from command (e.g., \"eslint src/\" → \"eslint\")",
    "Git context hooks generate script files in spec/hooks/.virtual/",
    "Scripts are automatically cleaned up when hooks are removed",
    "Blocking hooks emit <system-reminder> tags on failure for AI agents",
    "Non-blocking hooks show output but do not prevent workflow transitions",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist (fspec create-story, create-bug, or create-task)",
    "Command/tool must be installed and available in PATH",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Linting Before Implementation",
        example:
            "fspec add-virtual-hook AUTH-001 pre-implementing \"<quality-check-commands>\" --blocking",
        description:
            "Ensures code is clean before starting implementation. Prevents messy code from being committed.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Type Checking Before Validation",
        example:
            "fspec add-virtual-hook AUTH-001 pre-validating \"<quality-check-commands>\" --blocking",
        description:
            "Catches type errors before moving to validation phase. Strict quality gate.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Security Scan on Changed Files",
        example: "fspec add-virtual-hook FEAT-123 post-implementing \"npm audit\" --git-context",
        description:
            "Runs security audit only on changed files for efficiency. Uses git context.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Multiple Quality Checks",
        example:
            "fspec add-virtual-hook AUTH-001 post-implementing \"eslint src/\" --blocking\nfspec add-virtual-hook AUTH-001 post-implementing \"prettier --check .\" --blocking",
        description: "Adds multiple hooks to same event. Both must pass to proceed.",
    }),
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-virtual-hook",
    description: "Add a work unit-scoped virtual hook for dynamic validation",
    usage: Some("fspec add-virtual-hook <workUnitId> <event> <command> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to attach ephemeral validation hooks to specific work units. Virtual hooks are scoped to a single work unit and run BEFORE global hooks. Perfect for one-off quality checks (linting, type checking, security scans) that apply only to the current story/bug/task.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "Move to specifying → AI asks about quality checks → Add virtual hooks → Move to testing → Hooks run automatically at specified events → Move to done → AI asks to keep or remove hooks",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
