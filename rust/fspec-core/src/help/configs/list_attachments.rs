//! `list-attachments` help configuration — Rust port of
//! `src/commands/list-attachments-help.ts`.
//!
//! IMPORTANT — this config preserves three TS quirks for byte-parity:
//!   1. `typical_workflow` mirrors TS where the source is `string[]` but the
//!      formatter renders `${config.typicalWorkflow}` — i.e. the JS Array's
//!      `.toString()` which is comma-joined. We pre-join with `,` here.
//!   2. `related_commands` entries in TS already include the literal "fspec "
//!      prefix (e.g. `"fspec add-attachment - Add attachment"`), and the
//!      help-formatter prefixes them AGAIN with `fspec ` — producing
//!      `fspec fspec add-attachment - Add attachment`. We mirror this here.
//!   3. `common_errors` items in TS specify `solution:` but the formatter
//!      reads `err.fix` (which is `undefined`) producing `Fix: undefined`.
//!      We use the literal string "undefined" here.
//!
//! All three quirks are tested explicitly by
//! `cli_list_attachments::scenario_clap_exposes_list_attachments_with_flag_aware_help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "The ID of the work unit to list attachments for (e.g., AUTH-001, DASH-002)",
    required: true,
}];

const EXAMPLE_1_OUTPUT: &str = "Attachments for AUTH-001 (3):\n\n  ✓ spec/attachments/AUTH-001/auth-flow.png\n    Size: 125.45 KB\n    Modified: 1/15/2025, 10:30:00 AM\n\n  ✓ spec/attachments/AUTH-001/token-refresh.png\n    Size: 89.23 KB\n    Modified: 1/15/2025, 11:00:00 AM";

const EXAMPLE_3_OUTPUT: &str = "Attachments for AUTH-003 (2):\n\n  ✓ spec/attachments/AUTH-003/diagram.png\n    Size: 45.12 KB\n    Modified: 1/14/2025, 3:00:00 PM\n\n  ✗ spec/attachments/AUTH-003/deleted-file.pdf\n    File not found on filesystem";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-attachments AUTH-001",
        description: Some("List attachments for work unit"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec list-attachments AUTH-002",
        description: Some("No attachments"),
        output: Some("No attachments found for work unit AUTH-002"),
    },
    CommandExample {
        command: "fspec list-attachments AUTH-003",
        description: Some("Missing files detected"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        // TS quirk #3: TS `solution` is renamed to `fix` by formatter; it
        // doesn't exist on these objects, so output is "Fix: undefined".
        fix: "undefined",
    },
    CommonError {
        error: "No attachments found for work unit AUTH-001",
        fix: "undefined",
    },
];

const NOTES: &[&str] = &[
    "Attachments are stored in spec/attachments/<workUnitId>/",
    "File sizes are displayed in kilobytes (KB)",
    "Missing files are highlighted with ✗ marker",
    "Use 'fspec remove-attachment' to remove broken links",
];

// TS quirk #2: each entry already includes "fspec " prefix; the formatter
// prefixes again, producing "fspec fspec …".
const RELATED: &[&str] = &[
    "fspec add-attachment - Add attachment",
    "fspec remove-attachment - Remove attachment",
    "fspec show-work-unit - View work unit details (includes attachments)",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist (created with 'fspec create-story', 'fspec create-bug', or 'fspec create-task')",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-attachments",
    description: "List all attachments for a work unit. Shows file paths, sizes, and modification dates. Verifies whether files exist on the filesystem and highlights missing files.",
    usage: Some("fspec list-attachments <workUnitId>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to see what files are attached to a work unit, to verify attachments exist on filesystem, to check file sizes and modification dates, before adding duplicate attachments, or when reviewing work unit context.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    // TS quirk #1: comma-joined Array.toString()
    typical_workflow: Some(
        "View work unit details: fspec show-work-unit AUTH-001,List attachments: fspec list-attachments AUTH-001,Review attachment files in spec/attachments/AUTH-001/,Add more attachments if needed: fspec add-attachment AUTH-001 <file>",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
