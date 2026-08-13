//! `add-question` help configuration — Rust port of the TS Commander.js
//! help output captured via `node dist/index.js add-question --help`.
//!
//! Captured fixture: `rust/fspec/tests/fixtures/help/add-question.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID (e.g., AUTH-001)",
        required: true,
    },
    CommandArgument {
        name: "question",
        description: "The question text (use @human: prefix for human-directed questions)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        description: Some("Add question during Example Mapping"),
        command: "fspec add-question AUTH-001 \"@human: Should we support OAuth providers?\"",
        output: Some("✓ Question added successfully"),
    },
    CommandExample {
        description: Some("Answer the question (index 0)"),
        command: "fspec answer-question AUTH-001 0 --answer \"Yes, support Google and GitHub OAuth\"",
        output: Some(
            "✓ Answered question: \"@human: Should we support OAuth providers?\"\n  Answer: \"Yes, support Google and GitHub OAuth\"",
        ),
    },
];

use super::super::CommonPatternEntry;

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet(
        "Use @human: prefix for questions directed at the human (e.g., \"@human: Should we support OAuth?\")",
    ),
    CommonPatternEntry::Bullet(
        "Ask one focused question at a time rather than multiple questions in one",
    ),
    CommonPatternEntry::Bullet("Frame questions to elicit concrete examples or rules"),
    CommonPatternEntry::Bullet("Questions should be answered before moving to testing phase"),
];

const RELATED: &[&str] = &[
    "answer-question",
    "add-rule",
    "add-example",
    "generate-scenarios",
    "show-work-unit",
];

const NOTES: &[&str] = &[
    "Questions must be answered before transitioning from specifying to testing",
    "Use show-work-unit to see all questions and their status",
    "Questions capture uncertainties that become rules or assumptions once answered",
];

const WHEN_TO_USE: &str =
    "Use during Example Mapping in the specifying phase when you encounter uncertainties, ambiguities, or need clarification from the human about requirements, rules, or expected behavior.";

const TYPICAL_WORKFLOW: &str =
    "1. Add question during discovery\n  2. Human answers question\n  3. Convert answer to rule or assumption\n  4. Continue until no questions remain\n  5. Move to testing phase";

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-question",
    description: "Add a question to a work unit during Example Mapping discovery phase",
    usage: Some("fspec add-question <workUnitId> <question>"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(WHEN_TO_USE),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: &[],
    notes: NOTES,
};
