//! `bootstrap` help configuration — Rust port of
//! `src/commands/bootstrap-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js bootstrap --help`.

use super::super::{CommandExample, CommandHelpConfig};

const WHEN_TO_USE: &str = "Use this command at the start of EVERY fspec session to load complete workflow documentation into AI agent context. Required before using any fspec commands. This is typically called automatically by the /fspec slash command in Claude Code.";

const PREREQUISITES: &[&str] = &[
    "fspec must be installed and accessible",
    "Run from project root directory",
    "Foundation file (spec/foundation.json) should exist (will prompt to create if missing)",
    "Tool configuration recommended (fspec configure-tools)",
];

const EXAMPLE_OUTPUT: &str = "# fspec Command - Kanban-Based Project Management\n\n[Complete workflow documentation output...]\n\nStep 1: Load fspec Context\n...";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec bootstrap",
    description: Some("Load complete fspec documentation"),
    output: Some(EXAMPLE_OUTPUT),
}];

const TYPICAL_WORKFLOW: &str = "1. Start session: fspec bootstrap\n2. Load context into AI agent memory\n3. Begin work: fspec board (view Kanban)\n4. Follow ACDD workflow: discovery → specifying → testing → implementing → validating → done";

const RELATED: &[&str] = &["configure-tools", "discover-foundation", "board", "help"];

const NOTES: &[&str] = &[
    "Outputs comprehensive documentation by calling: getSpecsHelpContent(), getWorkHelpContent(), getDiscoveryHelpContent(), getMetricsHelpContent(), getSetupHelpContent(), getHooksHelpContent()",
    "Typically invoked by /fspec slash command automatically",
    "Output is designed for AI agent context loading, not human reading",
    "Includes ACDD workflow, Example Mapping guidance, and all command documentation",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "bootstrap",
    description: "Load complete fspec documentation and workflow guidance for AI agents. Internally calls all help section functions to provide comprehensive context.",
    usage: Some("fspec bootstrap [options]"),
    arguments: &[],
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(WHEN_TO_USE),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: &[],
    notes: NOTES,
};
