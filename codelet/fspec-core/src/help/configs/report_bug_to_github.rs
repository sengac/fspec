//! `report-bug-to-github` help configuration — Rust port of
//! `src/commands/report-bug-to-github-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js report-bug-to-github --help`.
//!
//! ⚠️ PARITY QUIRKS (faithful to the TS output):
//!   * `typicalWorkflow` is a `string[]` in the TS config but `formatCommandHelp`
//!     interpolates it as `${config.typicalWorkflow}`, so JS comma-joins the
//!     array into a single line. `typical_workflow` here is that comma-joined
//!     string.
//!   * `commonErrors` entries use the field name `solution` in the TS config,
//!     but `formatCommandHelp` reads `err.fix`, rendering the literal
//!     `undefined`. Each `CommonError.fix` here is the literal "undefined".
//!   * `relatedCommands` strings already begin with `fspec ` in the TS config,
//!     and the formatter prefixes another `fspec `, producing `fspec fspec …`.
//!     Stored verbatim (with the leading `fspec `) to reproduce the double.
//!
//! Unlike `research-help.ts`, this config's `options` use the correct `flag`
//! field, so the OPTIONS block renders real flags (no `undefined`).

use super::super::{CommandExample, CommandHelpConfig, CommandOption, CommonError};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--project-root <path>",
        description: "Project root directory (default: auto-detected)",
        default_value: None,
    },
    CommandOption {
        flag: "--bug-description <text>",
        description: "Brief description of the bug",
        default_value: None,
    },
    CommandOption {
        flag: "--expected-behavior <text>",
        description: "What you expected to happen",
        default_value: None,
    },
    CommandOption {
        flag: "--actual-behavior <text>",
        description: "What actually happened",
        default_value: None,
    },
    CommandOption {
        flag: "--interactive",
        description: "Enable interactive mode with prompts for bug details",
        default_value: None,
    },
];

const TYPICAL_WORKFLOW: &str = "Encounter a bug while using fspec,Run: fspec report-bug-to-github,System automatically gathers context (version, platform, git status, work unit),Browser opens with pre-filled GitHub issue,Review and submit the issue";

const EX1_OUTPUT: &str = "Gathering system context...\n✓ fspec version: 0.6.0\n✓ Node version: v22.20.0\n✓ Platform: linux\n✓ Git branch: feature-branch\n✓ Work unit: CLI-014 - Report bug to GitHub\n\nOpening GitHub issue in browser...\n✓ Browser opened with pre-filled issue";

const EX2_OUTPUT: &str =
    "Gathering system context...\n✓ Generated bug report\n✓ Opening browser with pre-filled issue";

const EX3_OUTPUT: &str = "What command were you running? fspec validate\nWhat did you expect to happen? Successful validation\nWhat actually happened? Command crashed\n\nGenerating bug report...\n✓ Opening browser with pre-filled issue";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec report-bug-to-github",
        description: Some("Generate bug report with automatic context gathering and open in browser"),
        output: Some(EX1_OUTPUT),
    },
    CommandExample {
        command: r#"fspec report-bug-to-github --bug-description "Validation command crashes""#,
        description: Some("Report bug with description"),
        output: Some(EX2_OUTPUT),
    },
    CommandExample {
        command: "fspec report-bug-to-github --interactive",
        description: Some("Interactive bug reporting with prompts"),
        output: Some(EX3_OUTPUT),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Browser failed to open",
        fix: "undefined",
    },
    CommonError {
        error: "No work unit context found",
        fix: "undefined",
    },
];

const PREREQUISITES: &[&str] = &[
    "fspec must be installed (to gather version information)",
    "Git repository (optional, for git context)",
    "Work units file (optional, for work unit context)",
    "Browser installed (to open GitHub issue)",
];

const RELATED: &[&str] = &[
    "fspec validate - Validate feature files",
    "fspec check - Run comprehensive validation",
    "fspec show-work-unit - View work unit details",
];

const NOTES: &[&str] = &[
    "Automatically includes: fspec version, Node version, OS platform",
    "Git context: current branch, uncommitted changes status",
    "Work unit context: ID, title, status, related feature file",
    "Error logs: recent errors from .fspec/error-logs/",
    "All data is URL-encoded for GitHub issue form",
    "The command does NOT automatically submit the issue - you review first",
    "Special characters and code blocks are properly escaped",
    "Supports both interactive and non-interactive modes",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "report-bug-to-github",
    description: "Report bugs to GitHub with AI-assisted context gathering. Automatically collects system information, git status, work unit context, and recent error logs to create a pre-filled GitHub issue.",
    usage: Some("fspec report-bug-to-github [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when you encounter a bug in fspec and want to report it to the maintainers. This command gathers all relevant context automatically, making it easy to create comprehensive bug reports without manually collecting system information.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
