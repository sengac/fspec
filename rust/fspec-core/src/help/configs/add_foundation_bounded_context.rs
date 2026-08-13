//! `add-foundation-bounded-context` help configuration — Rust port of
//! `src/commands/add-foundation-bounded-context-help.ts`.
//!
//! Byte-for-byte parity with
//! `node dist/index.js add-foundation-bounded-context --help`.
//! Captured fixture:
//! `rust/fspec/tests/fixtures/help/add-foundation-bounded-context.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommonError, CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "text",
    description:
        "Bounded context name (e.g., \"Work Management\", \"Specification\", \"Testing & Validation\")",
    required: true,
}];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Foundation bounded contexts are strategic (Big Picture level)"),
    CommonPatternEntry::Bullet("Work unit bounded contexts are tactical (Process Modeling level)"),
    CommonPatternEntry::Bullet("Typically 3-8 major bounded contexts for most domains"),
    CommonPatternEntry::Bullet("Each bounded context has its own ubiquitous language"),
    CommonPatternEntry::Bullet("Foundation Event Storm informs tag ontology generation"),
    CommonPatternEntry::Bullet("Auto-regenerates FOUNDATION.md after adding bounded context"),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-foundation-bounded-context \"Work Management\"",
        description: Some("Add bounded context to foundation Event Storm"),
        output: Some(
            "✓ Added bounded context \"Work Management\" to foundation Event Storm\n✓ Regenerated FOUNDATION.md",
        ),
    },
    CommandExample {
        command: "fspec add-foundation-bounded-context \"Specification\"",
        description: Some("Add specification bounded context"),
        output: Some(
            "✓ Added bounded context \"Specification\" to foundation Event Storm\n✓ Regenerated FOUNDATION.md",
        ),
    },
    CommandExample {
        command: "fspec add-foundation-bounded-context \"Testing & Validation\"",
        description: Some("Add testing bounded context"),
        output: Some(
            "✓ Added bounded context \"Testing & Validation\" to foundation Event Storm\n✓ Regenerated FOUNDATION.md",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "spec/foundation.json not found",
    fix: "Run foundation discovery first: fspec discover-foundation",
}];

const PREREQUISITES: &[&str] = &[
    "spec/foundation.json must exist",
    "Bounded context should represent a strategic domain boundary",
    "Should be used for high-level domain architecture (not work unit-level)",
];

const RELATED: &[&str] = &[
    "remove-foundation-bounded-context",
    "add-aggregate-to-foundation",
    "add-domain-event-to-foundation",
    "add-command-to-foundation",
    "show-foundation-event-storm",
    "add-bounded-context",
];

const NOTES: &[&str] = &[
    "Foundation-level bounded contexts define strategic domain architecture",
    "Used for Big Picture Event Storming (not Process Modeling)",
    "Bounded contexts assigned stable IDs starting from 1",
    "Stored in foundation.json eventStorm section",
    "Auto-regenerates FOUNDATION.md documentation",
    "Use show-foundation-event-storm to view all foundation Event Storm items",
    "Foundation bounded contexts used for tag derivation (EXMAP-004)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-foundation-bounded-context",
    description:
        "Add bounded context to foundation-level Big Picture Event Storm for strategic domain boundaries",
    usage: Some("fspec add-foundation-bounded-context <text>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Big Picture Event Storming (foundation level) to identify major strategic boundaries (Bounded Contexts) in your domain architecture.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
