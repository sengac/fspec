//! `list-features` — Rust port of `src/commands/list-features.ts` (RPC-245).
//!
//! Walks `spec/features/**/*.feature`, parses each file with an inline
//! gherkin line-scanner (parity with the parts of `@cucumber/gherkin` the
//! TS implementation depends on — Feature name, top-level tags, and the
//! count of `Scenario:` / `Scenario Outline:` children), and emits either
//! a 2-space-indented JSON payload or a plain-text listing.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.
//!
//! Error semantics mirror the TS implementation:
//!   - Missing `spec/features/` → escalated structured error containing
//!     the substring `"Directory not found: spec/features/"`. The CLI
//!     bridge inspects this substring to choose exit code 2 vs 1.
//!   - Per-file parse failures → silently skipped (no escalation).
//!     Parity with `src/commands/list-features.ts:92-95` bare `catch`.
//!   - Empty results → success with an empty features array.
//!
//! The inline scanner is a deliberate divergence from the upstream
//! `gherkin` crate — it understands the subset of Gherkin needed by
//! `list-features` (Feature header + tag lines + Scenario/Scenario
//! Outline keyword counting). The Rust port intentionally avoids the
//! `gherkin` crate dependency to keep the dep tree tight; see
//! architecture note [0] on RPC-245 for the canonical rationale.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;

/// CLI arguments accepted by `list-features`.
///
/// Mirrors the TS `ListFeaturesOptions` interface at
/// `src/commands/list-features.ts:17-20`, plus the dispatcher-only
/// `format` selector also exposed by every other ported listing command
/// (e.g. `list-prefixes`, `list-work-units`).
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListFeaturesArgs {
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
    /// Exact-string tag filter (e.g. `"@critical"`). When set, only
    /// features whose top-level tag set `.includes()` this value
    /// survive the filter (parity with TS line 76).
    #[serde(default)]
    tag: Option<String>,
}

/// One row of the rendered listing. Mirrors the TS `FeatureInfo` interface
/// at `src/commands/list-features.ts:10-15`. Field declaration order is
/// preserved verbatim so the JSON output matches `JSON.stringify`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FeatureInfo {
    file: String,
    name: String,
    scenario_count: usize,
    tags: Vec<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call
/// `std::env::current_dir()` so the same binary can serve multiple
/// sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListFeaturesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-features",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Resolve the candidate file set. ENOENT on spec/features/ escalates
    // here (parity with TS `access(featuresDir)` ENOENT branch).
    let files = glob_feature_files(project_root)?;

    // Parse each candidate, skipping files whose top-level Feature
    // header cannot be located (parity with the TS bare `catch` at
    // `src/commands/list-features.ts:92-95`). Tag filter is applied
    // INSIDE the loop, AFTER parse, so unparseable + filtered-out files
    // both drop out before sorting.
    //
    // Per-file parse failures emit `Warning: Could not parse <path>` on
    // stderr (parity with TS `output.warn(...)` at line 94) WHEN the
    // output format is text/default. The JSON format path stays silent
    // so structured consumers receive a clean stderr alongside the
    // pretty-printed payload on stdout.
    let emit_warnings = !matches!(args.format.as_deref(), Some("json"));
    let mut features: Vec<FeatureInfo> = Vec::new();
    for file in files {
        let abs = project_root.join(&file);
        let content = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            // I/O error reading a single feature file is treated as a
            // parse failure — silently skipped, parity with TS.
            Err(_) => {
                if emit_warnings {
                    eprintln!("Warning: Could not parse {file}");
                }
                continue;
            }
        };
        let parsed = match parse_feature_header(&content) {
            Some(p) => p,
            None => {
                if emit_warnings {
                    eprintln!("Warning: Could not parse {file}");
                }
                continue;
            }
        };
        if let Some(ref needle) = args.tag {
            if !parsed.tags.iter().any(|t| t == needle) {
                continue;
            }
        }
        features.push(FeatureInfo {
            file,
            name: parsed.name,
            scenario_count: parsed.scenario_count,
            tags: parsed.tags,
        });
    }

    // Sort alphabetically by file path (parity with TS
    // `features.sort((a,b) => a.file.localeCompare(b.file))`). For ASCII
    // forward-slash paths the default Rust ordering matches localeCompare.
    features.sort_by(|a, b| a.file.cmp(&b.file));

    match args.format.as_deref() {
        Some("json") => {
            let payload = json!({ "features": features });
            serde_json::to_string_pretty(&payload).map_err(|e| FspecCoreError::InvalidArgs {
                command: "list-features",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // Default to text format.
        _ => Ok(render_text(&features, args.tag.as_deref())),
    }
}

/// Top-level metadata extracted from a `.feature` file by
/// [`parse_feature_header`].
struct ParsedHeader {
    name: String,
    tags: Vec<String>,
    scenario_count: usize,
}

/// Inline gherkin scanner — extracts the Feature name, top-level tag set,
/// and total Scenario/Scenario Outline count from a `.feature` file.
///
/// Returns `None` when the file does NOT contain a `Feature:` keyword line
/// (parity with the TS `gherkinDocument.feature` truthiness check at
/// `src/commands/list-features.ts:72-91`). Lines starting with `#`
/// (Gherkin comments) are ignored. Tag lines are accumulated until a
/// keyword line (`Feature:`, `Scenario:`, `Scenario Outline:`,
/// `Background:`, `Rule:`) is encountered, at which point the accumulated
/// set is attached to that keyword.
fn parse_feature_header(content: &str) -> Option<ParsedHeader> {
    let mut pending_tags: Vec<String> = Vec::new();
    let mut feature_name: Option<String> = None;
    let mut feature_tags: Vec<String> = Vec::new();
    let mut scenario_count: usize = 0;

    for raw in content.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('@') {
            // Tag line — collect every whitespace-separated `@<name>`
            // token. Anything that doesn't start with `@` (e.g. an
            // inline comment after the last tag) is silently skipped.
            for tok in trimmed.split_whitespace() {
                if tok.starts_with('@') {
                    pending_tags.push(tok.to_string());
                }
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("Feature:") {
            // First Feature header wins (Gherkin permits exactly one).
            if feature_name.is_none() {
                feature_name = Some(rest.trim().to_string());
                feature_tags = std::mem::take(&mut pending_tags);
            } else {
                pending_tags.clear();
            }
            continue;
        }
        if trimmed.starts_with("Scenario Outline:") || trimmed.starts_with("Scenario:") {
            // Only count scenarios that appear under a declared Feature
            // header — leading lines before `Feature:` cannot reach
            // here in well-formed input, but we guard anyway.
            if feature_name.is_some() {
                scenario_count += 1;
            }
            pending_tags.clear();
            continue;
        }
        if trimmed.starts_with("Background:") || trimmed.starts_with("Rule:") {
            pending_tags.clear();
            continue;
        }
        // Step lines, doc-strings, data-tables, free-form description
        // text: tags do NOT attach across these. We deliberately do not
        // clear pending_tags here because a tag block immediately
        // preceding a keyword is the only legal Gherkin shape; if the
        // first non-tag non-keyword line is description text, the
        // accumulated tags were never meant for anything anyway.
    }

    feature_name.map(|name| ParsedHeader {
        name,
        tags: feature_tags,
        scenario_count,
    })
}

/// Render the text output format expected by the TS CLI wrapper
/// (`src/commands/list-features.ts:109-134`).
///
/// Returns the rendered text. The populated path appends a trailing `\n`
/// after the summary line (`Found N feature files\n`). The empty-list
/// sentinel path returns `"No feature files found in spec/features/"`
/// WITHOUT a trailing newline — the CLI bridge re-appends one for
/// shell-pipeline friendliness.
fn render_text(features: &[FeatureInfo], tag_filter: Option<&str>) -> String {
    if features.is_empty() {
        // Sentinel — exact byte-for-byte parity with TS line 111.
        return "No feature files found in spec/features/".to_string();
    }

    let mut out = String::new();
    for f in features {
        let tags_str = if f.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", f.tags.join(" "))
        };
        out.push_str(&format!(
            "  {} - {} ({} scenarios){}\n",
            f.file, f.name, f.scenario_count, tags_str
        ));
    }
    out.push('\n');
    if let Some(tag) = tag_filter {
        out.push_str(&format!(
            "Found {} feature files matching {}\n",
            features.len(),
            tag
        ));
    } else {
        out.push_str(&format!("Found {} feature files\n", features.len()));
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_with_defaults() {
        let a: ListFeaturesArgs = serde_json::from_str("{}").unwrap();
        assert!(a.format.is_none());
        assert!(a.tag.is_none());
    }

    #[test]
    fn args_parse_format_and_tag() {
        let a: ListFeaturesArgs =
            serde_json::from_str(r#"{"format":"json","tag":"@critical"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
        assert_eq!(a.tag.as_deref(), Some("@critical"));
    }

    #[test]
    fn parse_header_extracts_name_tags_and_scenario_count() {
        let body = "@critical @auth\nFeature: User Authentication\n\n  Scenario: A\n    Given x\n  Scenario: B\n    Given y\n";
        let p = parse_feature_header(body).unwrap();
        assert_eq!(p.name, "User Authentication");
        assert_eq!(p.tags, vec!["@critical", "@auth"]);
        assert_eq!(p.scenario_count, 2);
    }

    #[test]
    fn parse_header_returns_none_when_no_feature_keyword() {
        assert!(parse_feature_header("not a feature file").is_none());
    }

    #[test]
    fn parse_header_counts_scenario_outline_as_scenario() {
        let body = "Feature: F\n  Scenario Outline: O\n    Given <x>\n  Scenario: S\n";
        let p = parse_feature_header(body).unwrap();
        assert_eq!(p.scenario_count, 2);
    }

    #[test]
    fn parse_header_skips_comments_and_blanks() {
        let body = "# comment\n\n@critical\n# another\nFeature: F\n\n  Scenario: A\n";
        let p = parse_feature_header(body).unwrap();
        assert_eq!(p.name, "F");
        assert_eq!(p.tags, vec!["@critical"]);
        assert_eq!(p.scenario_count, 1);
    }

    #[test]
    fn render_text_empty_returns_sentinel_no_newline() {
        assert_eq!(
            render_text(&[], None),
            "No feature files found in spec/features/"
        );
    }

    #[test]
    fn render_text_populated_omits_tag_brackets_when_no_tags() {
        let features = vec![FeatureInfo {
            file: "spec/features/a.feature".into(),
            name: "Alpha".into(),
            scenario_count: 1,
            tags: vec![],
        }];
        let out = render_text(&features, None);
        assert!(
            out.lines()
                .any(|l| l == "  spec/features/a.feature - Alpha (1 scenarios)"),
            "missing untagged line; got:\n{out}"
        );
        assert!(out.lines().any(|l| l == "Found 1 feature files"));
    }

    #[test]
    fn render_text_populated_includes_tag_brackets_when_tagged() {
        let features = vec![FeatureInfo {
            file: "spec/features/auth.feature".into(),
            name: "User Authentication".into(),
            scenario_count: 2,
            tags: vec!["@critical".into(), "@auth".into()],
        }];
        let out = render_text(&features, None);
        assert!(
            out.lines().any(|l| l
                == "  spec/features/auth.feature - User Authentication (2 scenarios) [@critical @auth]"),
            "missing tagged line; got:\n{out}"
        );
    }

    #[test]
    fn render_text_tag_filter_uses_matching_summary_phrasing() {
        let features = vec![FeatureInfo {
            file: "spec/features/auth.feature".into(),
            name: "Auth".into(),
            scenario_count: 1,
            tags: vec!["@critical".into()],
        }];
        let out = render_text(&features, Some("@critical"));
        assert!(
            out.lines()
                .any(|l| l == "Found 1 feature files matching @critical"),
            "missing matching-summary line; got:\n{out}"
        );
    }
}
