//! `remove-attachment` help configuration — Rust port of
//! `src/commands/remove-attachment-help.ts`.
//!
//! See `add_attachment.rs` for the three TS quirks (comma-joined workflow,
//! double-`fspec` related commands, `Fix: undefined`).

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "The ID of the work unit to remove the attachment from (e.g., AUTH-001)",
        required: true,
    },
    CommandArgument {
        name: "fileName",
        description: "The name of the file to remove (e.g., diagram.png). Must match the basename of the attachment path.",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--keep-file",
    description: "Keep the file on disk (only remove from work unit tracking)",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-attachment AUTH-001 auth-flow.png",
        description: Some("Remove attachment and delete file"),
        output: Some("✓ Attachment removed from work unit and file deleted\n  File: spec/attachments/AUTH-001/auth-flow.png"),
    },
    CommandExample {
        command: "fspec remove-attachment AUTH-001 important-doc.pdf --keep-file",
        description: Some("Remove attachment but keep file on disk"),
        output: Some("✓ Attachment removed from work unit (file kept)\n  File: spec/attachments/AUTH-001/important-doc.pdf"),
    },
    CommandExample {
        command: "fspec remove-attachment AUTH-001 missing-file.png",
        description: Some("Remove attachment when file is already missing"),
        output: Some("⚠ Attachment removed from work unit (file was already missing)\n  File: spec/attachments/AUTH-001/missing-file.png"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        fix: "undefined",
    },
    CommonError {
        error: "Work unit 'AUTH-001' has no attachments to remove",
        fix: "undefined",
    },
    CommonError {
        error: "Attachment 'diagram.png' not found for work unit 'AUTH-001'",
        fix: "undefined",
    },
];

const NOTES: &[&str] = &[
    "By default, both tracking entry and file are removed",
    "Use --keep-file to preserve the file on disk",
    "Removing attachment updates work unit timestamps",
    "File name is matched against basename of attachment paths",
    "If file doesn't exist, only tracking entry is removed (warning shown)",
];

// TS quirk: each entry already includes "fspec " prefix; the formatter
// prefixes again, producing "fspec fspec …".
const RELATED: &[&str] = &[
    "fspec add-attachment - Add attachment",
    "fspec list-attachments - List all attachments",
    "fspec show-work-unit - View work unit details",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Attachment must be tracked in the work unit",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-attachment",
    description: "Remove an attachment from a work unit. By default, this removes both the tracking entry and deletes the file from disk. Use --keep-file to preserve the file.",
    usage: Some("fspec remove-attachment <workUnitId> <fileName> [options]"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to remove outdated or incorrect attachments, to clean up broken attachment links, to remove duplicate attachments, or when attachment is no longer relevant to the work unit.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "List attachments: fspec list-attachments AUTH-001,Identify attachment to remove,Remove attachment: fspec remove-attachment AUTH-001 old-diagram.png,Verify removal: fspec list-attachments AUTH-001",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
