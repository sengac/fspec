//! `list-foundation-sections` -- Rust port of
//! `src/commands/list-foundation-sections.ts` (RPC-246).
//!
//! Exposes every valid foundation section name with its JSON path and
//! constraint info. Weaker LLMs (and humans) use this as a discovery
//! mechanism when filling out the foundation draft.
//!
//! This command performs ZERO project root I/O: the section list is a
//! static `&'static [FoundationSectionSpec]` constant -- the single
//! source of truth that must stay in lock-step with the TS source.
//! `project_root` is accepted on the signature for dispatcher contract
//! parity (matching `list_hooks::run` shape) but is intentionally
//! ignored.
//!
//! Behaviour parity with TypeScript (`src/commands/list-foundation-sections.ts`):
//!
//! * `format = "json"` -> `serde_json::to_string_pretty` (2-space indent)
//!   over the 7-entry section array. Section objects emit fields in
//!   `name`, `jsonPath`, `constraint`, optional `examples`, `description`
//!   order (parity with the TS object-literal declaration order).
//! * `format = "text"` (default) -> byte-for-byte equivalent of
//!   `renderSectionsAsText`: header line, 57-character '=' separator,
//!   blank line, then per-section bullet `* <name>` with indented
//!   `path:`, `constraint:`, optional `examples:`, and `about:` rows
//!   followed by a blank line, ending with a 2-line footer note about
//!   capabilities and personas.
//! * The `examples` field is omitted from JSON output for every section
//!   EXCEPT `projectType` (parity with TS optional `examples?: string[]`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;

/// CLI arguments accepted by `list-foundation-sections`. The TS
/// Commander.js registration declares a single `--format <format>`
/// option, default `"text"`. The structured dispatcher path mirrors
/// this with `{format?: "text" | "json"}` and `#[serde(default)]`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListFoundationSectionsArgs {
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// One row in the canonical sections table. Field declaration order
/// matters: `serde_json::to_string_pretty` emits fields in declaration
/// order, and `name`, `jsonPath`, `constraint`, `examples`,
/// `description` matches the TS object-literal declaration order in
/// `src/commands/list-foundation-sections.ts`.
///
/// `examples` is `&'static [&'static str]` so the constant table needs
/// no heap allocations. We skip serialising it when empty, which
/// matches the TS optional `examples?: string[]` semantics: every
/// section EXCEPT `projectType` has zero examples and omits the field
/// from JSON output.
#[derive(Debug, Serialize)]
struct FoundationSectionSpec {
    /// Section name as passed to `fspec update-foundation <section>`.
    name: &'static str,
    /// Dotted JSON path inside the foundation document.
    #[serde(rename = "jsonPath")]
    json_path: &'static str,
    /// Human-readable constraint description.
    constraint: &'static str,
    /// Optional non-exhaustive examples of valid values. Omitted from
    /// JSON output (and the text row) when empty.
    #[serde(skip_serializing_if = "slice_is_empty")]
    examples: &'static [&'static str],
    /// Short explanation of what the field represents.
    description: &'static str,
}

/// Helper for `#[serde(skip_serializing_if = ...)]` -- serde requires a
/// named function path rather than an inline turbofish on the slice
/// type.
fn slice_is_empty(s: &&'static [&'static str]) -> bool {
    s.is_empty()
}

/// The canonical 7-section list. This is the single source of truth in
/// the Rust port -- when fields are added to `update-foundation`, the
/// matching entry MUST be added here AND in the TS source
/// (`src/commands/list-foundation-sections.ts:55-99`). Order, field
/// values, and the optional `examples` are byte-for-byte identical to
/// the TS constant.
const FOUNDATION_SECTIONS: &[FoundationSectionSpec] = &[
    FoundationSectionSpec {
        name: "projectName",
        json_path: "project.name",
        constraint: "freeform string",
        examples: &[],
        description: "Project name",
    },
    FoundationSectionSpec {
        name: "projectVision",
        json_path: "project.vision",
        constraint: "freeform string",
        examples: &[],
        description: "One-sentence elevator pitch",
    },
    FoundationSectionSpec {
        name: "projectType",
        json_path: "project.projectType",
        constraint: "freeform string (1-30 characters)",
        examples: &["cli-tool", "web-app", "saas-platform"],
        description: "Short descriptor of what kind of software this is",
    },
    FoundationSectionSpec {
        name: "problemTitle",
        json_path: "problemSpace.primaryProblem.title",
        constraint: "freeform string",
        examples: &[],
        description: "Short title of the primary problem the project solves",
    },
    FoundationSectionSpec {
        name: "problemDefinition",
        json_path: "problemSpace.primaryProblem.description",
        constraint: "freeform string",
        examples: &[],
        description: "Detailed description of the primary problem",
    },
    FoundationSectionSpec {
        name: "problemImpact",
        json_path: "problemSpace.primaryProblem.impact",
        constraint: "enum: high, medium, low",
        examples: &[],
        description: "How critical the problem is",
    },
    FoundationSectionSpec {
        name: "solutionOverview",
        json_path: "solutionSpace.overview",
        constraint: "freeform string",
        examples: &[],
        description: "High-level solution approach",
    },
];

/// Dispatcher entry point. Accepts `project_root` for parity with the
/// ported-command contract (matching `list_hooks::run`) but never reads
/// the filesystem -- the section list is a compile-time constant.
pub async fn run(args_json: &str, _project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListFoundationSectionsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-foundation-sections",
            reason: format!("failed to parse args: {e}"),
        })?;

    match args.format.as_deref() {
        Some("json") => serde_json::to_string_pretty(FOUNDATION_SECTIONS).map_err(|e| {
            FspecCoreError::InvalidArgs {
                command: "list-foundation-sections",
                reason: format!("failed to serialize result: {e}"),
            }
        }),
        // Default to text (also covers explicit "text" and any future
        // unknown format strings, mirroring the TS `format = 'text'`
        // default).
        _ => Ok(render_text(FOUNDATION_SECTIONS)),
    }
}

/// Byte-for-byte port of `renderSectionsAsText` from
/// `src/commands/list-foundation-sections.ts:106-131`.
///
/// Layout (per section, with blank line between sections and a 2-line
/// footer at the end):
///
/// ```text
/// Foundation Sections (update-foundation field reference)
/// =========================================================
///
/// * <name>
///     path:       <jsonPath>
///     constraint: <constraint>
///     examples:   <comma-joined>          (only when non-empty)
///     about:      <description>
///
/// Note: capabilities and personas are managed via dedicated commands
///       (add-capability, add-persona) and cannot be updated via update-foundation.
/// ```
///
/// The bullet character is the Unicode BULLET (U+2022); the separator
/// is exactly 57 '=' characters (matching the TS string literal).
fn render_text(sections: &[FoundationSectionSpec]) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("Foundation Sections (update-foundation field reference)".to_string());
    lines.push("=========================================================".to_string());
    lines.push(String::new());

    for section in sections {
        lines.push(format!("\u{2022} {}", section.name));
        lines.push(format!("    path:       {}", section.json_path));
        lines.push(format!("    constraint: {}", section.constraint));
        if !section.examples.is_empty() {
            lines.push(format!("    examples:   {}", section.examples.join(", ")));
        }
        lines.push(format!("    about:      {}", section.description));
        lines.push(String::new());
    }

    lines.push("Note: capabilities and personas are managed via dedicated commands".to_string());
    lines.push(
        "      (add-capability, add-persona) and cannot be updated via update-foundation."
            .to_string(),
    );

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_with_defaults() {
        let a: ListFoundationSectionsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_format_json() {
        let a: ListFoundationSectionsArgs =
            serde_json::from_str(r#"{"format":"json"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn constant_has_seven_sections_in_canonical_order() {
        assert_eq!(FOUNDATION_SECTIONS.len(), 7);
        let expected = [
            "projectName",
            "projectVision",
            "projectType",
            "problemTitle",
            "problemDefinition",
            "problemImpact",
            "solutionOverview",
        ];
        for (i, name) in expected.iter().enumerate() {
            assert_eq!(FOUNDATION_SECTIONS[i].name, *name);
        }
    }

    #[test]
    fn only_project_type_carries_examples() {
        for s in FOUNDATION_SECTIONS {
            if s.name == "projectType" {
                assert_eq!(s.examples, &["cli-tool", "web-app", "saas-platform"]);
            } else {
                assert!(
                    s.examples.is_empty(),
                    "section {} unexpectedly has examples",
                    s.name
                );
            }
        }
    }

    #[test]
    fn render_text_starts_with_header_and_separator() {
        let out = render_text(FOUNDATION_SECTIONS);
        assert!(out.starts_with("Foundation Sections (update-foundation field reference)\n"));
        assert!(out.contains("\n=========================================================\n"));
    }

    #[test]
    fn render_text_includes_footer_note() {
        let out = render_text(FOUNDATION_SECTIONS);
        assert!(out.contains("Note: capabilities and personas are managed via dedicated commands"));
        assert!(out.contains(
            "      (add-capability, add-persona) and cannot be updated via update-foundation."
        ));
    }

    #[test]
    fn render_text_emits_examples_only_for_project_type() {
        let out = render_text(FOUNDATION_SECTIONS);
        let count = out.lines().filter(|l| l.starts_with("    examples:")).count();
        assert_eq!(count, 1, "expected exactly one examples row");
        assert!(out.contains("    examples:   cli-tool, web-app, saas-platform"));
    }
}
