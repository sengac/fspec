//! `research` help configuration — Rust port of `src/commands/research-help.ts`.
//!
//! ⚠️ PARITY QUIRKS (faithful to the TS output): the TS `research-help.ts`
//! config object uses field names (`name`, `title`/`description`/`example`,
//! `cause`/`solution`, `typicalWorkflow: string[]`) that DO NOT match the
//! fields `formatCommandHelp` reads (`flag`, `pattern`, `fix`, a single
//! `typicalWorkflow` string). The TS formatter therefore renders literal
//! `undefined` for every option flag, every common-pattern title, and every
//! common-error fix, and comma-joins the workflow array. The captured fixture
//! bakes in those quirks, so this Rust CONFIG reproduces them verbatim:
//!   * every `CommandOption.flag` is the literal "undefined"
//!   * every structured `CommonPattern.pattern` is the literal "undefined"
//!   * every `CommonError.fix` is the literal "undefined"
//!   * `typical_workflow` is the single comma-joined string
//!
//! CLI-015: the bundled `ast` tool is no longer a research tool — AST code
//! search moved to the native AstGrep tool (harness mode) and the
//! `fspec astgrep` subcommand (CLI mode). This help page documents the four
//! remaining bundled tools (perplexity, jira, confluence, stakeholder).

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "undefined",
        description:
            "Research tool to execute (perplexity, jira, confluence, stakeholder). Omit to list available tools.",
        default_value: None,
    },
    CommandOption {
        flag: "undefined",
        description:
            "Work unit context (forwarded to tool). Enables attaching results as attachments.",
        default_value: None,
    },
    CommandOption {
        flag: "undefined",
        description:
            "Additional arguments passed directly to the research tool. See tool --help for details.",
        default_value: None,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "undefined",
        example: r#"fspec research --tool=stakeholder --platform=teams,slack --question="Need OAuth?" --work-unit=AUTH-001"#,
        description: "Send questions to Teams/Slack and attach responses",
    }),
];

const TYPICAL_WORKFLOW: &str = r#"During Example Mapping, identify a question that needs research,Add question: fspec add-question AUTH-001 "@human: Support OAuth?",Research the question: fspec research --tool=stakeholder --platform=teams --question="Support OAuth?" --work-unit=AUTH-001,When prompted, attach results to work unit (y),Review attached research in spec/attachments/AUTH-001/,Answer question based on research: fspec answer-question AUTH-001 0 --answer "Yes" --add-to rule,Generate scenarios: fspec generate-scenarios AUTH-001"#;

const EX1_OUTPUT: &str = "Available Research Tools:\n\n  perplexity\n    Usage: fspec research --tool=perplexity <args>\n\n  stakeholder\n    Usage: fspec research --tool=stakeholder <args>";

const EX2_OUTPUT: &str = "# Stakeholder Question for AUTH-001\n\n**Platform:** teams\n**Question:** Should we support OAuth2?\n\n**Work Unit Context:**\n- ID: AUTH-001\n- Title: User Login\n- Epic: user-management\n\n**Rules:**\n1. Password must be 8+ chars\n\nMessage sent successfully.\n\nAttach research results to work unit? (y/n):";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec research",
        description: Some("List all available research tools"),
        output: Some(EX1_OUTPUT),
    },
    CommandExample {
        command: r#"fspec research --tool=stakeholder --platform=teams --question="Should we support OAuth2?" --work-unit=AUTH-001"#,
        description: Some("Send question to stakeholders via Teams"),
        output: Some(EX2_OUTPUT),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Research tool not found: xyz",
        fix: "undefined",
    },
];

const RELATED: &[&str] = &[
    "add-question",
    "answer-question",
    "add-attachment",
    "generate-scenarios",
];

const NOTES: &[&str] = &[
    "Research tools are auto-discovered from spec/research-scripts/ directory",
    "Any executable file in that directory becomes a research tool",
    "Tools can be written in any language (shell, Node.js, Python, etc.)",
    "Use <tool> --help to see tool-specific options",
    "Results can be attached to work units during discovery",
    "Attachments are saved to spec/attachments/<work-unit-id>/",
    "For AST code search and refactoring, use the `fspec astgrep` command",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "research",
    description: "Execute research tools to answer questions during Example Mapping",
    usage: Some("fspec research [--tool=<name>] [tool-specific-args...]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during specifying phase when you have questions that require external research (stakeholder input, API queries, web answers, etc.). Research tools help answer red card questions during Example Mapping. For AST code search, use the `fspec astgrep` command instead."
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
