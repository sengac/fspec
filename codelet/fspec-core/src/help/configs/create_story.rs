//! `create-story` help configuration — Rust port of
//! `src/commands/create-story-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js create-story --help`.
//!
//! NOTE on quirks faithfully reproduced from the TS reference:
//! * `relatedCommands` entries already carry a `fspec ` prefix, so the
//!   formatter renders them as `fspec fspec add-rule - ...`.
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
            "Story prefix (e.g., AUTH, UI, API). Must be registered with create-prefix first.",
        required: true,
    },
    CommandArgument {
        name: "title",
        description: "Brief description of the story",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "-d, --description <description>",
        description: "Detailed description of the story",
        default_value: None,
    },
    CommandOption {
        flag: "-e, --epic <epic>",
        description: "Epic ID to associate with this story",
        default_value: None,
    },
    CommandOption {
        flag: "-p, --parent <parent>",
        description: "Parent story ID for hierarchical relationships",
        default_value: None,
    },
];

const EXAMPLE_1_OUTPUT: &str = "✓ Created story AUTH-001\n  Title: User login feature\n\n<system-reminder>\nStory AUTH-001 created successfully.\n\nNext steps - Example Mapping:\n  fspec add-rule AUTH-001 \"Rule text\"\n  fspec add-example AUTH-001 \"Example text\"\n  fspec add-question AUTH-001 \"@human: Question?\"\n  fspec set-user-story AUTH-001 --role \"...\" --action \"...\" --benefit \"...\"\n</system-reminder>";
const EXAMPLE_2_OUTPUT: &str =
    "✓ Created story AUTH-002\n  Title: OAuth integration\n  Epic: user-management";
const EXAMPLE_3_OUTPUT: &str =
    "✓ Created story AUTH-003\n  Title: Google login\n  Description: Support Google OAuth\n  Parent: AUTH-001";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec create-story AUTH \"User login feature\"",
        description: Some("Create simple story with Example Mapping guidance"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec create-story AUTH \"OAuth integration\" --epic=user-management",
        description: Some("Create story with epic"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command:
            "fspec create-story AUTH \"Google login\" --parent=AUTH-001 --description=\"Support Google OAuth\"",
        description: Some("Create child story with description"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const PREREQUISITES: &[&str] = &[
    "Prefix must be registered: fspec create-prefix PREFIX \"Description\"",
    "Epic must exist if using --epic: fspec create-epic EPIC \"Title\"",
    "Parent story must exist if using --parent",
];

const TYPICAL_WORKFLOW: &str = "Create story: fspec create-story PREFIX \"Title\",Follow Example Mapping guidance from system-reminder,Set user story: fspec set-user-story STORY-001 --role \"...\" --action \"...\" --benefit \"...\",Add rules: fspec add-rule STORY-001 \"Rule text\",Add examples: fspec add-example STORY-001 \"Example text\",Add questions: fspec add-question STORY-001 \"@human: Question?\",Generate scenarios: fspec generate-scenarios STORY-001,Move to specifying: fspec update-work-unit-status STORY-001 specifying";

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Prefix 'PREFIX' is not registered",
        fix: "undefined",
    },
    CommonError {
        error: "Parent story 'PARENT-001' does not exist",
        fix: "undefined",
    },
    CommonError {
        error: "Epic 'epic-name' does not exist",
        fix: "undefined",
    },
];

const RELATED: &[&str] = &[
    "fspec add-rule - Add business rule to story",
    "fspec add-example - Add concrete example to story",
    "fspec add-question - Add question for human clarification",
    "fspec set-user-story - Set user story fields (role, action, benefit)",
    "fspec generate-scenarios - Generate Gherkin scenarios from example map",
    "fspec update-work-unit-status - Move story through ACDD workflow",
];

const NOTES: &[&str] = &[
    "Stories use Example Mapping for collaborative requirement discovery",
    "System-reminder guides AI agents to use Example Mapping commands",
    "Stories require feature files with acceptance criteria",
    "Stories require tests before implementation (ACDD)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "create-story",
    description:
        "Create a new story with Example Mapping guidance for defining acceptance criteria",
    usage: Some("fspec create-story <prefix> <title> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when creating user-facing features that require Example Mapping to clarify acceptance criteria through collaborative conversation (rules, examples, questions).",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
