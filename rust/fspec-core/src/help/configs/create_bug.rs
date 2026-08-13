//! `create-bug` help configuration — Rust port of
//! `src/commands/create-bug-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js create-bug --help`.
//!
//! NOTE on quirks faithfully reproduced from the TS reference:
//! * `relatedCommands` entries already carry a `fspec ` prefix, so the
//!   formatter renders them as `fspec fspec search-scenarios - ...`.
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
            "Bug prefix (e.g., BUG, FIX, HOTFIX). Must be registered with create-prefix first.",
        required: true,
    },
    CommandArgument {
        name: "title",
        description: "Brief description of the bug",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "-d, --description <description>",
        description: "Detailed description of the bug",
        default_value: None,
    },
    CommandOption {
        flag: "-e, --epic <epic>",
        description: "Epic ID to associate with this bug",
        default_value: None,
    },
    CommandOption {
        flag: "-p, --parent <parent>",
        description: "Parent bug ID for hierarchical relationships",
        default_value: None,
    },
];

const EXAMPLE_1_OUTPUT: &str = "✓ Created bug BUG-001\n  Title: Login validation broken\n\n<system-reminder>\nBug BUG-001 created successfully.\n\nCRITICAL: Research existing code FIRST before fixing bugs.\n  fspec search-scenarios --query=\"login\"\n  fspec search-implementation --function=\"validateLogin\"\n  fspec show-coverage\n</system-reminder>";
const EXAMPLE_2_OUTPUT: &str =
    "✓ Created bug BUG-002\n  Title: Memory leak in dashboard\n  Epic: performance";
const EXAMPLE_3_OUTPUT: &str =
    "✓ Created bug HOTFIX-001\n  Title: Critical auth bypass\n  Description: CVE-2024-1234";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec create-bug BUG \"Login validation broken\"",
        description: Some("Create simple bug with research guidance"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec create-bug BUG \"Memory leak in dashboard\" --epic=performance",
        description: Some("Create bug with epic"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec create-bug HOTFIX \"Critical auth bypass\" --description=\"CVE-2024-1234\"",
        description: Some("Create critical bug with description"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const PREREQUISITES: &[&str] = &[
    "Prefix must be registered: fspec create-prefix PREFIX \"Description\"",
    "Epic must exist if using --epic: fspec create-epic EPIC \"Title\"",
    "Parent bug must exist if using --parent",
];

const TYPICAL_WORKFLOW: &str = "Create bug: fspec create-bug PREFIX \"Title\",Follow research guidance from system-reminder,Search scenarios: fspec search-scenarios --query=\"keyword\",Search implementation: fspec search-implementation --function=\"functionName\",Check coverage: fspec show-coverage feature-name,Add reproduction steps: fspec add-example BUG-001 \"Reproduction steps\",Add fix scenarios: fspec add-rule BUG-001 \"Expected behavior\",Move to specifying: fspec update-work-unit-status BUG-001 specifying";

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Prefix 'PREFIX' is not registered",
        fix: "undefined",
    },
    CommonError {
        error: "Parent bug 'PARENT-001' does not exist",
        fix: "undefined",
    },
    CommonError {
        error: "Epic 'epic-name' does not exist",
        fix: "undefined",
    },
];

const RELATED: &[&str] = &[
    "fspec search-scenarios - Search existing scenarios by keyword",
    "fspec search-implementation - Search implementation for function usage",
    "fspec show-coverage - Check test coverage for features",
    "fspec list-features - List all feature files",
    "fspec add-rule - Add expected behavior rule",
    "fspec add-example - Add reproduction steps",
    "fspec update-work-unit-status - Move bug through ACDD workflow",
];

const NOTES: &[&str] = &[
    "Bugs require research BEFORE fixing to prevent regressions",
    "System-reminder guides AI agents to use research commands",
    "Bugs should link to existing features when fixing behavior",
    "Bugs may or may not require new tests (depends on coverage gaps)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "create-bug",
    description:
        "Create a new bug with research guidance for understanding existing code before fixing",
    usage: Some("fspec create-bug <prefix> <title> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when tracking bugs that require researching existing scenarios, implementation, and test coverage before fixing to prevent regression.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
