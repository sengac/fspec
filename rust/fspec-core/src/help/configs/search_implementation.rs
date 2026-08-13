//! `search-implementation` help configuration — Rust port of
//! `src/commands/search-implementation-help.ts` (RPC-296).
//!
//! Byte-for-byte parity with `node dist/index.js search-implementation --help`
//! piped to non-TTY. Reference fixture:
//! `rust/fspec/tests/fixtures/help/search-implementation.txt`.
//!
//! ## Quirk: literal `undefined` in COMMON PATTERNS
//! The TS help config feeds `{title, commands}` objects but the formatter
//! consumes `{pattern, example, description}`, so all three interpolate as
//! the literal text `undefined`. Reproduced verbatim for byte parity.

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonPattern, CommonPatternEntry,
};

const EXAMPLE_1_OUTPUT: &str = r#"Found loadConfig in 8 files:

src/commands/init.ts (line 45)
src/commands/validate.ts (line 23)
src/utils/config-manager.ts (line 12)
..."#;
const EXAMPLE_2_OUTPUT: &str = r#"loadConfig usage across work units:

src/commands/init.ts
  └─ INIT-001: Initialize fspec project

src/commands/validate.ts
  └─ CLI-003: Validate feature files

src/utils/config-manager.ts
  └─ CONFIG-001: Configuration utilities"#;
const EXAMPLE_3_OUTPUT: &str = r#"{
  "function": "readFile|writeFile",
  "files": [
    {
      "path": "src/commands/create-feature.ts",
      "lines": [23, 45],
      "workUnits": ["CLI-001"]
    }
  ]
}"#;

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--function <name>",
        description: "Function name to search for",
        default_value: None,
    },
    CommandOption {
        flag: "--show-work-units",
        description: "Display which work units use each file",
        default_value: None,
    },
    CommandOption {
        flag: "--json",
        description: "Output results in JSON format",
        default_value: None,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "undefined",
        example: "undefined",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "undefined",
        example: "undefined",
        description: "undefined",
    }),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec search-implementation --function=loadConfig",
        description: Some("Find all uses of loadConfig function"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec search-implementation --function=loadConfig --show-work-units",
        description: Some("Show which work units use the function"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec search-implementation --function=\"readFile|writeFile\" --json",
        description: Some("Search with pattern matching"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const RELATED: &[&str] = &[
    "compare-implementations",
    "search-scenarios",
    "show-coverage",
];

const NOTES: &[&str] = &[
    "Searches implementation files linked in coverage data",
    "Uses simple text search (not AST analysis)",
    "Supports regex patterns for advanced matching",
    "Helps identify opportunities to replace manual code with existing utilities",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "search-implementation",
    description: "Search implementation code for specific function usage across work units",
    usage: Some("fspec search-implementation --function=<name> [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Finding all places where a specific utility function is used,Impact analysis before refactoring a function,Identifying code reuse opportunities,Detecting manual implementations that could use existing utilities"),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
