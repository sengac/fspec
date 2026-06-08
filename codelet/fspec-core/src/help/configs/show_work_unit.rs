//! `show-work-unit` help configuration — Rust port of
//! `src/commands/show-work-unit-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js show-work-unit --help`.
//! The captured fixture lives at
//! `codelet/fspec/tests/fixtures/help/show-work-unit.txt`; the scenario
//! `scenario_show_work_unit_help_matches_ts_formatcommandhelp_reference`
//! locks every byte of this output against the fixture.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPattern, CommonPatternEntry,
};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID (e.g., AUTH-001)",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "-f, --format <format>",
    description: "Output format: text (default) or json",
    default_value: None,
}];

const EX1_OUTPUT: &str = "AUTH-001\nType: story\nStatus: specifying\n\nUser login feature\nImplement user authentication\n\nRules:\n  1. Must validate email format\n  2. Password must be 8+ characters\n\nQuestions:\n  [0] Should we support OAuth?\n\nLinked Features:\n  spec/features/auth/login.feature\n    spec/features/auth/login.feature:10 - Valid user login\n\nCreated: 13/10/2025 14:30:00\nUpdated: 13/10/2025 15:45:00";

const EX2_OUTPUT: &str = "{\n  \"id\": \"AUTH-001\",\n  \"title\": \"User login feature\",\n  \"type\": \"story\",\n  \"status\": \"specifying\",\n  \"description\": \"Implement user authentication\",\n  \"rules\": [\"Must validate email format\"],\n  \"questions\": [\"[0] Should we support OAuth?\"],\n  \"linkedFeatures\": [...],\n  \"createdAt\": \"2025-10-13T14:30:00Z\",\n  \"updatedAt\": \"2025-10-13T15:45:00Z\"\n}";

const EX3_OUTPUT: &str = "EPIC-001\nType: epic\nStatus: implementing\n\nUser Management\n\nChildren: AUTH-001, AUTH-002, AUTH-003\n\nCreated: 01/10/2025 10:00:00";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec show-work-unit AUTH-001",
        description: Some("Show work unit details in text format"),
        output: Some(EX1_OUTPUT),
    },
    CommandExample {
        command: "fspec show-work-unit AUTH-001 --format json",
        description: Some("Show work unit details as JSON"),
        output: Some(EX2_OUTPUT),
    },
    CommandExample {
        command: "fspec show-work-unit EPIC-001",
        description: Some("Show epic details with children"),
        output: Some(EX3_OUTPUT),
    },
];

const PATTERN_VIEW_EXAMPLE: &str = "# Check current state\nfspec show-work-unit AUTH-001\n\n# Add rules/examples/questions\nfspec add-rule AUTH-001 \"Must validate email\"\nfspec add-example AUTH-001 \"User with valid credentials\"\nfspec add-question AUTH-001 \"Should we support OAuth?\" --human";

const PATTERN_EXPORT_EXAMPLE: &str = "# Export as JSON for scripts/tools\nfspec show-work-unit AUTH-001 --format json > auth-001.json\n\n# Process with jq\nfspec show-work-unit AUTH-001 --format json | jq .rules";

const PATTERN_REMINDERS_EXAMPLE: &str = "# View work unit to see reminders\nfspec show-work-unit AUTH-001\n\n# Output includes:\n# <system-reminder>\n# Missing estimate for work unit AUTH-001\n# </system-reminder>";

// The TS reference fixture renders the `description` field as the literal
// string "undefined" because the TS source's CommonPattern entries omit
// the `description` key and Commander's template-string interpolation
// stringifies `undefined`. The Rust port mirrors that surface byte-for-
// byte by setting the description to the literal "undefined" — see the
// captured fixture at codelet/fspec/tests/fixtures/help/show-work-unit.txt.
const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "View work unit before Example Mapping session",
        example: PATTERN_VIEW_EXAMPLE,
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Export work unit data for processing",
        example: PATTERN_EXPORT_EXAMPLE,
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Check system reminders",
        example: PATTERN_REMINDERS_EXAMPLE,
        description: "undefined",
    }),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-999' does not exist",
        fix: "Verify the work unit ID exists with: fspec list-work-units",
    },
    CommonError {
        error: "ENOENT: no such file or directory, spec/work-units.json",
        fix: "Initialize work units with: fspec create-story PREFIX \"title\" (or create-bug/create-task)",
    },
];

const RELATED: &[&str] = &[
    "list-work-units",
    "update-work-unit",
    "update-work-unit-status",
    "add-rule",
    "add-example",
    "add-question",
    "dependencies",
];

const NOTES: &[&str] = &[
    "Text format shows all Example Mapping data (rules, examples, questions, assumptions, architecture notes)",
    "Text format shows all dependency relationships (blocks, blockedBy, dependsOn, relatesTo)",
    "Text format shows linked feature files with scenario names and line numbers",
    "JSON format includes all fields, suitable for scripting and automation",
    "System reminders are displayed for missing estimates, empty Example Mapping, long phase duration, or large estimates (> 13 points for story/bug)",
    "Questions are filtered to show only unselected questions (answered questions are hidden)",
    "Linked features are automatically discovered by scanning feature files for work unit tags",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-work-unit",
    description: "Display detailed information about a work unit including Example Mapping data, dependencies, and linked feature files",
    usage: Some("fspec show-work-unit <workUnitId> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use to view complete details of a work unit: status, type, description, Example Mapping (rules, examples, questions, assumptions), dependencies (blocks/blockedBy/dependsOn/relatesTo), linked feature files, attachments, and system reminders. Supports both human-readable text and JSON output."),
    when_not_to_use: None,
    prerequisites: &["Work unit must exist in spec/work-units.json"],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some("1. List work units: fspec list-work-units → 2. Show details: fspec show-work-unit <workUnitId> → 3. Update fields: fspec update-work-unit <workUnitId> --title \"New title\" → 4. Verify changes: fspec show-work-unit <workUnitId>"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
