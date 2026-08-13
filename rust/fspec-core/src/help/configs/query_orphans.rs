//! `query-orphans` help configuration — Rust port of
//! `src/commands/query-orphans-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js query-orphans --help`
//! when piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/query-orphans.txt`.

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const EX1_OUTPUT: &str = "Found 3 orphaned work unit(s):\n\n1. MISC-001 - Update documentation (backlog)\n   ⚠ No epic or dependency relationships\n   Suggested actions:\n     • Assign epic\n     • Add relationship\n     • Delete\n\n2. REFAC-003 - Refactor auth module (implementing)\n   ⚠ No epic or dependency relationships\n   Suggested actions:\n     • Assign epic\n     • Add relationship\n     • Delete\n\nTo fix orphaned work units:\n  fspec update-work-unit <id> --epic=<epic-name>\n  fspec add-dependency <id> --depends-on=<other-id>  (or --blocks, --relates-to)\n  fspec delete-work-unit <id>";

const EX2_OUTPUT: &str = "Found 1 orphaned work unit(s):\n\n1. MISC-001 - Update documentation (backlog)\n   ⚠ No epic or dependency relationships\n   Suggested actions:\n     • Assign epic\n     • Add relationship\n     • Delete";

const EX3_OUTPUT: &str = "{\n  \"orphans\": [\n    {\n      \"id\": \"MISC-001\",\n      \"title\": \"Update documentation\",\n      \"status\": \"backlog\",\n      \"suggestedActions\": [\"Assign epic\", \"Add relationship\", \"Delete\"]\n    }\n  ]\n}";

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--output <format>",
        description: "Output format: json or text",
        default_value: Some("text"),
    },
    CommandOption {
        flag: "--exclude-done",
        description: "Exclude work units in done status from orphan detection",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec query-orphans",
        description: Some("Find all orphaned work units (no epic, no relationships)"),
        output: Some(EX1_OUTPUT),
    },
    CommandExample {
        command: "fspec query-orphans --exclude-done",
        description: Some("Find orphans excluding completed work"),
        output: Some(EX2_OUTPUT),
    },
    CommandExample {
        command: "fspec query-orphans --output json",
        description: Some("Output orphans as JSON for automation"),
        output: Some(EX3_OUTPUT),
    },
    CommandExample {
        command: "fspec query-orphans | wc -l",
        description: Some("Count orphaned work units"),
        output: Some("15"),
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Maintenance Workflow",
        example: "# Weekly cleanup: find orphans\nfspec query-orphans\n\n# Assign epics to valid work\nfspec update-work-unit MISC-001 --epic=documentation\nfspec update-work-unit REFAC-003 --epic=authentication\n\n# Add relationships for cross-cutting work\nfspec add-dependency MISC-001 --relates-to AUTH-001\n\n# Delete invalid work units\nfspec delete-work-unit INVALID-001\n\n# Verify cleanup\nfspec query-orphans",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Bulk Work Unit Validation",
        example: "# After creating multiple work units\nfspec create-story AUTH \"Setup auth\"\nfspec create-story AUTH \"Login flow\"\nfspec create-story AUTH \"Logout flow\"\n\n# Check for orphans\nfspec query-orphans\n\n# Assign epic to all\nfspec update-work-unit AUTH-001 --epic=authentication\nfspec update-work-unit AUTH-002 --epic=authentication\nfspec update-work-unit AUTH-003 --epic=authentication",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "JSON Export for Reporting",
        example: "# Export orphans for dashboard\nfspec query-orphans --output json > orphans.json\n\n# Count orphans by status\njq '[.orphans[].status] | group_by(.) | map({status: .[0], count: length})' orphans.json",
        description: "undefined",
    }),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: No orphaned work units found",
        fix: "All work units have epic or dependency relationships. No action needed.",
    },
    CommonError {
        error: "Error: work-units.json not found",
        fix: "Run: fspec init to create work-units.json file",
    },
    CommonError {
        error: "Error: Invalid work-units.json format",
        fix: "Check for JSON syntax errors in spec/work-units.json",
    },
];

const RELATED: &[&str] = &[
    "query-bottlenecks",
    "suggest-dependencies",
    "update-work-unit",
    "add-dependency",
    "delete-work-unit",
    "list-epics",
];

const NOTES: &[&str] = &[
    "A work unit is orphaned if it has NEITHER epic NOR dependency relationships",
    "Orphaned work units lack context and traceability (unclear WHY they exist)",
    "By default, includes ALL statuses (even \"done\" work can be orphaned)",
    "Use --exclude-done to focus on active work units needing attention",
    "Orphans often indicate ad-hoc work added without proper planning",
    "Suggested actions: (1) Assign epic, (2) Add relationship, (3) Delete if invalid",
    "Regular orphan detection prevents unplanned scope creep",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "query-orphans",
    description:
        "Detect orphaned work units with no epic assignment or dependency relationships",
    usage: Some("fspec query-orphans [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to identify work units lacking context and traceability. Essential for work unit hygiene, preventing unplanned scope creep, and ensuring all work aligns with epics or has clear relationships. Run after bulk work unit creation or periodically for maintenance.",
    ),
    when_not_to_use: None,
    prerequisites: &["spec/work-units.json exists with work units"],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Run orphan detection: fspec query-orphans → 2. Review orphaned work units → 3. Assign epic OR add relationship: fspec update-work-unit <id> --epic=<epic> OR fspec add-dependency <id> --relates-to=<other-id> → 4. Delete if invalid: fspec delete-work-unit <id> → 5. Verify: fspec query-orphans",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
