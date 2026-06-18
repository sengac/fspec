//! `research` help configuration — Rust port of `src/commands/research-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js research --help`.
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

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "undefined",
        description:
            "Research tool to execute (ast, perplexity, confluence, jira, stakeholder). Omit to list available tools.",
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
        example: r#"fspec research --tool=ast --pattern="function $NAME" --lang=typescript --path=src/"#,
        description: "Search for function declarations using AST pattern matching",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "undefined",
        example: r#"fspec research --tool=ast --pattern="async function $NAME" --lang=typescript --path=src/"#,
        description: "Search for async function declarations",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "undefined",
        example: r#"fspec research --tool=ast --pattern="class $NAME" --lang=typescript --path=src/"#,
        description: "Search for class or interface definitions",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "undefined",
        example: r#"fspec research --tool=ast --refactor --pattern="const MyComponent" --lang=tsx --source=src/big.tsx --target=src/MyComponent.tsx"#,
        description: "Extract a component or function to its own file",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "undefined",
        example: r#"fspec research --tool=stakeholder --platform=teams,slack --question="Need OAuth?" --work-unit=AUTH-001"#,
        description: "Send questions to Teams/Slack and attach responses",
    }),
];

const TYPICAL_WORKFLOW: &str = r#"During Example Mapping, identify a question that needs research,Add question: fspec add-question AUTH-001 "@human: Support OAuth?",Research the question: fspec research --tool=stakeholder --platform=teams --question="Support OAuth?" --work-unit=AUTH-001,When prompted, attach results to work unit (y),Review attached research in spec/attachments/AUTH-001/,Answer question based on research: fspec answer-question AUTH-001 0 --answer "Yes" --add-to rule,Generate scenarios: fspec generate-scenarios AUTH-001"#;

const EX1_OUTPUT: &str = "Available Research Tools:\n\n  ast\n    Usage: fspec research --tool=ast <args>\n\n  stakeholder\n    Usage: fspec research --tool=stakeholder <args>";

const EX2_OUTPUT: &str = "src/services/api.ts:15:1:async function fetchUser(id: string) {\nsrc/services/auth.ts:42:1:async function validateToken(token: string) {\nsrc/utils/cache.ts:8:1:async function getCacheValue(key: string) {";

const EX3_OUTPUT: &str = "src/auth.ts:12:1:function login(username: string, password: string) {\nsrc/auth.ts:28:1:function logout() {\nsrc/auth.ts:45:1:function validateCredentials(user: User) {";

const EX4_OUTPUT: &str = "# Stakeholder Question for AUTH-001\n\n**Platform:** teams\n**Question:** Should we support OAuth2?\n\n**Work Unit Context:**\n- ID: AUTH-001\n- Title: User Login\n- Epic: user-management\n\n**Rules:**\n1. Password must be 8+ chars\n\nMessage sent successfully.\n\nAttach research results to work unit? (y/n):";

const EX5_OUTPUT: &str = "{\n  \"success\": true,\n  \"movedCode\": \"const MyComponent = () => { ... }\",\n  \"sourceFile\": \"src/big-file.tsx\",\n  \"targetFile\": \"src/components/MyComponent.tsx\"\n}";

const EX6_OUTPUT: &str = "src/types/user.ts:5:1:interface User {\nsrc/types/user.ts:12:1:interface UserProfile {\nsrc/types/auth.ts:3:1:interface AuthToken {\nsrc/types/auth.ts:18:1:interface Session {";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec research",
        description: Some("List all available research tools"),
        output: Some(EX1_OUTPUT),
    },
    CommandExample {
        command: r#"fspec research --tool=ast --pattern="async function $NAME" --lang=typescript --path=src/"#,
        description: Some("Search codebase for async functions using AST analysis"),
        output: Some(EX2_OUTPUT),
    },
    CommandExample {
        command: r#"fspec research --tool=ast --pattern="function $NAME" --lang=typescript --path=src/auth.ts"#,
        description: Some("Find all functions in a specific file"),
        output: Some(EX3_OUTPUT),
    },
    CommandExample {
        command: r#"fspec research --tool=stakeholder --platform=teams --question="Should we support OAuth2?" --work-unit=AUTH-001"#,
        description: Some("Send question to stakeholders via Teams"),
        output: Some(EX4_OUTPUT),
    },
    CommandExample {
        command: r#"fspec research --tool=ast --refactor --pattern="const MyComponent" --lang=tsx --source=src/big-file.tsx --target=src/components/MyComponent.tsx"#,
        description: Some("Move a component to its own file using AST refactoring"),
        output: Some(EX5_OUTPUT),
    },
    CommandExample {
        command: r#"fspec research --tool=ast --pattern="interface $NAME" --lang=typescript --path=src/types/"#,
        description: Some("Find all TypeScript interfaces in a directory"),
        output: Some(EX6_OUTPUT),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Research tool not found: xyz",
        fix: "undefined",
    },
    CommonError {
        error: "Error: --pattern is required",
        fix: "undefined",
    },
    CommonError {
        error: "Error: --lang is required",
        fix: "undefined",
    },
    CommonError {
        error: "Pattern matched N nodes. Refactor requires exactly 1 match.",
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
        "Use during specifying phase when you have questions that require external research (AST analysis, stakeholder input, API queries, etc.). Research tools help answer red card questions during Example Mapping.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
