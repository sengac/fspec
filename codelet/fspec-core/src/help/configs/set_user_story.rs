//! `set-user-story` help configuration — Rust port of
//! `src/commands/set-user-story-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js set-user-story --help`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "work-unit-id",
    description: "Work unit ID (e.g., AUTH-001, DASH-002). Must match existing work unit.",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--role <role>",
        description: "User role or persona (e.g., \"developer\", \"system admin\")",
        default_value: None,
    },
    CommandOption {
        flag: "--action <action>",
        description: "What the user wants to do (e.g., \"validate feature files\")",
        default_value: None,
    },
    CommandOption {
        flag: "--benefit <benefit>",
        description: "Why the user wants it (e.g., \"catch syntax errors early\")",
        default_value: None,
    },
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist (create with fspec create-story, create-bug, or create-task)",
    "Work unit should be in \"specifying\" status",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet(
        "Example Mapping: Set user story first, then add rules and examples before generating scenarios",
    ),
    CommonPatternEntry::Bullet(
        "Feature Discovery: Capture user story during stakeholder conversations to ensure alignment",
    ),
    CommonPatternEntry::Bullet(
        "ACDD Workflow: User story defines WHAT and WHY, scenarios define HOW",
    ),
];

const EXAMPLE_1_OUTPUT: &str = "✓ User story set for AUTH-001\n  As a developer\n  I want to validate feature files automatically\n  So that I catch syntax errors before committing";

const EXAMPLE_2_OUTPUT: &str = "✓ User story set for DASH-002\n  As a system admin\n  I want to view all work unit statuses in one place\n  So that I can track team progress at a glance";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command:
            "fspec set-user-story AUTH-001 --role \"developer\" --action \"validate feature files automatically\" --benefit \"I catch syntax errors before committing\"",
        description: Some("Set user story for authentication work unit"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command:
            "fspec set-user-story DASH-002 --role \"system admin\" --action \"view all work unit statuses in one place\" --benefit \"I can track team progress at a glance\"",
        description: Some("Set user story for dashboard work unit"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Work unit 'AUTH-001' does not exist",
        fix: "Create work unit first: fspec create-story AUTH \"User Authentication\"",
    },
    CommonError {
        error: "Error: Missing required option --role",
        fix: "Provide all three flags: --role, --action, and --benefit",
    },
];

const RELATED: &[&str] = &[
    "create-story",
    "create-bug",
    "create-task",
    "generate-scenarios",
    "add-rule",
    "add-example",
    "show-work-unit",
];

const NOTES: &[&str] = &[
    "User story format: \"As a [role] I want to [action] So that [benefit]\"",
    "ALWAYS set user story BEFORE generating scenarios (prevents prefill placeholders)",
    "Role should be a persona or user type, not a system component",
    "Action should describe capability, not implementation",
    "Benefit should explain business value or user outcome",
    "User story is stored in spec/work-units.json and used by generate-scenarios",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "set-user-story",
    description:
        "Set the user story for a work unit, capturing the role, action, and benefit for Example Mapping",
    usage: Some("fspec set-user-story <work-unit-id> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command during Example Mapping (Discovery phase) to capture the user story BEFORE generating scenarios. This is the FIRST step in defining acceptance criteria. Essential for ensuring generated scenarios align with user needs.",
    ),
    when_not_to_use: Some(
        "Do not use after generating scenarios (use update-work-unit to change fields). Do not use for technical implementation details (those go in architecture notes).",
    ),
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Create work unit → 2. Move to specifying status → 3. fspec set-user-story (this command) → 4. Add rules/examples → 5. fspec generate-scenarios",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
