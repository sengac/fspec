//! `add-diagram` help configuration — Rust port of
//! `src/commands/add-diagram-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-diagram --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/add-diagram.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "section",
        description: "Section name (e.g., \"Architecture\", \"Data Flow\") - used for organization",
        required: true,
    },
    CommandArgument {
        name: "title",
        description: "Diagram title (e.g., \"Command Flow\", \"System Architecture\")",
        required: true,
    },
    CommandArgument {
        name: "code",
        description: "Mermaid diagram code (syntax validated before adding)",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-diagram \"Architecture\" \"Command Flow\" \"graph TB\\n  CLI-->Parser\\n  Parser-->Validator\"",
        description: Some("Add flowchart diagram"),
        output: Some(
            "✓ Added diagram \"Command Flow\"\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md",
        ),
    },
    CommandExample {
        command: "fspec add-diagram \"Architecture\" \"System Overview\" \"graph LR\\n  User-->API\\n  API-->Database\"",
        description: Some("Add system architecture diagram"),
        output: Some(
            "✓ Added diagram \"System Overview\"\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md",
        ),
    },
    CommandExample {
        command: "fspec add-diagram \"Data Flow\" \"Authentication Flow\" \"sequenceDiagram\\n  User->>API: Login\\n  API->>DB: Verify\"",
        description: Some("Add sequence diagram"),
        output: Some(
            "✓ Added diagram \"Authentication Flow\"\n  Updated: spec/foundation.json\n  Regenerated: spec/FOUNDATION.md",
        ),
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Add Architecture Diagrams",
        example: "fspec add-diagram \"Architecture\" \"System Overview\" \"graph TB\\n  User-->API\\n  API-->Database\"\nfspec add-diagram \"Architecture\" \"Component Structure\" \"graph LR\\n  CLI-->Core\\n  Core-->Utils\"",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Update Existing Diagram",
        example: "# Adding a diagram with the same title replaces the existing one\nfspec add-diagram \"Architecture\" \"System Overview\" \"graph TB\\n  User-->Gateway\\n  Gateway-->Services\"",
        description: "undefined",
    }),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Invalid Mermaid syntax: ...",
        fix: "Validate your Mermaid code at https://mermaid.live or check Mermaid documentation",
    },
    CommonError {
        error: "Error: Diagram code cannot be empty",
        fix: "Provide Mermaid diagram code as the third argument",
    },
    CommonError {
        error: "Error: Updated foundation.json failed schema validation: ...",
        fix: "Ensure the diagram conforms to foundation schema requirements",
    },
];

const PREREQUISITES: &[&str] =
    &["spec/foundation.json exists (created by fspec init or discover-foundation)"];

const RELATED: &[&str] = &[
    "delete-diagram",
    "show-foundation",
    "generate-foundation-md",
];

const NOTES: &[&str] = &[
    "Mermaid syntax is validated using mermaid.parse() before adding",
    "Invalid syntax will be rejected with detailed error messages",
    "If a diagram with the same title exists, it will be replaced",
    "Use \\n for line breaks in diagram code when passing as string",
    "Supports all Mermaid diagram types: flowchart, sequence, class, state, etc.",
    "Diagrams are stored in foundation.json and rendered in FOUNDATION.md",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-diagram",
    description: "Add or update Mermaid diagram in foundation.json and regenerate FOUNDATION.md",
    usage: Some("fspec add-diagram <section> <title> <code>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to add architecture diagrams to foundation.json. The diagram code is validated against Mermaid syntax before being added. Useful for documenting system architecture, data flows, and component relationships.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Design diagram in Mermaid Live Editor → 2. fspec add-diagram <section> <title> <code> → 3. Verify: fspec show-foundation → 4. View in FOUNDATION.md",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
