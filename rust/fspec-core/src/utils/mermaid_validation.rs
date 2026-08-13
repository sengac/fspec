//! Shared Mermaid syntax validation backed by the `merman` Rust-native Mermaid
//! parser ([`merman_core`], parity target mermaid@11.15.0).
//!
//! This module replaces the original "Framing A" divergence — a pure-regex
//! pre-check that stood in for the TypeScript `validateMermaidSyntax`
//! (`mermaid.parse()` / `mermaid.render()` inside jsdom). It is consumed by:
//!
//! * `commands::add_diagram` (RPC-178) — validates user-supplied diagram code.
//! * `generators::foundation_md_util::validate_mermaid` (RPC-233) — validates
//!   every diagram the FOUNDATION.md generator emits.
//! * `commands::add_attachment` (RPC-170) — validates `.mmd` / `.mermaid` /
//!   `.md` attachment sources.
//!
//! ## Strategy
//!
//! 1. **Pure-string pre-checks** (`precheck_subgraphs`) run first and preserve
//!    the canonical TypeScript error messages for the two patterns the TS
//!    `validateMermaidSyntax` rejects *before* invoking mermaid — quoted
//!    subgraph titles (`subgraph "Title"`) and subgraph identifiers outside
//!    `[A-Za-z0-9_-]+`. merman itself *accepts* quoted subgraph titles, so
//!    retaining the pre-check is what keeps fspec's rejection + message stable.
//! 2. **merman parse** (`Engine::parse_diagram_sync` with
//!    [`ParseOptions::strict`]) is the comprehensive syntax gate. `Ok(Some)`
//!    is valid; `Ok(None)` (no diagram type detected) and `Err` both fail.

use std::path::Path;

use merman_core::{Engine, ParseOptions};
use regex::Regex;

/// Validate a single Mermaid diagram string.
///
/// Returns `Ok(())` when the diagram is syntactically valid, or `Err(message)`
/// describing the first failure. The message for the two pre-check patterns is
/// byte-stable (TS parity); merman's parse-error wording is not guaranteed
/// stable across merman versions, so callers should assert on substrings.
pub fn validate_mermaid_syntax(code: &str) -> Result<(), String> {
    // Pre-checks first so their canonical messages win.
    precheck_subgraphs(code)?;

    // Real syntax validation via the merman parser.
    let engine = Engine::new();
    match engine.parse_diagram_sync(code, ParseOptions::strict()) {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err("No diagram type detected".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// Pure-string pre-checks mirroring the portion of
/// `src/utils/mermaid-validation.ts` that runs before `mermaid.parse()`.
fn precheck_subgraphs(code: &str) -> Result<(), String> {
    // Pass 1: quoted subgraph titles (`/subgraph\s+"[^"]+"/`) — global, wins
    // over any identifier error (matches TS ordering).
    for body in subgraph_bodies(code) {
        if let Some(stripped) = body.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                if end >= 1 {
                    return Err(
                        "Quoted subgraph titles are not supported. Use: subgraph ID[Title]"
                            .to_string(),
                    );
                }
            }
        }
    }

    // Pass 2: subgraph identifiers (`/^[a-zA-Z0-9_-]+$/`).
    for body in subgraph_bodies(code) {
        let id_end = body
            .find(|c: char| c.is_whitespace() || c == '[')
            .unwrap_or(body.len());
        let id = &body[..id_end];
        if !id.is_empty() && !is_valid_subgraph_id(id) {
            return Err(format!(
                "Invalid subgraph identifier '{id}'. Use only letters, numbers, underscores, and hyphens"
            ));
        }
    }

    Ok(())
}

/// Yield the trimmed body following each `subgraph` keyword that is followed by
/// at least one whitespace char (mirrors the `subgraph\s+` regex prefix).
fn subgraph_bodies(code: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = code[search_from..].find("subgraph") {
        let after = search_from + rel + "subgraph".len();
        let rest = &code[after..];
        let trimmed = rest.trim_start();
        // Require `\s+` after the keyword.
        if trimmed.len() < rest.len() && !trimmed.is_empty() {
            out.push(trimmed);
        }
        search_from = after;
    }
    out
}

/// `/^[a-zA-Z0-9_-]+$/` — letters, digits, underscore, hyphen.
fn is_valid_subgraph_id(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Returns `true` when the file extension is one fspec treats as Mermaid:
/// `.mmd`, `.mermaid`, or `.md` (mirrors `shouldValidateMermaid`).
pub fn should_validate_mermaid(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => {
            let ext = ext.to_ascii_lowercase();
            ext == "mmd" || ext == "mermaid" || ext == "md"
        }
        None => false,
    }
}

/// Extract Mermaid code blocks (` ```mermaid ` fences) from markdown content.
/// Mirrors `extractMermaidFromMarkdown` (regex `/```mermaid\n([\s\S]*?)\n```/g`).
pub fn extract_mermaid_from_markdown(content: &str) -> Vec<String> {
    // Build once per call; the pattern is a compile-time constant so this never
    // fails, but we avoid `unwrap`/`expect` to satisfy the workspace lints.
    let re = match Regex::new(r"(?s)```mermaid\n(.*?)\n```") {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };
    re.captures_iter(content)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// Validate Mermaid content for an attachment, dispatching on extension.
///
/// * `.md` → extract every ` ```mermaid ` fence and validate each; a file with
///   zero fences is valid (it is just markdown).
/// * `.mmd` / `.mermaid` → validate the whole file as a single diagram.
/// * anything else → valid (not a Mermaid file).
pub fn validate_mermaid_attachment_content(ext: &str, content: &str) -> Result<(), String> {
    let ext = ext.to_ascii_lowercase();
    if ext == "md" {
        let blocks = extract_mermaid_from_markdown(content);
        for (i, block) in blocks.iter().enumerate() {
            if let Err(e) = validate_mermaid_syntax(block) {
                return Err(format!("Mermaid code block {} is invalid: {e}", i + 1));
            }
        }
        Ok(())
    } else if ext == "mmd" || ext == "mermaid" {
        validate_mermaid_syntax(content)
    } else {
        Ok(())
    }
}

/// File-backed wrapper over [`validate_mermaid_attachment_content`]. Reads the
/// file at `path` and validates its Mermaid content. Non-Mermaid extensions
/// short-circuit to `Ok(())` without reading the file.
pub fn validate_mermaid_attachment(path: &Path) -> Result<(), String> {
    if !should_validate_mermaid(path) {
        return Ok(());
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read attachment '{}': {e}", path.display()))?;
    validate_mermaid_attachment_content(ext, &content)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::path::Path;

    #[test]
    fn accepts_valid_flowchart() {
        assert!(validate_mermaid_syntax("graph TD\n  A-->B").is_ok());
        assert!(validate_mermaid_syntax("flowchart TD\n  A[Start] --> B[Done]").is_ok());
    }

    #[test]
    fn accepts_valid_sequence() {
        assert!(validate_mermaid_syntax("sequenceDiagram\n  Alice->>Bob: Hi").is_ok());
    }

    #[test]
    fn rejects_unterminated_node_label() {
        // Real syntax error caught by merman, not by the regex pre-check.
        let err = validate_mermaid_syntax("flowchart TD\n  A[Start --> B[Done").unwrap_err();
        assert!(!err.is_empty(), "merman must surface a parse error");
    }

    #[test]
    fn rejects_unknown_diagram_type() {
        let err = validate_mermaid_syntax("this is not a diagram at all").unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn rejects_empty_code() {
        assert!(validate_mermaid_syntax("").is_err());
    }

    #[test]
    fn precheck_rejects_quoted_subgraph_with_canonical_message() {
        let err = validate_mermaid_syntax("graph TB\n  subgraph \"Quoted\"\n  end").unwrap_err();
        assert!(
            err.contains("Quoted subgraph titles are not supported"),
            "got: {err}"
        );
    }

    #[test]
    fn precheck_rejects_invalid_subgraph_identifier_with_canonical_message() {
        let err = validate_mermaid_syntax("graph TB\n  subgraph Id!!!\n  end").unwrap_err();
        assert!(
            err.contains("Invalid subgraph identifier 'Id!!!'"),
            "got: {err}"
        );
    }

    #[test]
    fn accepts_generator_bounded_context_map_shape() {
        let code = "graph TB\n  BC1[\"Work Management<br/>Stories, Epics, Dependencies\"]\n  BC2[\"Specification<br/>Features, Scenarios, Steps\"]\n\n  BC1 -->|generates| BC2";
        assert!(
            validate_mermaid_syntax(code).is_ok(),
            "generator bc-map must parse"
        );
    }

    #[test]
    fn accepts_generator_event_flow_shape() {
        let code = "flowchart TB\n  subgraph Commands[\"\u{26a1} Commands\"]\n    C3[Create Work Unit]\n  end\n\n  subgraph Aggregates[\"\u{1f4e6} Aggregates\"]\n    A1[Work Unit]\n  end\n\n  Commands -.-> Aggregates";
        assert!(
            validate_mermaid_syntax(code).is_ok(),
            "generator event-flow must parse"
        );
    }

    #[test]
    fn should_validate_mermaid_by_extension() {
        assert!(should_validate_mermaid(Path::new("a.mmd")));
        assert!(should_validate_mermaid(Path::new("a.mermaid")));
        assert!(should_validate_mermaid(Path::new("a.md")));
        assert!(should_validate_mermaid(Path::new("a.MD")));
        assert!(!should_validate_mermaid(Path::new("a.png")));
        assert!(!should_validate_mermaid(Path::new("noext")));
    }

    #[test]
    fn extracts_mermaid_fences_from_markdown() {
        let md = "# Title\n\n```mermaid\ngraph TD\n  A-->B\n```\n\ntext\n\n```mermaid\nsequenceDiagram\n  A->>B: hi\n```\n";
        let blocks = extract_mermaid_from_markdown(md);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("graph TD"));
        assert!(blocks[1].contains("sequenceDiagram"));
    }

    #[test]
    fn markdown_without_fences_is_valid() {
        assert!(
            validate_mermaid_attachment_content("md", "# Just markdown\n\nno diagrams").is_ok()
        );
    }

    #[test]
    fn markdown_with_invalid_fence_reports_block_number() {
        let md = "```mermaid\ngraph TD\n  A-->B\n```\n\n```mermaid\nflowchart TD\n  A[Start --> B\n```\n";
        let err = validate_mermaid_attachment_content("md", md).unwrap_err();
        assert!(
            err.contains("Mermaid code block 2 is invalid"),
            "got: {err}"
        );
    }

    #[test]
    fn mmd_file_content_validated_as_single_diagram() {
        assert!(validate_mermaid_attachment_content("mmd", "graph TD\n  A-->B").is_ok());
        assert!(validate_mermaid_attachment_content("mermaid", "not a diagram").is_err());
    }

    #[test]
    fn non_mermaid_extension_is_not_validated() {
        assert!(validate_mermaid_attachment_content("png", "garbage bytes").is_ok());
    }
}
