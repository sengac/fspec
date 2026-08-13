//! `generate-coverage` help configuration — Rust port of
//! `src/commands/generate-coverage-help.ts` (RPC-231).
//!
//! Byte-for-byte parity with `node dist/index.js generate-coverage --help`
//! piped to non-TTY. Reference fixture:
//! `rust/fspec/tests/fixtures/help/generate-coverage.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--dry-run",
    description: "Preview what coverage files would be created without actually creating them",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec generate-coverage",
        description: Some("Generate or update coverage files for all .feature files"),
        output: Some("✓ Created 2, Updated 1, Skipped 3\n\nCreated: user-authentication.feature.coverage, user-registration.feature.coverage\nUpdated: existing-feature.feature.coverage (added 2 missing scenarios)\nSkipped: already-up-to-date.feature.coverage"),
    },
    CommandExample {
        command: "fspec generate-coverage --dry-run",
        description: Some("Preview what would be created without creating files"),
        output: Some("[DRY RUN] Would generate coverage for:\n  - user-authentication.feature (3 scenarios)\n  - user-registration.feature (2 scenarios)\n\nWould create 2 coverage files"),
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Reverse ACDD Setup: Run generate-coverage to create empty coverage structures, then use link-coverage to map scenarios to tests"),
    CommonPatternEntry::Bullet("Post-Recovery: After accidentally deleting coverage files, run generate-coverage to recreate empty structures"),
    CommonPatternEntry::Bullet("Project Migration: When adding coverage tracking to existing fspec project, use --dry-run first to preview"),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: No feature files found in spec/features/",
        fix: "Ensure you have .feature files in spec/features/ directory. Create features with: fspec create-feature",
    },
    CommonError {
        error: "Error: Invalid Gherkin syntax in feature file",
        fix: "Fix Gherkin syntax errors first. Run: fspec validate to identify issues.",
    },
];

const RELATED: &[&str] = &[
    "create-feature",
    "link-coverage",
    "show-coverage",
    "audit-coverage",
];

const NOTES: &[&str] = &[
    "Coverage files are automatically created by fspec create-feature for new features",
    "This command creates new coverage files AND updates existing ones with missing scenarios",
    "Automatically removes stale scenarios (in coverage but not in feature file)",
    "Existing test mappings are preserved when updating coverage files",
    "New scenarios are added with empty testMappings arrays",
    "Coverage statistics are recalculated after adding/removing scenarios",
    "Safe to run anytime - idempotent and preserves existing data",
    "Use --dry-run to preview before making changes",
    "Returns status: created, updated, skipped, or recreated (if invalid JSON)",
];

const PREREQUISITES: &[&str] = &[
    "Valid .feature files must exist in spec/features/",
    "Feature files must have valid Gherkin syntax (run fspec validate first)",
];

const ARGUMENTS: &[CommandArgument] = &[];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "generate-coverage",
    description: "Generate or update .feature.coverage files for existing .feature files. Creates new coverage files or updates existing ones with missing scenarios.",
    usage: Some("fspec generate-coverage [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use this command when: 1) Setting up coverage tracking for existing features, 2) You added new scenarios to existing .feature files and need to update .coverage files, 3) .feature.coverage files were accidentally deleted. Essential for reverse ACDD setup and maintaining coverage sync."),
    when_not_to_use: Some("Do not use for creating new features (use fspec create-feature instead, which auto-creates coverage files). This command is safe to run anytime - it preserves existing test mappings."),
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some("1. Run fspec generate-coverage --dry-run to preview → 2. Review what will be created → 3. Run fspec generate-coverage to create files → 4. Verify with fspec show-coverage"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
