//! `show-event-storm` help configuration — Rust port of
//! `src/commands/show-event-storm-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js show-event-storm --help`
//! piped to non-TTY. Captured fixture:
//! `codelet/fspec/tests/fixtures/help/show-event-storm.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommonError, CommonPatternEntry,
};

const EXAMPLE_OUTPUT: &str = "{\n  \"items\": [\n    {\"id\": 0, \"type\": \"domain-event\", \"text\": \"UserRegistered\"},\n    {\"id\": 1, \"type\": \"command\", \"text\": \"RegisterUser\"},\n    {\"id\": 2, \"type\": \"policy\", \"text\": \"Send welcome email\", \"when\": \"UserRegistered\", \"then\": \"SendWelcomeEmail\"}\n  ],\n  \"nextItemId\": 3\n}";

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "work-unit-id",
    description: "Work unit ID to query",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec show-event-storm AUTH-001",
    description: Some("Display Event Storm artifacts for work unit"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &[
    "add-domain-event",
    "add-command",
    "add-policy",
    "add-hotspot",
    "show-foundation-event-storm",
    "generate-example-mapping-from-event-storm",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Work unit must have eventStorm section initialized",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Use for debugging Event Storm structure"),
    CommonPatternEntry::Bullet("Verify Event Storm artifacts before generating Example Mapping"),
    CommonPatternEntry::Bullet("Check item IDs and types"),
    CommonPatternEntry::Bullet("View all artifacts in a single work unit"),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit not found",
        fix: "Ensure work unit exists: fspec show-work-unit <id>",
    },
    CommonError {
        error: "No Event Storm artifacts",
        fix: "Add Event Storm items first: fspec add-domain-event <id> <text>",
    },
];

const NOTES: &[&str] = &[
    "Output is raw JSON (not formatted for human reading)",
    "Shows ALL items including deleted ones (soft-delete pattern)",
    "Use show-foundation-event-storm for foundation-level Event Storm",
    "Use generate-example-mapping-from-event-storm to convert to Example Mapping",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-event-storm",
    description: "Display Event Storm artifacts as JSON (no semantic interpretation)",
    usage: Some("fspec show-event-storm <work-unit-id>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to view all Event Storm artifacts (events, commands, policies, hotspots) for a specific work unit in JSON format.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
