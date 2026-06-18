//! `discover-foundation` help configuration — Rust port of
//! `src/commands/discover-foundation-help.ts` (RPC-226).
//!
//! Byte-for-byte parity with `node dist/index.js discover-foundation --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/discover-foundation.txt`.
//!
//! NOTE: the TS help config also declares a `workflow` array, but
//! `formatCommandHelp` only renders `typicalWorkflow` (a string) — the
//! `workflow` array is NOT rendered. We therefore omit `typical_workflow`
//! entirely so the rendered help matches the TS output (no TYPICAL WORKFLOW
//! section).

use super::super::{CommandExample, CommandHelpConfig, CommandOption, CommonPatternEntry};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--output <path>",
        description: "Output path for final foundation.json (default: spec/foundation.json)",
        default_value: None,
    },
    CommandOption {
        flag: "--finalize",
        description: "Finalize foundation.json from edited draft file",
        default_value: None,
    },
    CommandOption {
        flag: "--draft-path <path>",
        description: "Path to draft file (default: spec/foundation.json.draft)",
        default_value: None,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet(
        "CLI tools: AI analyzes CLI framework usage, command definitions, identifies command-line user persona",
    ),
    CommonPatternEntry::Bullet(
        "Web apps: AI analyzes routes, UI components, identifies End User and API Consumer personas",
    ),
    CommonPatternEntry::Bullet(
        "Libraries: AI analyzes exports and API, identifies Developer persona",
    ),
];

const EXAMPLE_1_OUTPUT: &str = "✓ Generated spec/foundation.json.draft\n\nDraft created. To complete foundation, you must ULTRATHINK the entire codebase.\n\nAnalyze EVERYTHING: code structure, entry points, user interactions, documentation.\nUnderstand HOW it works, then determine WHY it exists and WHAT users can do.\n\nI will guide you field-by-field.\n\n<system-reminder>\nField 1/8: project.name\n\nAnalyze project configuration to determine project name. Confirm with human.\n\nRun: fspec update-foundation projectName \"<name>\"\n</system-reminder>";

const EXAMPLE_2_OUTPUT: &str = "✓ Updated \"projectName\" in foundation.json.draft\n  Updated: spec/foundation.json.draft\n\n<system-reminder>\nField 2/8: project.vision (elevator pitch)\n\nULTRATHINK: Read ALL code, understand the system deeply. What is the core PURPOSE?\nFocus on WHY this exists, not HOW it works.\n\nAsk human to confirm vision.\n\nRun: fspec update-foundation projectVision \"your vision\"\n</system-reminder>";

const EXAMPLE_3_OUTPUT: &str = "✓ Generated spec/foundation.json\n✓ Foundation discovered and validated successfully\n\nDiscovery complete!\n\nCreated: spec/foundation.json, spec/FOUNDATION.md\n\nFoundation is ready.";

const EXAMPLE_4_OUTPUT: &str = "✗ Foundation validation failed\n\nSchema validation failed. Missing required: solutionSpace.capabilities, personas[0].name\n\nFix by running appropriate commands:\n  - For simple fields: fspec update-foundation <section> \"<value>\"\n  - For capabilities: fspec add-capability \"<name>\" \"<description>\"\n  - For personas: fspec add-persona \"<name>\" \"<description>\" --goal \"<goal>\"\n\nThen re-run: fspec discover-foundation --finalize";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec discover-foundation",
        description: Some("Create draft and receive guidance for first field"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec update-foundation projectName \"fspec\"",
        description: Some("Fill first field, command chains to next field"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec discover-foundation --finalize",
        description: Some("Validate complete draft and create final foundation.json"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
    CommandExample {
        command: "fspec discover-foundation --finalize",
        description: Some("Finalize with incomplete draft shows detailed validation errors"),
        output: Some(EXAMPLE_4_OUTPUT),
    },
];

const RELATED: &[&str] = &["update-foundation", "show-foundation", "generate-foundation-md"];

const NOTES: &[&str] = &[
    "AI-driven feedback loop: command guides AI field-by-field with system-reminders",
    "After each fspec update-foundation, command automatically chains to next unfilled field",
    "Draft file contains [DETECTED: value] for code-inferred values requiring verification",
    "Draft file contains [QUESTION: text] placeholders for AI to fill via commands",
    "AI must NEVER manually edit draft - system detects manual edits and reverts changes",
    "Focuses on ULTRATHINK (deep analysis), WHY (problems), WHAT (capabilities), not HOW",
    "System-reminders are invisible to user but visible to AI for guidance",
    "Generated foundation.json uses v2.0.0 generic schema",
    "Finalization validates draft against schema and auto-generates FOUNDATION.md",
    "Draft file is automatically deleted after successful finalization",
    "Validation errors show detailed feedback with missing fields and fix commands",
    "Error messages list exact field paths (e.g., \"Missing required: personas[0].name\")",
    "Includes command examples for each error type (update-foundation, add-capability, add-persona)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "discover-foundation",
    description:
        "Interactive AI-guided workflow to discover project foundation field-by-field with system-reminders",
    usage: Some("fspec discover-foundation [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when starting a new project with fspec to bootstrap foundation.json. Creates draft with placeholders, then guides AI field-by-field using system-reminders and chaining to next field after each fspec update-foundation command.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
