//! `workflow-automation` help configuration — Rust port of
//! `src/commands/workflow-automation-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js workflow-automation --help`.
//!
//! NOTE: each [`CommonPattern`] carries `description: "undefined"` on purpose.
//! The TS config sets only `pattern` + `example` (no `description`), so the TS
//! `formatCommandHelp` renders the literal string `undefined` on the
//! description line. The captured fixture echoes that, so we reproduce it.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "action",
        description: "Action to perform: record-iteration, auto-advance, validate-alignment",
        required: true,
    },
    CommandArgument {
        name: "work-unit-id",
        description: "Work unit ID (required for all actions)",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--event <event>",
        description: "Event triggering state change: tests-pass, validation-pass, specs-complete (used with auto-advance)",
        default_value: None,
    },
    CommandOption {
        flag: "--from-state <state>",
        description: "Current state before transition (used with auto-advance for safety check)",
        default_value: None,
    },
];

const PREREQUISITES: &[&str] = &[
    "spec/work-units.json exists with work units",
    "Work unit is in appropriate state for action",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "TDD Cycle Tracking",
        example: "# Start testing phase\nfspec update-work-unit-status AUTH-001 testing\n\n# Red: write failing test\n<test-command>  # Tests fail\nfspec workflow-automation record-iteration AUTH-001\n\n# Green: implement code\n<test-command>  # Tests pass\nfspec workflow-automation auto-advance AUTH-001 --event tests-pass --from-state testing\n\n# Now in implementing state, ready for next feature",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "ACDD State Automation",
        example: "# Specifying → Testing (after specs complete)\nfspec workflow-automation auto-advance AUTH-001 --event specs-complete --from-state specifying\n\n# Testing → Implementing (after tests pass)\nfspec workflow-automation auto-advance AUTH-001 --event tests-pass --from-state testing\n\n# Validating → Done (after validation passes)\nfspec workflow-automation auto-advance AUTH-001 --event validation-pass --from-state validating",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Spec Alignment Validation",
        example: "# After creating feature files, validate alignment\nfspec workflow-automation validate-alignment AUTH-001\n\n# If not aligned, add @AUTH-001 tag to scenarios\nfspec add-scenario user-authentication \"Login flow\" --tags @AUTH-001\n\n# Re-validate\nfspec workflow-automation validate-alignment AUTH-001",
        description: "undefined",
    }),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec workflow-automation record-iteration AUTH-001",
        description: Some("Increment iteration counter (tracks red→green cycles)"),
        output: Some("✓ Recorded iteration for AUTH-001\n  Total iterations: 3"),
    },
    CommandExample {
        command: "fspec workflow-automation auto-advance AUTH-001 --event tests-pass --from-state testing",
        description: Some("Auto-advance from testing → implementing after tests pass"),
        output: Some(
            "✓ Advanced AUTH-001 from testing → implementing\n  Event: tests-pass\n  Updated: 2025-01-15T10:30:00Z",
        ),
    },
    CommandExample {
        command: "fspec workflow-automation auto-advance AUTH-001 --event validation-pass --from-state validating",
        description: Some("Auto-advance from validating → done after validation passes"),
        output: Some("✓ Advanced AUTH-001 from validating → done\n  Event: validation-pass"),
    },
    CommandExample {
        command: "fspec workflow-automation validate-alignment AUTH-001",
        description: Some("Check if work unit has corresponding Gherkin scenarios"),
        output: Some(
            "✓ Work unit AUTH-001 is aligned with specifications\n  Scenarios found: 5\n  Features:\n    - spec/features/user-authentication.feature\n    - spec/features/session-management.feature",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Work unit does not exist",
        fix: "Verify work unit ID. Run: fspec list-work-units",
    },
    CommonError {
        error: "Error: Invalid transition: tests-pass from implementing",
        fix: "tests-pass event only valid from \"testing\" state. Check current state with: fspec show-work-unit <id>",
    },
    CommonError {
        error: "Error: Work unit is in state \"implementing\", expected \"testing\"",
        fix: "State mismatch. Verify current state before auto-advance. Use --from-state to ensure safety.",
    },
    CommonError {
        error: "Error: No scenarios found for work unit AUTH-001",
        fix: "Work unit has no @AUTH-001 tagged scenarios. Add tag to scenarios or create feature file.",
    },
];

const RELATED: &[&str] = &[
    "update-work-unit-status",
    "show-work-unit",
    "create-story",
    "create-bug",
    "create-task",
    "list-features",
    "add-scenario",
];

const NOTES: &[&str] = &[
    "Metrics tracked:",
    "  - iterations: Number of red→green cycles (TDD iterations)",
    "State transitions (auto-advance):",
    "  - specs-complete: specifying → testing",
    "  - tests-pass: testing → implementing",
    "  - validation-pass: validating → done",
    "Auto-advance safety:",
    "  - Requires --from-state to verify current state before transition",
    "  - Prevents invalid state transitions",
    "  - Updates stateHistory with timestamp",
    "Spec alignment:",
    "  - Searches for @WORK-UNIT-ID tags in feature files",
    "  - Returns count of matching scenarios and feature files",
    "  - Essential for reverse ACDD to track specification coverage",
    "Iteration tracking:",
    "  - Increments on each red→green cycle",
    "  - Useful metric for complexity/difficulty assessment",
    "  - High iteration count may indicate unclear requirements",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "workflow-automation",
    description: "Workflow automation utilities for work units (record iterations, auto-advance state, validate spec alignment)",
    usage: Some(
        "fspec workflow-automation <action> [work-unit-id] [options]\n\nActions:\n  record-iteration     - Increment iteration counter for work unit\n  auto-advance         - Auto-advance state after event (tests-pass, validation-pass, specs-complete)\n  validate-alignment   - Check if work unit has corresponding Gherkin scenarios",
    ),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command for workflow automation and metrics tracking. Essential for AI agents to track progress, measure efficiency (iterations), auto-advance work unit states after successful events, and validate alignment between work units and Gherkin specs.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Start work: fspec update-work-unit-status <id> testing → 2. Write tests (red) → 3. Record iteration: fspec workflow-automation record-iteration <id> → 4. Implement (green) → 5. Tests pass → 6. Auto-advance: fspec workflow-automation auto-advance <id> --event tests-pass --from-state testing → 7. Validate alignment: fspec workflow-automation validate-alignment <id>",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
