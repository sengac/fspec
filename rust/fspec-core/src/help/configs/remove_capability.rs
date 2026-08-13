//! `remove-capability` help configuration — Rust port of
//! `src/commands/remove-capability-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js remove-capability --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/remove-capability.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "name",
    description:
        "Exact name of the capability to remove (case-sensitive). Must match existing capability name exactly.",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-capability \"User Authentication\"",
        description: Some("Remove capability by exact name"),
        output: Some("✓ Removed capability \"User Authentication\" from foundation.json.draft"),
    },
    CommandExample {
        command: "fspec remove-capability \"Data Export\"",
        description: Some("Remove capability from finalized foundation"),
        output: Some("✓ Removed capability \"Data Export\" from foundation.json"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "foundation.json not found",
        fix: "undefined",
    },
    CommonError {
        error: "Capability \"Name\" not found",
        fix: "undefined",
    },
    CommonError {
        error: "No capabilities exist in foundation",
        fix: "undefined",
    },
];

const PREREQUISITES: &[&str] = &[
    "foundation.json or foundation.json.draft must exist",
    "Capability must exist with exact name match",
];

const RELATED: &[&str] = &[
    "fspec add-capability - Add capability to foundation",
    "fspec discover-foundation - Foundation discovery process",
    "fspec remove-persona - Remove persona from foundation",
    "fspec update-foundation - Update foundation fields",
];

const NOTES: &[&str] = &[
    "Draft file (foundation.json.draft) takes precedence over foundation.json",
    "Name matching is case-sensitive and must be exact",
    "Removal is permanent - no undo functionality",
    "Error message shows available capabilities if name not found",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-capability",
    description:
        "Remove a capability from foundation.json or foundation.json.draft by exact name match (case-sensitive). Used to correct mistakes during foundation discovery or remove obsolete capabilities.",
    usage: Some("fspec remove-capability \"<name>\""),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during foundation discovery to correct mistakes, or when updating an existing foundation to remove capabilities that are no longer part of the system.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "List current capabilities: Check foundation.json or foundation.json.draft,Remove capability: fspec remove-capability \"exact name\",Verify removal: Check updated foundation file",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
