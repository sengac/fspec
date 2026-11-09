//! `add-attachment` help configuration — Rust port of
//! `src/commands/add-attachment-help.ts`.
//!
//! BUG-566: the TypeScript formatter printed `undefined` for keys this config
//! left unset; the config now holds the real text (each fix is the TS
//! `solution`, a pattern without a description prints none).

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "The ID of the work unit to attach the file to (e.g., AUTH-001, DASH-002)",
        required: true,
    },
    CommandArgument {
        name: "filePath",
        description: "Path to the file to attach. Can be absolute or relative to current directory. The file will be copied to spec/attachments/<workUnitId>/",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "-d, --description <text>",
    description: "Optional description of the attachment (what it represents, why it's relevant)",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-attachment AUTH-001 diagrams/auth-flow.png",
        description: Some("Add architecture diagram during discovery"),
        output: Some("✓ Attachment added successfully\n  File: spec/attachments/AUTH-001/auth-flow.png"),
    },
    CommandExample {
        command: "fspec add-attachment UI-002 mockups/dashboard.png --description \"Dashboard layout v2\"",
        description: Some("Add mockup with description"),
        output: Some("✓ Attachment added successfully\n  File: spec/attachments/UI-002/dashboard.png\n  Description: Dashboard layout v2"),
    },
    CommandExample {
        command: "fspec add-attachment API-003 docs/stripe-api-reference.pdf",
        description: Some("Add API documentation"),
        output: Some("✓ Attachment added successfully\n  File: spec/attachments/API-003/stripe-api-reference.pdf"),
    },
    CommandExample {
        command: "fspec add-attachment BUG-005 screenshots/error-state.png",
        description: Some("Add screenshot for bug reproduction"),
        output: Some("✓ Attachment added successfully\n  File: spec/attachments/BUG-005/error-state.png"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        fix: "Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'",
    },
    CommonError {
        error: "Source file 'diagram.png' does not exist",
        fix: "Check the file path is correct (absolute or relative to current directory)",
    },
    CommonError {
        error: "Attachment 'diagram.png' already exists",
        fix: "The file has already been attached to this work unit. Use 'fspec list-attachments AUTH-001' to see all attachments. Remove the old attachment first with 'fspec remove-attachment'.",
    },
];

const NOTES: &[&str] = &[
    "Attachments are copied to spec/attachments/<workUnitId>/ directory",
    "Original files are not modified",
    "Paths are stored as relative paths from project root",
    "All file types are supported (images, PDFs, documents, etc.)",
    "Attachments are version controlled (add to git)",
    "Use attachments to supplement, not replace, architecture notes",
];

// TS quirk #2: each entry already includes "fspec " prefix; the formatter
// prefixes again, producing "fspec fspec …".
const RELATED: &[&str] = &[
    "list-attachments - List all attachments for a work unit",
    "remove-attachment - Remove attachment",
    "show-work-unit - View work unit details (includes attachments)",
    "add-architecture-note - Add text-based architecture notes",
    "add-rule - Add business rules",
    "add-example - Add examples",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist (created with 'fspec create-story', 'fspec create-bug', or 'fspec create-task')",
    "Source file must exist at the specified path",
    "Work unit should be in 'specifying' or earlier status",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-attachment",
    description: "Add an attachment to a work unit during Example Mapping. Attachments are stored in spec/attachments/<workUnitId>/ and tracked as relative paths in the work unit.",
    usage: Some("fspec add-attachment <workUnitId> <filePath> [options]"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use attachments to supplement architecture notes and non-functional requirements with visual context (architecture diagrams, UI mockups, API documentation, screenshots, reference documents, or any supporting materials).",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "Create work unit: fspec create-story AUTH \"User Authentication\". Move to specifying: fspec update-work-unit-status AUTH-001 specifying. Start Example Mapping: Add rules, examples, questions. Add architecture notes: fspec add-architecture-note AUTH-001 \"Uses JWT tokens\". Add attachments: fspec add-attachment AUTH-001 docs/auth-flow.png. Continue discovery until all questions answered. Generate scenarios: fspec generate-scenarios AUTH-001",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
