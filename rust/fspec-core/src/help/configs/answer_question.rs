//! `answer-question` help configuration — Rust port of
//! `src/commands/answer-question-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js answer-question --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/answer-question.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "index",
        description: "Question index number (from show-work-unit)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "--answer <answer>",
        description: "The answer text",
        default_value: None,
    },
    CommandOption {
        flag: "--add-to <type>",
        description: "Add answer to: rule, assumption, or none (default: none)",
        default_value: None,
    },
    CommandOption {
        flag: "--add-to-rules",
        description: "Also add answer as a rule (deprecated: use --add-to rule)",
        default_value: None,
    },
    CommandOption {
        flag: "--add-to-assumptions",
        description: "Also add answer as an assumption (deprecated: use --add-to assumption)",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command:
            "fspec answer-question AUTH-001 0 --answer \"Yes, support Google OAuth\"",
        description: Some("Answer question"),
        output: Some(
            "✓ Answered question: \"Should we support OAuth?\"\n  Answer: \"Yes, support Google OAuth\"",
        ),
    },
    CommandExample {
        command:
            "fspec answer-question AUTH-001 0 --answer \"Email required\" --add-to-rules",
        description: Some("Answer and add as rule"),
        output: Some("✓ Answered question and added to rules"),
    },
];

const RELATED: &[&str] = &["add-question", "add-rule", "add-assumption"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "answer-question",
    description:
        "Answer a question from Example Mapping and optionally convert to rule or assumption",
    usage: Some("fspec answer-question <workUnitId> <index> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: None,
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
