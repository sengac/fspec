//! `discover-event-storm` help configuration — Rust port of
//! `src/commands/discover-event-storm-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js discover-event-storm --help`
//! piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/discover-event-storm.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommonError, CommonPatternEntry,
};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID to conduct Event Storm discovery on",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec discover-event-storm AUTH-001",
        description: Some("Start Event Storm discovery session for authentication work unit"),
        output: Some(
            "✓ Event Storm guidance emitted\n\nFollow the guidance to capture domain events, commands, policies, and hotspots.",
        ),
    },
    CommandExample {
        command: "fspec discover-event-storm CHECKOUT-001",
        description: Some("Start Event Storm discovery for checkout feature"),
        output: Some("✓ Event Storm guidance emitted"),
    },
];

const RELATED: &[&str] = &[
    "add-domain-event",
    "add-command",
    "add-policy",
    "add-hotspot",
    "add-aggregate",
    "add-bounded-context",
    "add-external-system",
    "show-event-storm",
    "generate-example-mapping-from-event-storm",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Work unit should be in specifying status",
    "High domain complexity or unclear requirements",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet(
        "Run discover-event-storm BEFORE Example Mapping for complex domains",
    ),
    CommonPatternEntry::Bullet(
        "Follow free-form collaborative session (not field-by-field like discover-foundation)",
    ),
    CommonPatternEntry::Bullet(
        "Capture artifacts using add-domain-event, add-command, add-policy, add-hotspot",
    ),
    CommonPatternEntry::Bullet("Stop when shared understanding is reached (~25 minutes)"),
    CommonPatternEntry::Bullet(
        "Transform to Example Mapping: fspec generate-example-mapping-from-event-storm",
    ),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit not found",
        fix: "Ensure work unit exists: fspec show-work-unit <id>",
    },
    CommonError {
        error: "Work unit not in specifying status",
        fix: "Move work unit to specifying: fspec update-work-unit-status <id> specifying",
    },
    CommonError {
        error: "spec/work-units.json not found",
        fix: "Initialize fspec first: fspec init",
    },
];

const NOTES: &[&str] = &[
    "Event Storm is a workshop technique invented by Alberto Brandolini",
    "Use sticky notes metaphor: orange (events), blue (commands), purple (policies), red (hotspots)",
    "Two levels: Big Picture (foundation) and Process Modeling (work unit)",
    "Guidance emitted as system-reminder for AI agents",
    "Free-form session - use commands in any order as needed",
    "Stop when: all major events captured, commands identified, policies documented, hotspots recorded",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "discover-event-storm",
    description: "Emit Event Storm guidance for domain discovery through collaborative workshop",
    usage: Some("fspec discover-event-storm <workUnitId>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use BEFORE Example Mapping when domain complexity is high, events/commands/policies are unclear, or you need to model complex business workflows collaboratively.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
