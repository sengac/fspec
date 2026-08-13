//! `add-capability` help configuration — Rust port of
//! `src/commands/add-capability-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-capability --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/add-capability.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "name",
        description:
            "Capability name (e.g., \"User Authentication\", \"Data Export\"). Should describe what users can do, not implementation details.",
        required: true,
    },
    CommandArgument {
        name: "description",
        description:
            "Description of the capability from user perspective. Focus on WHAT users can achieve, not HOW it works.",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command:
            "fspec add-capability \"User Authentication\" \"Users can register, login, and manage their accounts\"",
        description: Some("Add capability during foundation discovery"),
        output: Some(
            "✓ Added capability to foundation.json.draft\n  Name: User Authentication\n  Description: Users can register, login, and manage their accounts",
        ),
    },
    CommandExample {
        command:
            "fspec add-capability \"Data Export\" \"Users can export their data in multiple formats (CSV, JSON, PDF)\"",
        description: Some("Add capability to existing foundation"),
        output: Some(
            "✓ Added capability to foundation.json\n  Name: Data Export\n  Description: Users can export their data in multiple formats (CSV, JSON, PDF)",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "foundation.json not found",
        fix: "undefined",
    },
    CommonError {
        error: "Capability already exists",
        fix: "undefined",
    },
];

const PREREQUISITES: &[&str] = &[
    "foundation.json or foundation.json.draft must exist",
    "Run fspec discover-foundation to create foundation.json.draft if needed",
];

const RELATED: &[&str] = &[
    "fspec discover-foundation - Start foundation discovery process",
    "fspec remove-capability - Remove capability from foundation",
    "fspec add-persona - Add user persona to foundation",
    "fspec update-foundation - Update foundation fields",
];

const NOTES: &[&str] = &[
    "Draft file (foundation.json.draft) takes precedence over foundation.json",
    "Capabilities describe WHAT users can do, not HOW system works",
    "Focus on user-facing functionality, not implementation details",
    "Capabilities are part of the solution space (what system provides)",
    "Use present tense and active voice (e.g., \"Users can...\" not \"User will...\")",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-capability",
    description:
        "Add a capability to foundation.json or foundation.json.draft. Capabilities describe WHAT the system CAN DO (from the user perspective), not HOW it works internally. Used during foundation discovery to incrementally build the solution space.",
    usage: Some("fspec add-capability \"<name>\" \"<description>\""),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during fspec discover-foundation workflow when adding capabilities to foundation.json.draft, or when adding new capabilities to an existing foundation.json after initial discovery.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "Start foundation discovery: fspec discover-foundation,AI analyzes codebase and identifies capabilities,Add each capability: fspec add-capability \"name\" \"description\",Repeat for all capabilities,Finalize foundation: fspec discover-foundation --finalize",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
