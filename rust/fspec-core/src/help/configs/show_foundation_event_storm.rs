//! `show-foundation-event-storm` help configuration — Rust port of
//! `src/commands/show-foundation-event-storm-help.ts` (RPC-306).
//!
//! Byte-for-byte parity with `node dist/index.js show-foundation-event-storm --help`
//! piped to non-TTY. Reference fixture:
//! `rust/fspec/tests/fixtures/help/show-foundation-event-storm.txt`.

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPatternEntry,
};

const EXAMPLE_1_OUTPUT: &str =
    "{\n  \"boundedContexts\": [...],\n  \"pivotalEvents\": [...],\n  \"aggregates\": [...]\n}";

const EXAMPLE_2_OUTPUT: &str =
    "{\n  \"boundedContexts\": [\n    {\"id\": 0, \"name\": \"Identity\", \"description\": \"User authentication and authorization\"}\n  ]\n}";

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--type <type>",
    description: "Filter by Event Storm item type (bounded-context, pivotal-event, aggregate)",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec show-foundation-event-storm",
        description: Some("Display all foundation Event Storm artifacts"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec show-foundation-event-storm --type bounded-context",
        description: Some("Display only bounded contexts"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Use for strategic-level Event Storming (foundation scope)"),
    CommonPatternEntry::Bullet("View bounded contexts and their relationships"),
    CommonPatternEntry::Bullet("Check pivotal events that cross context boundaries"),
    CommonPatternEntry::Bullet("Verify aggregate structures"),
    CommonPatternEntry::Bullet(
        "Foundation Event Storm is for BIG PICTURE, work unit Event Storm is for TACTICAL",
    ),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "foundation.json not found",
        fix: "Initialize foundation first: fspec discover-foundation",
    },
    CommonError {
        error: "No Event Storm data in foundation",
        fix: "Add Big Picture Event Storm artifacts to foundation.json",
    },
];

const RELATED: &[&str] = &[
    "show-event-storm",
    "show-foundation",
    "discover-foundation",
    "generate-example-mapping-from-event-storm",
    "remove-foundation-bounded-context",
    "remove-aggregate-from-foundation",
    "remove-domain-event-from-foundation",
    "remove-command-from-foundation",
];

const PREREQUISITES: &[&str] = &[
    "foundation.json must exist",
    "Big Picture Event Storm must be initialized in foundation",
];

const NOTES: &[&str] = &[
    "Foundation Event Storm is stored in foundation.json (not work-units.json)",
    "Represents strategic domain model (DDD bounded contexts)",
    "Output is raw JSON format",
    "Use --type to filter specific artifact types",
    "Foundation-level artifacts define system architecture",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-foundation-event-storm",
    description: "Display foundation Event Storm artifacts as JSON (no semantic interpretation)",
    usage: Some("fspec show-foundation-event-storm [--type <type>]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to view foundation-level Event Storm artifacts (bounded contexts, pivotal events, aggregates) stored in foundation.json.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
