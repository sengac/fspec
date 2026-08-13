//! `remove-dependency` help configuration — Rust port of
//! `src/commands/remove-dependency-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js remove-dependency --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/remove-dependency.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID to remove dependency from",
        required: true,
    },
    CommandArgument {
        name: "dependsOnId",
        description: "Work unit ID to remove from dependsOn (shorthand for --depends-on)",
        required: false,
    },
];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "--blocks <targetId>",
        description: "Remove blocks relationship (also removes reverse blockedBy from target)",
        default_value: None,
    },
    CommandOption {
        flag: "--blocked-by <targetId>",
        description: "Remove blockedBy relationship (also removes reverse blocks from target)",
        default_value: None,
    },
    CommandOption {
        flag: "--depends-on <targetId>",
        description: "Remove dependsOn relationship (unidirectional)",
        default_value: None,
    },
    CommandOption {
        flag: "--relates-to <targetId>",
        description: "Remove relatesTo relationship (also removes reverse relatesTo from target)",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-dependency AUTH-002 AUTH-001",
        description: Some("Shorthand: Remove AUTH-001 from AUTH-002 dependsOn list"),
        output: Some("✓ Dependency removed successfully"),
    },
    CommandExample {
        command: "fspec remove-dependency AUTH-002 --blocks API-001",
        description: Some("Remove blocks relationship (AUTH-002 no longer blocks API-001)"),
        output: Some("✓ Dependency removed successfully"),
    },
    CommandExample {
        command: "fspec remove-dependency UI-001 --blocked-by API-001",
        description: Some("Remove blockedBy relationship (UI-001 no longer blocked)"),
        output: Some("✓ Dependency removed successfully"),
    },
    CommandExample {
        command: "fspec remove-dependency DASH-001 --depends-on AUTH-001",
        description: Some("Explicit: Remove AUTH-001 from DASH-001 dependsOn list"),
        output: Some("✓ Dependency removed successfully"),
    },
    CommandExample {
        command: "fspec remove-dependency UI-005 --relates-to UI-004",
        description: Some("Remove relatesTo relationship (bidirectional cleanup)"),
        output: Some("✓ Dependency removed successfully"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-999' does not exist",
        fix: "Verify the work unit ID exists with: fspec list-work-units",
    },
    CommonError {
        error: "Must specify at least one relationship to remove: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to",
        fix: "Provide either a second argument (dependsOnId) or one of the relationship flags",
    },
    CommonError {
        error: "Cannot specify dependency both as argument and --depends-on option",
        fix: "Use either shorthand (two arguments) OR --depends-on flag, not both",
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Clean up completed blocker",
        example: "# After AUTH-001 is done, unblock dependent tasks\nfspec remove-dependency AUTH-001 --blocks API-001\nfspec remove-dependency AUTH-001 --blocks UI-001",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Remove all dependencies for a work unit",
        example: "# List current dependencies\nfspec dependencies AUTH-002\n\n# Remove each dependency\nfspec remove-dependency AUTH-002 AUTH-001\nfspec remove-dependency AUTH-002 --relates-to UI-001",
        description: "undefined",
    }),
];

const RELATED: &[&str] = &[
    "add-dependency",
    "dependencies",
    "clear-dependencies",
    "export-dependencies",
];

const NOTES: &[&str] = &[
    "Use shorthand syntax (two arguments) for simple dependsOn removals",
    "Blocks/blocked-by and relates-to relationships are bidirectional - removing from one side removes from both",
    "DependsOn relationships are unidirectional - only removed from the specified work unit",
    "The command will succeed even if the relationship does not exist (idempotent)",
];

const PREREQS: &[&str] = &[
    "Work units must exist in spec/work-units.json",
    "At least one dependency relationship must exist to remove",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-dependency",
    description:
        "Remove dependency relationships between work units to clean up outdated blockers and dependencies",
    usage: Some("fspec remove-dependency <workUnitId> [dependsOnId] [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command when a dependency relationship is no longer valid (task completed, blocker removed, or relationship was added by mistake). This is the inverse of add-dependency and supports bidirectional cleanup for blocks/blocked-by and relates-to relationships.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQS,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Verify dependency exists: fspec dependencies <workUnitId> → 2. Remove dependency: fspec remove-dependency <workUnitId> --blocks <targetId> → 3. Verify removal: fspec dependencies <workUnitId>",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
