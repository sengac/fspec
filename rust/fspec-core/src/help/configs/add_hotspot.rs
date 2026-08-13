//! `add-hotspot` help configuration — Rust port of
//! `src/commands/add-hotspot-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-hotspot --help`
//! piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/add-hotspot.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "text",
        description: "Hotspot description (brief title)",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--concern <description>",
        description: "Risk, uncertainty, or problem description (detailed explanation)",
        default_value: None,
    },
    CommandOption {
        flag: "--timestamp <ms>",
        description: "Timeline position in milliseconds",
        default_value: None,
    },
    CommandOption {
        flag: "--bounded-context <name>",
        description: "Bounded context association",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command:
            "fspec add-hotspot AUTH-001 \"Password Reset Flow\" --concern \"Unclear timeout logic\"",
        description: Some("Add hotspot with concern description"),
        output: Some("✓ Added hotspot \"Password Reset Flow\" to AUTH-001 (ID: 0)"),
    },
    CommandExample {
        command:
            "fspec add-hotspot PAYMENT-001 \"Payment Gateway\" --concern \"Third-party API availability unclear\"",
        description: Some("Capture external dependency uncertainty"),
        output: Some("✓ Added hotspot \"Payment Gateway\" to PAYMENT-001 (ID: 0)"),
    },
];

const RELATED: &[&str] = &[
    "add-domain-event",
    "add-command",
    "add-policy",
    "show-event-storm",
    "generate-example-mapping-from-event-storm",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Work unit must have eventStorm section initialized",
    "Hotspot should identify a PROBLEM or UNCERTAINTY",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Hotspots represent QUESTIONS or RISKS to investigate"),
    CommonPatternEntry::Bullet("Use --concern to document detailed problem description"),
    CommonPatternEntry::Bullet(
        "Convert hotspots to questions via generate-example-mapping-from-event-storm",
    ),
    CommonPatternEntry::Bullet("Hotspots often become @human questions in Example Mapping"),
];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "Work unit not found",
    fix: "Ensure work unit exists: fspec show-work-unit <id>",
}];

const NOTES: &[&str] = &[
    "Hotspots highlight areas needing more discovery",
    "Concerns can be converted to questions for stakeholders",
    "Use show-event-storm to view all hotspots",
    "Hotspots assigned stable IDs starting from 0",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-hotspot",
    description:
        "Add hotspot to Event Storm section for capturing uncertainties, risks, or problems",
    usage: Some("fspec add-hotspot <workUnitId> <text> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Big Picture Event Storming when encountering areas of uncertainty, risk, or problems that need further investigation.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
