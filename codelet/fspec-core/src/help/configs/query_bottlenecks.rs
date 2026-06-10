//! `query-bottlenecks` help configuration — Rust port of
//! `src/commands/query-bottlenecks-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js query-bottlenecks --help`
//! when piped to non-TTY. Captured fixture:
//! `codelet/fspec/tests/fixtures/help/query-bottlenecks.txt`.

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const EX1_OUTPUT: &str = "Bottleneck Work Units (blocking 2+ work units):\n\nAUTH-001 (implementing) - Setup authentication infrastructure\n  Bottleneck Score: 5\n  Direct Blocks: AUTH-002, AUTH-003\n  Transitive Blocks: AUTH-004, AUTH-005, AUTH-006\n\nDB-001 (testing) - Database schema migration\n  Bottleneck Score: 3\n  Direct Blocks: DB-002, DB-003, DB-004\n\nTotal bottlenecks: 2";

const EX2_OUTPUT: &str = "{\n  \"bottlenecks\": [\n    {\n      \"id\": \"AUTH-001\",\n      \"title\": \"Setup authentication infrastructure\",\n      \"status\": \"implementing\",\n      \"score\": 5,\n      \"directBlocks\": [\"AUTH-002\", \"AUTH-003\"],\n      \"transitiveBlocks\": [\"AUTH-004\", \"AUTH-005\", \"AUTH-006\"]\n    }\n  ]\n}";

const EX3_OUTPUT: &str = "Bottleneck Work Units (blocking 2+ work units):\n\nAUTH-001 (implementing) - Setup authentication infrastructure\n  Bottleneck Score: 5\n  Direct Blocks: AUTH-002, AUTH-003\n  Transitive Blocks: AUTH-004, AUTH-005, AUTH-006";

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--output <format>",
    description: "Output format: text or json",
    default_value: Some("text"),
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec query-bottlenecks",
        description: Some("List bottleneck work units blocking 2+ work units"),
        output: Some(EX1_OUTPUT),
    },
    CommandExample {
        command: "fspec query-bottlenecks --output json",
        description: Some("Output bottlenecks as JSON for automation"),
        output: Some(EX2_OUTPUT),
    },
    CommandExample {
        command: "fspec query-bottlenecks | head -20",
        description: Some("Show only top bottlenecks (highest scores)"),
        output: Some(EX3_OUTPUT),
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Daily Prioritization",
        example: "# Morning standup: identify blockers\nfspec query-bottlenecks\n\n# Prioritize highest score bottleneck\nfspec update-work-unit-status AUTH-001 implementing\n\n# After completion\nfspec update-work-unit-status AUTH-001 done\n\n# Re-check for new bottlenecks\nfspec query-bottlenecks",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Automation with JSON Output",
        example: "# Export bottlenecks for dashboard\nfspec query-bottlenecks --output json > bottlenecks.json\n\n# Process with jq (highest score first)\njq '.bottlenecks | sort_by(-.score) | .[0]' bottlenecks.json",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Team Coordination",
        example: "# Identify parallelizable work\nfspec query-bottlenecks\n\n# If bottleneck score is high (5+), assign multiple team members:\n# - Member 1: Complete bottleneck AUTH-001\n# - Member 2-4: Work on unblocked parallel tasks\n\n# After bottleneck complete, previously blocked work can proceed",
        description: "undefined",
    }),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: No bottlenecks found",
        fix: "No work units are blocking 2+ other work units. This is good - no major blockers exist.",
    },
    CommonError {
        error: "Error: work-units.json not found",
        fix: "Run: fspec init to create work-units.json file",
    },
    CommonError {
        error: "Error: Invalid work-units.json format",
        fix: "Check for JSON syntax errors. Run: fspec validate-work-units (if available)",
    },
];

const RELATED: &[&str] = &[
    "query-orphans",
    "suggest-dependencies",
    "add-dependency",
    "update-work-unit-status",
    "show-work-unit",
];

const NOTES: &[&str] = &[
    "Bottleneck score = total work units blocked (direct + transitive)",
    "Only work units NOT in \"done\" or \"blocked\" status are considered bottlenecks",
    "Minimum bottleneck score is 2 (blocking at least 2 work units)",
    "Work units are ranked by score (highest to lowest)",
    "Transitive blocks = work units blocked indirectly through dependency chain",
    "Direct blocks = work units directly blocked by this work unit",
    "Completing a high-score bottleneck can unblock significant parallelizable work",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "query-bottlenecks",
    description:
        "Identify bottleneck work units blocking the most downstream work (2+ blocked work units)",
    usage: Some("fspec query-bottlenecks [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to identify critical path blockers that are preventing progress on multiple work units. Essential for prioritization decisions and unblocking parallelizable work. Run daily during active development to maximize throughput.",
    ),
    when_not_to_use: None,
    prerequisites: &["spec/work-units.json exists with work units and dependency relationships"],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Run bottleneck query: fspec query-bottlenecks → 2. Prioritize highest-score bottleneck → 3. Complete bottleneck work unit → 4. Update status: fspec update-work-unit-status <id> done → 5. Re-run query to identify next bottleneck",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
