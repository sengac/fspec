//! `update-work-unit-estimate` help configuration — Rust port of
//! `src/commands/update-work-unit-estimate-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js update-work-unit-estimate --help`.
//!
//! NOTE: the TS `-help.ts` declares each common error with a `solution` key,
//! but the shared `formatCommandHelp` reads `error.fix` — which is therefore
//! `undefined` and renders the literal string `undefined` after `Fix: `. We
//! reproduce that quirk verbatim by setting `fix: "undefined"`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "id",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "points",
        description: "Story points (Fibonacci: 1, 2, 3, 5, 8, 13, 21)",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec update-work-unit-estimate AUTH-001 5",
        description: Some("Set estimate for story work unit (requires completed feature file)"),
        output: Some("✓ Work unit AUTH-001 estimate set to 5"),
    },
    CommandExample {
        command: "fspec update-work-unit-estimate TASK-001 3",
        description: Some("Set estimate for task work unit (no feature file required)"),
        output: Some("✓ Work unit TASK-001 estimate set to 3"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "ACDD requires feature file completion before estimation",
        fix: "undefined",
    },
    CommonError {
        error: "Feature file has prefill placeholders must be removed first",
        fix: "undefined",
    },
    CommonError {
        error: "Invalid estimate: 7. Must be one of: 1,2,3,5,8,13,21",
        fix: "undefined",
    },
];

const RELATED: &[&str] = &[
    "show-work-unit",
    "query-estimate-accuracy",
    "generate-scenarios",
    "set-user-story",
];

const NOTES: &[&str] = &[
    "Use Fibonacci sequence for estimates: 1, 2, 3, 5, 8, 13, 21",
    "Estimate AFTER generating scenarios from Example Mapping and completing feature file",
    "1=trivial, 3=small, 5=medium, 8=large, 13+=very large",
    "Story/Bug types: MUST have completed feature file with @WORK-UNIT-ID tag",
    "Task types: Can be estimated without feature files",
    "Prefill placeholders ([role], [action], etc.) block estimation",
    "IMPORTANT: Estimates > 13 points trigger a warning in show-work-unit recommending breakdown into smaller work units (1-13 points each)",
    "Large estimate warnings persist until estimate ≤ 13 or status = done (tasks exempt from warnings)",
];

const PREREQS: &[&str] = &[
    "For story/bug work units: Feature file must exist and be complete (no prefill placeholders)",
    "For task work units: No prerequisites (tasks do not require feature files)",
    "Work unit must be in specifying phase or later",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "update-work-unit-estimate",
    description: "Set story point estimate for a work unit using Fibonacci scale",
    usage: Some("fspec update-work-unit-estimate <id> <points>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use after generating scenarios from Example Mapping when you have enough information to estimate complexity. IMPORTANT: Story and Bug work units require a completed feature file before estimation. Task work units can be estimated at any stage."),
    when_not_to_use: None,
    prerequisites: PREREQS,
    common_patterns: &[],
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
