//! `validate-spec-alignment` help configuration.
//!
//! CLI-016: the help describes the command the CLI parses — a required
//! `<workUnitId>` and an accepted `--fix` that changes nothing — instead of the
//! captured TypeScript `[feature-files...]` shape, which the CLI never took.
//! Fixture: `rust/fspec/tests/fixtures/help/validate-spec-alignment.txt`.
use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit whose scenarios are checked (scenarios tagged @<workUnitId>)",
    required: true,
}];

const OPTS: &[CommandOption] = &[CommandOption {
    flag: "--fix",
    description: "Accepted for compatibility; makes no changes",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec validate-spec-alignment AUTH-001",
        description: Some("Check that AUTH-001 has tagged scenarios"),
        output: Some("✓ All specs are aligned with tests and implementation"),
    },
    CommandExample {
        command: "fspec validate-spec-alignment DASH-002",
        description: Some("A work unit without tagged scenarios"),
        output: Some("  - No scenarios for DASH-002 (exit code 1)"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Failed to validate spec alignment: Work unit AUTH-999 not found",
        fix: "Check the work unit ID: fspec list-work-units",
    },
    CommonError {
        error: "No scenarios for AUTH-001",
        fix: "Tag the work unit's scenarios: fspec add-tag-to-scenario <feature> \"<scenario>\" @AUTH-001",
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[CommonPatternEntry::Structured(CommonPattern {
    pattern: "Check before moving to done",
    example: "# Check that the work unit has tagged scenarios\nfspec validate-spec-alignment AUTH-001\n\n# Check its coverage mappings\nfspec audit --feature user-authentication --area coverage\n\n# Move to done\nfspec update-work-unit-status AUTH-001 done",
    description: None,
})];

const RELATED: &[&str] = &[
    "validate",
    "link-coverage",
    "show-coverage",
    "audit",
    "update-work-unit-status",
];

const NOTES: &[&str] = &[
    "A scenario counts when the line before its Scenario: line carries @<workUnitId>",
    "Exits 1 and prints the warning when the work unit has no tagged scenarios",
    "Use fspec audit --area coverage to check a feature's test and implementation mappings",
];

const PREREQS: &[&str] = &[
    "The work unit must exist in spec/work-units.json",
    "Feature files live in spec/features/",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "validate-spec-alignment",
    description: "Check that a work unit has scenarios tagged with its ID",
    usage: Some("fspec validate-spec-alignment <workUnitId> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use in the validating phase to confirm the work unit's scenarios are tagged with its ID before moving it to done.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQS,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Complete implementation → 2. Link coverage: fspec link-coverage <feature> → 3. Check tags: fspec validate-spec-alignment <workUnitId> → 4. Move to done: fspec update-work-unit-status <workUnitId> done",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
