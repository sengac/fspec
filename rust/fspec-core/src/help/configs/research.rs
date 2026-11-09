//! `research` help configuration.
//!
//! BUG-566: the TypeScript formatter printed `undefined` for keys this config
//! left unset; the config now holds the real text (each fix is the TS
//! `solution`, a pattern without a description prints none).
//!
//! CLI-015: AST code search is the `fspec astgrep` subcommand, not a research
//! tool.

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--tool <name>",
        description: "Research tool to select (perplexity, jira, confluence, stakeholder). Omit to list the tools. Running a tool is not ported yet.",
        default_value: None,
    },
    CommandOption {
        flag: "--work-unit <workUnitId>",
        description: "Work unit context for the tool",
        default_value: None,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[CommonPatternEntry::Structured(CommonPattern {
    pattern: "Check which research tools are configured",
    example: "fspec research",
    description: Some(
        "Lists each bundled tool with ✓ (configured) or ✗ and the environment variable it needs",
    ),
})];

const TYPICAL_WORKFLOW: &str = r#"During Example Mapping, identify a question that needs research. Add question: fspec add-question AUTH-001 "@human: Support OAuth?". Ask the stakeholder or check the tools listed by fspec research. Answer question: fspec answer-question AUTH-001 0 --answer "Yes" --add-to rule. Generate scenarios: fspec generate-scenarios AUTH-001"#;

const EX1_OUTPUT: &str = "Available Research Tools:\n\n  ✗ perplexity\n    Perplexity AI research tool for web search and AI-powered answers\n    Usage: fspec research --tool=perplexity <args>\n    Config: export PERPLEXITY_API_KEY=\"...\"";

const EX2_OUTPUT: &str = "Research tool execution is not yet ported to the Rust binary (tool 'stakeholder'); use the listing mode (omit --tool) for now";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec research",
        description: Some("List all available research tools"),
        output: Some(EX1_OUTPUT),
    },
    CommandExample {
        command: "fspec research --tool=stakeholder --work-unit=AUTH-001",
        description: Some("Select a tool (execution is not ported yet)"),
        output: Some(EX2_OUTPUT),
    },
];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "Error: Research tool not found: xyz",
    fix: "Run fspec research to list the available tools",
}];

const RELATED: &[&str] = &[
    "add-question",
    "answer-question",
    "add-attachment",
    "generate-scenarios",
];

const NOTES: &[&str] = &[
    "Bundled tools: perplexity, jira, confluence, stakeholder",
    "Each tool shows the environment variable that configures it",
    "Running a research tool is not ported to the Rust CLI yet",
    "Attachments are saved to spec/attachments/<workUnitId>/",
    "For AST code search and refactoring, use the `fspec astgrep` command",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "research",
    description: "List the research tools available during Example Mapping",
    usage: Some("fspec research [--tool=<name>] [--work-unit=<workUnitId>]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during the specifying phase to see which research tools are configured. For AST code search, use the `fspec astgrep` command instead."
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
