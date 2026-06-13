//! `remove-persona` help configuration — Rust port of the TS
//! `remove-persona` Commander.js `--help` action.
//!
//! Byte-for-byte parity with `node dist/index.js remove-persona --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/remove-persona.txt`.

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
        fix: "undefined",
    },
    CommonError {
        error: "Persona \"Name\" not found",
        fix: "undefined",
    },
    CommonError {
        error: "No personas exist in foundation",
        fix: "undefined",
    },
];

const PREREQUISITES: &[&str] = &[
    "foundation.json or foundation.json.draft must exist",
    "Persona must exist with exact name match",
];

const RELATED: &[&str] = &[
    "fspec add-persona - Add persona to foundation",
    "fspec discover-foundation - Foundation discovery process",
    "fspec remove-capability - Remove capability from foundation",
    "fspec update-foundation - Update foundation fields",
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
        "List current personas: Check foundation.json or foundation.json.draft,Remove persona: fspec remove-persona \"exact name\",Verify removal: Check updated foundation file",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
