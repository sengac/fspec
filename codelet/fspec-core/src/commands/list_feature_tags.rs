//! `list-feature-tags` — Rust port of `src/commands/list-feature-tags.ts` (RPC-244).
//!
//! Reads a single `.feature` file, parses out the feature-level tag block
//! using an inline gherkin line-scanner (parity with the approach used by
//! `list_features.rs`), and returns the canonical
//! `{success, tags, message?, categorizedTags?, error?}` result shape.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single `pub async fn run`
//! — RPC-003 §7/§11 two-front-doors invariant.
//!
//! Behaviour parity with TypeScript (`src/commands/list-feature-tags.ts`):
//!
//! * Missing file (ENOENT) → `{success:false, tags:[], error:"File not
//!   found: <path>"}` (TS lines 33-42).
//! * Invalid Gherkin / no Feature header → `{success:false, tags:[],
//!   error:"File does not contain a valid Feature"}` (TS lines 61-67).
//!   In the TS impl, the `@cucumber/gherkin` parser also throws on
//!   malformed input; the inline scanner here folds both cases into the
//!   "no Feature header found" branch, which matches the TS observable
//!   behaviour on the inputs exercised by the feature file.
//! * Feature with no tags → `{success:true, tags:[], message:"No tags
//!   found on this feature"}` (TS lines 72-78).
//! * Feature with tags → `{success:true, tags:[...]}` in declaration
//!   order; NO `message` field (TS lines 112-115).
//! * `showCategories=true` with a readable+parseable `spec/tags.json` →
//!   adds `categorizedTags` array of `{tag, category}` pairs; unknown
//!   tags map to `"Unknown"` (TS lines 80-102).
//! * `showCategories=true` but tags.json missing/unreadable/malformed →
//!   silently degrades to returning `tags` without `categorizedTags`
//!   (TS lines 103-109 bare `catch`).
//!
//! Only `args_json` deserialisation failure escalates as a
//! `FspecCoreError::InvalidArgs`; every other failure mode lives inside
//! the structured `{success:false, error:...}` result.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::types::tags::TagsData;

/// CLI / dispatcher arguments accepted by `list-feature-tags`.
///
/// Mirrors the TS `ListFeatureTagsOptions` interface at
/// `src/commands/list-feature-tags.ts:9-12` plus the dispatcher-only
/// `format` discriminator exposed by every other ported listing command.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListFeatureTagsArgs {
    /// Required feature file path (project-root-relative). The TS
    /// implementation `join`s this with `options.cwd || process.cwd()`;
    /// here we always resolve against the supplied `project_root` so
    /// the same binary can serve multiple sessions safely.
    #[serde(default)]
    file: Option<String>,
    /// Whether to attach a `categorizedTags` array using `spec/tags.json`.
    #[serde(default)]
    show_categories: bool,
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// One entry in the `categorizedTags` array returned when
/// `showCategories=true` and `spec/tags.json` is readable. Field
/// declaration order is preserved on the wire so `{tag, category}` is
/// stable (parity with the TS object literal at
/// `src/commands/list-feature-tags.ts:93-96`).
#[derive(Debug, Serialize)]
struct CategorizedTag {
    tag: String,
    category: String,
}

/// Canonical response shape. Mirrors the TS `ListFeatureTagsResult`
/// interface at `src/commands/list-feature-tags.ts:14-20`. Optional
/// fields are `#[serde(skip_serializing_if = "Option::is_none")]` /
/// `Vec::is_empty` so they are omitted in branches where the TS
/// implementation never sets them — keeping the JSON shape stable and
/// minimal.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListFeatureTagsResult {
    success: bool,
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    categorized_tags: Option<Vec<CategorizedTag>>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call
/// `std::env::current_dir()` so the same binary can serve multiple
/// sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListFeatureTagsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-feature-tags",
            reason: format!("failed to parse args: {e}"),
        })?;

    let file_rel = args.file.clone().ok_or_else(|| FspecCoreError::InvalidArgs {
        command: "list-feature-tags",
        reason: "missing required 'file' argument".to_string(),
    })?;

    let result = load_feature_tags(project_root, &file_rel, args.show_categories);

    match args.format.as_deref() {
        Some("json") => serde_json::to_string_pretty(&result).map_err(|e| {
            FspecCoreError::InvalidArgs {
                command: "list-feature-tags",
                reason: format!("failed to serialize result: {e}"),
            }
        }),
        // Default to text.
        _ => Ok(render_text(&result)),
    }
}

/// Top-level orchestration of the read/parse/categorize pipeline.
///
/// Errors are surfaced as `{success:false, error:...}` inside the
/// returned [`ListFeatureTagsResult`] — never escalated as a
/// `FspecCoreError`. This matches the TS `ListFeatureTagsResult` shape
/// exactly.
fn load_feature_tags(
    project_root: &Path,
    file_rel: &str,
    show_categories: bool,
) -> ListFeatureTagsResult {
    let abs = project_root.join(file_rel);

    let content = match std::fs::read_to_string(&abs) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return ListFeatureTagsResult {
                success: false,
                tags: Vec::new(),
                message: None,
                error: Some(format!("File not found: {file_rel}")),
                categorized_tags: None,
            };
        }
        Err(e) => {
            // Other I/O errors are surfaced as a structured error too —
            // the TS impl `throw`s here, which would surface to the CLI
            // as a non-zero exit; routing it through `error` is the
            // dispatcher-friendly equivalent and avoids escalating
            // through FspecCoreError (parity with the rest of this
            // command's error model).
            return ListFeatureTagsResult {
                success: false,
                tags: Vec::new(),
                message: None,
                error: Some(format!("Failed to read {file_rel}: {e}")),
                categorized_tags: None,
            };
        }
    };

    let feature_tags = match parse_feature_tags(&content) {
        Some(tags) => tags,
        None => {
            return ListFeatureTagsResult {
                success: false,
                tags: Vec::new(),
                message: None,
                error: Some("File does not contain a valid Feature".to_string()),
                categorized_tags: None,
            };
        }
    };

    if feature_tags.is_empty() {
        return ListFeatureTagsResult {
            success: true,
            tags: Vec::new(),
            message: Some("No tags found on this feature".to_string()),
            error: None,
            categorized_tags: None,
        };
    }

    if show_categories {
        if let Some(cats) = try_categorize(project_root, &feature_tags) {
            return ListFeatureTagsResult {
                success: true,
                tags: feature_tags,
                message: None,
                error: None,
                categorized_tags: Some(cats),
            };
        }
        // Silent degradation — parity with TS bare catch at lines 103-109.
    }

    ListFeatureTagsResult {
        success: true,
        tags: feature_tags,
        message: None,
        error: None,
        categorized_tags: None,
    }
}

/// Inline gherkin scanner — extracts ONLY the feature-level tag block
/// (the tag lines immediately preceding the first `Feature:` keyword).
///
/// Returns `None` when the file does NOT contain a `Feature:` keyword
/// line (parity with the TS `gherkinDocument.feature` truthiness check
/// at `src/commands/list-feature-tags.ts:61-67`). Lines starting with
/// `#` (Gherkin comments) are ignored. Tag lines are accumulated until
/// a keyword line is encountered, at which point the accumulated set is
/// attached to that keyword. Only the tags attached to `Feature:` are
/// returned — scenario-level tags are explicitly excluded.
fn parse_feature_tags(content: &str) -> Option<Vec<String>> {
    let mut pending_tags: Vec<String> = Vec::new();
    let mut feature_tags: Option<Vec<String>> = None;

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
        if trimmed.starts_with("Feature:") {
            // First Feature header wins (Gherkin permits exactly one).
            // Attach the accumulated tag block to it and STOP collecting —
            // every subsequent tag line belongs to a scenario / scenario
            // outline / rule / background, which the TS impl excludes
            // via `gherkinDocument.feature.tags` (NOT scenario tags).
            if feature_tags.is_none() {
                feature_tags = Some(std::mem::take(&mut pending_tags));
            } else {
                pending_tags.clear();
            }
            continue;
        }
        // Any other keyword (Scenario:, Scenario Outline:, Background:,
        // Rule:) or free-form description text resets the pending block.
        // We've already captured the feature-level tags by this point,
        // so further tag collection is irrelevant.
        pending_tags.clear();
    }

    feature_tags
}

/// Build the `categorizedTags` projection from `spec/tags.json`.
///
/// Returns `None` if the file is missing, unreadable, or malformed —
/// the caller silently degrades to returning tags without categories
/// (parity with the bare TS catch at
/// `src/commands/list-feature-tags.ts:103-109`).
///
/// Unknown tags map to category `"Unknown"` (TS line 95).
fn try_categorize(project_root: &Path, tags: &[String]) -> Option<Vec<CategorizedTag>> {
    let path = project_root.join("spec").join("tags.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let parsed: TagsData = serde_json::from_str(&raw).ok()?;

    // Build tag → category map by walking every category's tag list.
    // Earlier categories win on duplicate registrations (Gherkin allows
    // a tag to appear in only one category in practice; the TS impl
    // uses a Map and overwrites silently — our `HashMap::entry().or_insert`
    // semantics match the TS Map.set behaviour because we iterate in
    // declaration order).
    let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for category in &parsed.categories {
        for tag in &category.tags {
            map.entry(tag.name.clone())
                .or_insert_with(|| category.name.clone());
        }
    }

    Some(
        tags.iter()
            .map(|t| CategorizedTag {
                tag: t.clone(),
                category: map.get(t).cloned().unwrap_or_else(|| "Unknown".to_string()),
            })
            .collect(),
    )
}

/// Render the documented text-format layout.
///
/// Layout:
///
/// ```text
/// Tags on this feature:
///
///   @tag1
///   @tag2
/// ```
///
/// Empty/missing-tag/error cases collapse to a single line — the
/// canonical sentinel message or the error string, mirroring what the
/// TS CLI prints via `output.log(result.message)` /
/// `output.error('Error:', result.error)` (without the colourised
/// "Error:" prefix, since the dispatcher returns structured text not
/// terminal output).
fn render_text(result: &ListFeatureTagsResult) -> String {
    if let Some(err) = &result.error {
        return err.clone();
    }
    if let Some(msg) = &result.message {
        return msg.clone();
    }

    let mut out = String::from("Tags on this feature:\n\n");
    for tag in &result.tags {
        out.push_str("  ");
        out.push_str(tag);
        out.push('\n');
    }
    // Trim trailing newline so the output ends cleanly — callers can
    // append their own newline if needed.
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_with_defaults() {
        let a: ListFeatureTagsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.file.is_none());
        assert!(!a.show_categories);
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_camel_case_show_categories() {
        let a: ListFeatureTagsArgs =
            serde_json::from_str(r#"{"file":"f.feature","showCategories":true,"format":"json"}"#)
                .unwrap();
        assert_eq!(a.file.as_deref(), Some("f.feature"));
        assert!(a.show_categories);
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn parse_feature_tags_extracts_declaration_order() {
        let body = "@critical @auth @wip\nFeature: User Authentication\n\n  Scenario: A\n    Given x\n";
        let tags = parse_feature_tags(body).unwrap();
        assert_eq!(tags, vec!["@critical", "@auth", "@wip"]);
    }

    #[test]
    fn parse_feature_tags_returns_none_without_feature_header() {
        assert!(parse_feature_tags("This is not gherkin at all\n").is_none());
    }

    #[test]
    fn parse_feature_tags_excludes_scenario_level_tags() {
        let body = "@critical\nFeature: Mixed\n\n  @smoke\n  Scenario: A\n    Given x\n";
        let tags = parse_feature_tags(body).unwrap();
        assert_eq!(tags, vec!["@critical"]);
    }

    #[test]
    fn parse_feature_tags_returns_empty_for_no_tag_feature() {
        let body = "Feature: No Tags\n  Scenario: A\n    Given x\n";
        let tags = parse_feature_tags(body).unwrap();
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_feature_tags_skips_comments_and_blanks() {
        let body = "# comment\n\n@critical\n# another\nFeature: F\n  Scenario: A\n";
        let tags = parse_feature_tags(body).unwrap();
        assert_eq!(tags, vec!["@critical"]);
    }

    #[test]
    fn render_text_error_returns_error_string() {
        let r = ListFeatureTagsResult {
            success: false,
            tags: Vec::new(),
            message: None,
            error: Some("File not found: x.feature".to_string()),
            categorized_tags: None,
        };
        assert_eq!(render_text(&r), "File not found: x.feature");
    }

    #[test]
    fn render_text_message_returns_message_string() {
        let r = ListFeatureTagsResult {
            success: true,
            tags: Vec::new(),
            message: Some("No tags found on this feature".to_string()),
            error: None,
            categorized_tags: None,
        };
        assert_eq!(render_text(&r), "No tags found on this feature");
    }

    #[test]
    fn render_text_populated_uses_bullet_layout() {
        let r = ListFeatureTagsResult {
            success: true,
            tags: vec!["@critical".to_string(), "@auth".to_string()],
            message: None,
            error: None,
            categorized_tags: None,
        };
        let out = render_text(&r);
        assert!(out.starts_with("Tags on this feature:\n\n"));
        assert!(out.lines().any(|l| l == "  @critical"));
        assert!(out.lines().any(|l| l == "  @auth"));
    }
}
