//! `create-task` help configuration — Rust port of
//! `src/commands/create-task-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js create-task --help`.
//!
//! NOTE on quirks faithfully reproduced from the TS reference:
//! * `relatedCommands` entries already carry a `fspec ` prefix, so the
//!   formatter renders them as `fspec fspec update-work-unit-status - ...`.
//! * `typicalWorkflow` is a TS array joined with `,` (no space) into a
//!   single line.
//! * The TS help-formatter only reads `error.fix` for COMMON ERRORS; the
//!   help.ts data uses `solution` (not `fix`), so the rendered Fix line is
//!   `undefined` in the captured fixture. We mirror that verbatim.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "prefix",
        description:
            "Task prefix (e.g., TASK, INFRA, DEVOPS). Must be registered with create-prefix first.",
        required: true,
    },
    CommandArgument {
        name: "title",
        description: "Brief description of the task",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "-d, --description <description>",
        description: "Detailed description of the task",
        default_value: None,
    },
    CommandOption {
        flag: "-e, --epic <epic>",
        description: "Epic ID to associate with this task",
        default_value: None,
    },
    CommandOption {
        flag: "-p, --parent <parent>",
        description: "Parent task ID for hierarchical relationships",
        default_value: None,
    },
];

const EXAMPLE_1_OUTPUT: &str = "✓ Created task TASK-001\n  Title: Setup CI/CD pipeline\n\n<system-reminder>\nTask TASK-001 created successfully.\n\nTasks are for operational work.\n  - Tasks have optional feature file\n  - Tasks have optional tests\n  - Tasks can skip Example Mapping\n</system-reminder>";
const EXAMPLE_2_OUTPUT: &str =
    "✓ Created task INFRA-001\n  Title: Configure monitoring\n  Epic: infrastructure";
const EXAMPLE_3_OUTPUT: &str =
    "✓ Created task DEVOPS-001\n  Title: Update dependencies\n  Description: Security patches";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec create-task TASK \"Setup CI/CD pipeline\"",
        description: Some("Create simple task with minimal requirements"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec create-task INFRA \"Configure monitoring\" --epic=infrastructure",
        description: Some("Create task with epic"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command:
            "fspec create-task DEVOPS \"Update dependencies\" --description=\"Security patches\"",
        description: Some("Create task with description"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const PREREQUISITES: &[&str] = &[
    "Prefix must be registered: fspec create-prefix PREFIX \"Description\"",
    "Epic must exist if using --epic: fspec create-epic EPIC \"Title\"",
    "Parent task must exist if using --parent",
];

const TYPICAL_WORKFLOW: &str = "Create task: fspec create-task PREFIX \"Title\",Tasks can skip specifying phase (optional feature file),Move directly to implementing: fspec update-work-unit-status TASK-001 implementing,Complete work without tests (for operational tasks),Move to done: fspec update-work-unit-status TASK-001 done";

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Prefix 'PREFIX' is not registered",
        fix: "undefined",
    },
    CommonError {
        error: "Parent task 'PARENT-001' does not exist",
        fix: "undefined",
    },
    CommonError {
        error: "Epic 'epic-name' does not exist",
        fix: "undefined",
    },
];

const RELATED: &[&str] = &[
    "fspec update-work-unit-status - Move task through workflow",
    "fspec list-work-units - List all tasks",
    "fspec show-work-unit - Show task details",
    "fspec update-work-unit - Update task title/description",
];

const NOTES: &[&str] = &[
    "Tasks have optional feature files (not required for operational work)",
    "Tasks have optional tests (not required for infrastructure work)",
    "Tasks can skip Example Mapping (no need for acceptance criteria)",
    "Tasks can move directly to implementing without specifying phase",
    "Examples: Setup CI/CD, configure monitoring, update dependencies, refactor code, write docs",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "create-task",
    description:
        "Create a new task with minimal requirements for operational work (setup, config, infrastructure)",
    usage: Some("fspec create-task <prefix> <title> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when tracking operational work that does not require user-facing features, tests, or acceptance criteria (e.g., setup CI/CD, update dependencies, refactor code).",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
