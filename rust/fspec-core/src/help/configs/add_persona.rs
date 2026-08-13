//! `add-persona` help configuration — Rust port of the TS
//! `add-persona` Commander.js `--help` action.
//!
//! Byte-for-byte parity with `node dist/index.js add-persona --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/add-persona.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "name",
        description:
            "Persona name (e.g., \"Developer\", \"End User\", \"Administrator\"). Should represent a distinct user type.",
        required: true,
    },
    CommandArgument {
        name: "description",
        description:
            "Description of the persona: who they are, their characteristics, and how they interact with the system.",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--goal <goal>",
    description:
        "Primary goal for this persona (can be specified multiple times for multiple goals)",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-persona \"Developer\" \"Software developer using fspec for project management\" --goal \"Track work efficiently\" --goal \"Generate documentation\"",
        description: Some("Add persona with multiple goals"),
        output: Some(
            "✓ Added persona to foundation.json.draft\n  Name: Developer\n  Description: Software developer using fspec for project management\n  Goals: Track work efficiently, Generate documentation",
        ),
    },
    CommandExample {
        command: "fspec add-persona \"End User\" \"Person using the application features\" --goal \"Complete tasks quickly\"",
        description: Some("Add persona with single goal"),
        output: Some(
            "✓ Added persona to foundation.json.draft\n  Name: End User\n  Description: Person using the application features\n  Goals: Complete tasks quickly",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "foundation.json not found",
        fix: "undefined",
    },
    CommonError {
        error: "Persona already exists",
        fix: "undefined",
    },
];

const PREREQUISITES: &[&str] = &[
    "foundation.json or foundation.json.draft must exist",
    "Run fspec discover-foundation to create foundation.json.draft if needed",
];

const RELATED: &[&str] = &[
    "fspec discover-foundation - Start foundation discovery process",
    "fspec remove-persona - Remove persona from foundation",
    "fspec add-capability - Add capability to foundation",
    "fspec update-foundation - Update foundation fields",
];

const NOTES: &[&str] = &[
    "Draft file (foundation.json.draft) takes precedence over foundation.json",
    "Personas describe WHO uses the system (problem space)",
    "Goals should describe what the persona wants to achieve",
    "Personas help inform Example Mapping and acceptance criteria",
    "Multiple goals can be specified using multiple --goal flags",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-persona",
    description: "Add a user persona to foundation.json or foundation.json.draft. Personas describe WHO uses the system, their characteristics, and their goals. Used during foundation discovery to define the problem space.",
    usage: Some("fspec add-persona \"<name>\" \"<description>\" [options]"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during fspec discover-foundation workflow when adding personas to foundation.json.draft, or when adding new personas to an existing foundation.json after initial discovery.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "Start foundation discovery: fspec discover-foundation,AI analyzes codebase and identifies user types,Add each persona: fspec add-persona \"name\" \"description\" --goal \"goal1\" --goal \"goal2\",Repeat for all personas,Finalize foundation: fspec discover-foundation --finalize",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
