//! `generate-example-mapping-from-event-storm` help configuration — Rust port
//! of `src/commands/generate-example-mapping-from-event-storm-help.ts`.
//!
//! Byte-for-byte parity with
//! `node dist/index.js generate-example-mapping-from-event-storm --help`
//! piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/generate-example-mapping-from-event-storm.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommonError, CommonPatternEntry,
};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID containing Event Storm artifacts",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec generate-example-mapping-from-event-storm AUTH-001",
    description: Some("Convert Event Storm artifacts to Example Mapping"),
    output: Some(
        "✓ Generated 3 rule(s) from policies\n✓ Generated 5 example(s) from events\n✓ Generated 2 question(s) from hotspots",
    ),
}];

const RELATED: &[&str] = &[
    "add-domain-event",
    "add-command",
    "add-policy",
    "add-hotspot",
    "show-event-storm",
    "add-rule",
    "add-example",
    "add-question",
    "generate-scenarios",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Work unit must have Event Storm artifacts (policies, events, or hotspots)",
    "Event Storm section must be initialized",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Policies → Business rules (WHEN/THEN relationships)"),
    CommonPatternEntry::Bullet(
        "Events → Scenario examples (UserRegistered → \"User user registered and is logged in\")",
    ),
    CommonPatternEntry::Bullet("Hotspots → Questions for humans (concerns → @human questions)"),
    CommonPatternEntry::Bullet("Use after Event Storming, before generate-scenarios"),
    CommonPatternEntry::Bullet(
        "Bridge from strategic discovery (Event Storm) to tactical specification (Example Mapping)",
    ),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit not found",
        fix: "Ensure work unit exists: fspec show-work-unit <id>",
    },
    CommonError {
        error: "No Event Storm artifacts found",
        fix: "Add Event Storm items first: fspec add-domain-event, add-policy, etc.",
    },
];

const NOTES: &[&str] = &[
    "Policies converted to rules using WHEN/THEN pattern",
    "Events converted to examples using PascalCase → natural language",
    "Hotspot concerns converted to @human questions",
    "Skips deleted Event Storm items (deleted: true)",
    "Generated items include timestamps and proper ID sequencing",
    "After generation, use fspec generate-scenarios to create feature file",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "generate-example-mapping-from-event-storm",
    description:
        "Generate Example Mapping entries (rules, examples, questions) from Event Storm artifacts",
    usage: Some("fspec generate-example-mapping-from-event-storm <workUnitId>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use after completing Big Picture Event Storming to convert Event Storm artifacts (policies, events, hotspots) into Example Mapping for detailed specification.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
