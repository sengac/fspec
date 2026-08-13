//! `query-work-units` help configuration — Rust port of
//! `src/commands/query-work-units-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js query-work-units --help`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const EXAMPLE_1_OUTPUT: &str =
    "[{\"id\":\"AUTH-001\",\"status\":\"implementing\",\"title\":\"Login feature\"}]";

const EXAMPLE_2_OUTPUT: &str =
    "[{\"id\":\"CLI-001\",\"status\":\"done\",\"title\":\"CLI commands\"}]";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec query-work-units --status=implementing --format=json",
        description: Some("Query with filters"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec query-work-units --type=story --status=done --tag=@cli",
        description: Some("Query completed stories tagged with @cli"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--status <status>",
        description: "Filter by status",
        default_value: None,
    },
    CommandOption {
        flag: "--type <type>",
        description: "Filter by work unit type: story, task, or bug",
        default_value: None,
    },
    CommandOption {
        flag: "--tag <tag>",
        description: "Filter by tag (e.g., @cli, @high)",
        default_value: None,
    },
    CommandOption {
        flag: "--epic <epic>",
        description: "Filter by epic",
        default_value: None,
    },
    CommandOption {
        flag: "--prefix <prefix>",
        description: "Filter by prefix",
        default_value: None,
    },
    CommandOption {
        flag: "--format <format>",
        description: "Output format: table, json, csv",
        default_value: None,
    },
];

const RELATED: &[&str] = &[
    "list-work-units",
    "export-work-units",
    "search-scenarios",
    "compare-implementations",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "query-work-units",
    description: "Query work units with advanced filters and output formats",
    usage: Some("fspec query-work-units [options]"),
    arguments: &[],
    options: OPTIONS,
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
