//! `remove-persona` help configuration — Rust port of the TS
//! `remove-persona` Commander.js `--help` action.
//!
//! Byte-for-byte parity with `node dist/index.js remove-persona --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/remove-persona.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "name",
    description:
        "Exact name of the persona to remove (case-sensitive). Must match existing persona name exactly.",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-persona \"Developer\"",
        description: Some("Remove persona by exact name"),
        output: Some("✓ Removed persona \"Developer\" from foundation.json.draft"),
    },
    CommandExample {
        command: "fspec remove-persona \"End User\"",
        description: Some("Remove persona from finalized foundation"),
        output: Some("✓ Removed persona \"End User\" from foundation.json"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "foundation.json not found",
        fix: "Run: fspec discover-foundation to create foundation.json.draft",
    },
    CommonError {
        error: "Persona \"Name\" not found",
        fix: "Check exact name (case-sensitive) in foundation.json. Available personas are listed in the error message.",
    },
    CommonError {
        error: "No personas exist in foundation",
        fix: "Cannot remove personas from empty foundation. Add personas first with fspec add-persona.",
    },
];

const PREREQUISITES: &[&str] = &[
    "foundation.json or foundation.json.draft must exist",
    "Persona must exist with exact name match",
];

const RELATED: &[&str] = &[
    "add-persona - Add persona to foundation",
    "discover-foundation - Foundation discovery process",
    "remove-capability - Remove capability from foundation",
    "update-foundation - Update foundation fields",
];

const NOTES: &[&str] = &[
    "Draft file (foundation.json.draft) takes precedence over foundation.json",
    "Name matching is case-sensitive and must be exact",
    "Removal is permanent - no undo functionality",
    "Error message shows available personas if name not found",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-persona",
    description: "Remove a persona from foundation.json or foundation.json.draft by exact name match (case-sensitive). Used to correct mistakes during foundation discovery or remove obsolete personas.",
    usage: Some("fspec remove-persona \"<name>\""),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during foundation discovery to correct mistakes, or when updating an existing foundation to remove personas that are no longer relevant to the system.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "List current personas: Check foundation.json or foundation.json.draft. Remove persona: fspec remove-persona \"exact name\". Verify removal: Check updated foundation file",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
