//! `configure-tools` help configuration — Rust port of
//! `src/commands/configure-tools-help.ts` (RPC-208).
//!
//! Byte-for-byte parity with `node dist/index.js configure-tools --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/configure-tools.txt`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption, CommonError};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--test-command <command>",
        description:
            "Test command to run (e.g., \"<test-command>\", \"pytest\", \"cargo test\", \"go test ./...\")",
        default_value: None,
    },
    CommandOption {
        flag: "--quality-commands <commands...>",
        description:
            "Quality check commands to run (e.g., \"<quality-check-commands>\", \"black --check .\" \"mypy .\", \"cargo clippy\" \"cargo fmt --check\")",
        default_value: None,
    },
    CommandOption {
        flag: "--reconfigure",
        description: "Re-detect tools and update existing configuration",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command:
            "fspec configure-tools --test-command \"<test-command>\" --quality-commands \"<quality-check-commands>\"",
        description: Some("Configure tools for your platform"),
        output: Some(
            "✓ Test command configured: <test-command>\n✓ Quality check commands configured: <quality-check-commands>\n✓ Configuration saved to spec/fspec-config.json",
        ),
    },
    CommandExample {
        command:
            "fspec configure-tools --test-command \"pytest\" --quality-commands \"black --check .\" \"mypy .\"",
        description: Some("Configure for Python project with pytest"),
        output: Some(
            "✓ Test command configured: pytest\n✓ Quality check commands configured: black --check ., mypy .\n✓ Configuration saved to spec/fspec-config.json",
        ),
    },
    CommandExample {
        command:
            "fspec configure-tools --test-command \"cargo test\" --quality-commands \"cargo clippy\" \"cargo fmt --check\"",
        description: Some("Configure for Rust project with cargo"),
        output: Some(
            "✓ Test command configured: cargo test\n✓ Quality check commands configured: cargo clippy, cargo fmt --check\n✓ Configuration saved to spec/fspec-config.json",
        ),
    },
    CommandExample {
        command:
            "fspec configure-tools --test-command \"go test ./...\" --quality-commands \"go fmt ./...\" \"go vet ./...\"",
        description: Some("Configure for Go project"),
        output: Some(
            "✓ Test command configured: go test ./...\n✓ Quality check commands configured: go fmt ./..., go vet ./...\n✓ Configuration saved to spec/fspec-config.json",
        ),
    },
    CommandExample {
        command: "fspec configure-tools --reconfigure",
        description: Some("Re-detect and update tool configuration"),
        output: Some(
            "Detecting project type...\nFound: Node.js (package.json)\nRecommended test command: <test-command>\nRecommended quality commands: <quality-check-commands>\n[Interactive prompts to confirm...]",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "NO TEST COMMAND CONFIGURED",
        fix: "Run: fspec configure-tools --test-command \"your-test-command\"",
    },
    CommonError {
        error: "No quality check commands configured",
        fix: "Run: fspec configure-tools --quality-commands \"lint-cmd\" \"format-cmd\"",
    },
];

const PREREQUISITES: &[&str] = &[
    "Identify your testing framework (<test-command>, pytest, cargo test, go test, etc.)",
    "Identify your quality check commands (linting, formatting, type checking)",
    "Run from project root directory",
];

const RELATED: &[&str] = &["bootstrap", "validate", "check", "board"];

const NOTES: &[&str] = &[
    "Configuration persists in spec/fspec-config.json",
    "Commands are run during validating phase of ACDD workflow",
    "Platform-agnostic: works with any language/framework",
    "System reminders guide you if tools are not configured",
    "Use --reconfigure to update existing configuration",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "configure-tools",
    description:
        "Configure test and quality check commands for platform-agnostic workflow. Required for ACDD validation phase.",
    usage: Some("fspec configure-tools [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command during first-time setup or when changing test/quality tools. fspec is platform-agnostic and does NOT hardcode tool commands. You MUST configure tools for your specific platform (Node.js, Python, Rust, Go, etc.).",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "1. Detect framework: Check package.json, Cargo.toml, go.mod, etc.\n2. Configure tools: fspec configure-tools --test-command <cmd> --quality-commands <cmds>\n3. Verify: cat spec/fspec-config.json\n4. Use in ACDD: Tools are used automatically during validating phase",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
