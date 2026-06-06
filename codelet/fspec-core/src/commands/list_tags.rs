//! `list-tags` — Rust port of `src/commands/list-tags.ts` (RPC-251).
//!
//! Reads `spec/tags.json` (auto-creating it with the canonical 9-category
//! default when missing — parity with the TS `ensureTagsFile` helper),
//! projects each category to `{ name, tags: [{ tag, description }] }`,
//! alphabetically sorts the tags WITHIN each category, optionally
//! filters to a single category by exact `name` match, and emits either
//! a 2-space-indented JSON payload or a plain-text summary.
//!
//! Both invocation paths — the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand — call this single function
//! (RPC-003 §7/§11 two-front-doors invariant).
//!
//! `ensure_tags_file` is implemented locally for now; the orchestrator
//! will PROMOTE it to a shared helper in `io/ensure.rs` alongside
//! `ensure_prefixes_file` / `ensure_work_units_file` during the
//! Phase C wiring batch (see Phase C report). The local helper is
//! kept private (`fn`, not `pub fn`) so callers outside this module
//! cannot bind to it during the transition.

use std::cmp::Ordering;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_tags_file;

/// CLI arguments accepted by `list-tags`. The shell-facing `fspec
/// list-tags` advertises `--category` only; the dispatcher additionally
/// accepts a `format` discriminator so the agent-loop tool-call
/// protocol can request JSON output.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListTagsArgs {
    /// Exact-match category filter. When supplied, narrows the output
    /// to the single matching category or returns a structured
    /// `Category not found` error.
    #[serde(default)]
    category: Option<String>,
    /// `"text"` (default) or `"json"`. Dispatcher-only — the shell
    /// CLI registers no `--format` flag (matching the flag-less TS
    /// Commander.js registration at `src/commands/list-tags.ts:101-105`).
    #[serde(default)]
    format: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call
/// `std::env::current_dir()` so the same binary can serve multiple
/// sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListTagsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-tags",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Parity with TS: ensureTagsFile auto-creates the canonical
    // 9-category default if missing. Parse errors escalate via
    // FspecCoreError::ParseJson { file: "tags.json", ... }.
    let tags_data = ensure_tags_file(project_root)?;

    // Project tagsData.categories → CategoryEntry[], sorting tags
    // within each category alphabetically by tag.name (mirroring the
    // TS `localeCompare` call). Insertion order of categories
    // themselves is preserved verbatim — never re-sorted.
    let projected: Vec<CategoryEntry> = tags_data
        .categories
        .iter()
        .map(project_category)
        .collect();

    // Optional --category filter: exact-match against category.name.
    // On miss, emit the canonical `Category not found: <name>.
    // Available categories: <a>, <b>, …` error string (substring-
    // asserted by the dispatcher + CLI scenarios).
    let categories: Vec<CategoryEntry> = if let Some(filter) = args.category.as_deref() {
        let matched: Vec<CategoryEntry> = projected
            .iter()
            .filter(|c| c.name == filter)
            .cloned()
            .collect();
        if matched.is_empty() {
            let available = projected
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(FspecCoreError::InvalidArgs {
                command: "list-tags",
                reason: format!(
                    "Category not found: {filter}. Available categories: {available}"
                ),
            });
        }
        matched
    } else {
        projected
    };

    match args.format.as_deref() {
        Some("json") => {
            // Mirror TS `{ success: true, categories }` shape with
            // declaration-order field preservation. Using a typed
            // struct (rather than `json!{}`) so `Serialize` keeps
            // `success` before `categories` instead of alphabetising
            // the root keys via the underlying `BTreeMap`.
            #[derive(Serialize)]
            struct JsonResult<'a> {
                success: bool,
                categories: &'a [CategoryEntry],
            }
            let result = JsonResult {
                success: true,
                categories: &categories,
            };
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "list-tags",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // Default to text rendering — parity with the TS CLI wrapper
        // (`src/commands/list-tags.ts:64-83`) which always renders
        // via `output.log` when the command is invoked from the
        // shell.
        _ => Ok(render_text(&categories)),
    }
}

/// In-memory projection shape. Mirrors the TS `CategoryEntry`
/// interface at `src/commands/list-tags.ts:12-15` and `TagEntry` at
/// lines 7-10. `Serialize` field-order = declaration-order so the
/// JSON output reads `{ name, tags: [{ tag, description }] }`
/// — never alphabetised by serde_json's underlying BTreeMap.
#[derive(Debug, Clone, Serialize)]
struct CategoryEntry {
    name: String,
    tags: Vec<TagEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct TagEntry {
    tag: String,
    description: String,
}

/// Project a single `TagCategory` into its `CategoryEntry`
/// representation. Tag projection drops every auxiliary field
/// (`usage`, `scope`, `examples`, …) and the resulting `Vec` is
/// sorted alphabetically by `tag` — matching the TS
/// `.map(...).sort((a, b) => a.tag.localeCompare(b.tag))` pipeline.
fn project_category(cat: &crate::types::tags::TagCategory) -> CategoryEntry {
    let mut tags: Vec<TagEntry> = cat
        .tags
        .iter()
        .map(|t| TagEntry {
            tag: t.name.clone(),
            description: t.description.clone(),
        })
        .collect();
    tags.sort_by(|a, b| cmp_tag(&a.tag, &b.tag));
    CategoryEntry {
        name: cat.name.clone(),
        tags,
    }
}

/// Lexicographic comparison standing in for the TS
/// `String.prototype.localeCompare` call. For the ASCII-only tag
/// names used by fspec (`@` + alphanumeric), Rust's default `cmp`
/// matches `localeCompare` byte-for-byte; we factor it out into a
/// dedicated fn so a future locale-aware refactor lands as a
/// one-call change.
fn cmp_tag(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}

/// Render the text format expected by the TS CLI wrapper
/// (`src/commands/list-tags.ts:68-83`).
///
/// Exact output structure (chalk stripped) per category:
///
/// ```text
/// \n<name> (<N> tags)\n        ← leading \n is part of the chalk arg
///   No tags registered\n       ← when N == 0
///   <tag> - <description>\n    ← per tag, when N > 0
/// ```
///
/// followed by a trailing `output.log('')` which emits one more `\n`
/// after the last category. The output therefore ALWAYS ends with at
/// least one `\n`.
fn render_text(categories: &[CategoryEntry]) -> String {
    let mut out = String::new();
    for cat in categories {
        out.push('\n');
        out.push_str(&cat.name);
        out.push_str(&format!(" ({} tags)", cat.tags.len()));
        out.push('\n');
        if cat.tags.is_empty() {
            out.push_str("  No tags registered\n");
        } else {
            for t in &cat.tags {
                out.push_str("  ");
                out.push_str(&t.tag);
                out.push_str(" - ");
                out.push_str(&t.description);
                out.push('\n');
            }
        }
    }
    // Trailing `output.log('')` — emits one final `\n`.
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn make_tags_file(project_root: &Path, raw: &str) {
        let spec = project_root.join("spec");
        std::fs::create_dir_all(&spec).unwrap();
        std::fs::write(spec.join("tags.json"), raw).unwrap();
    }

    fn poll(f: impl std::future::Future<Output = Result<String, FspecCoreError>>) -> Result<String, FspecCoreError> {
        let mut fut = Box::pin(f);
        let waker = std::task::Waker::noop();
        let mut cx = std::task::Context::from_waker(waker);
        match fut.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(v) => v,
            std::task::Poll::Pending => panic!("future returned Pending"),
        }
    }

    #[test]
    fn args_parse_with_defaults() {
        let a: ListTagsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.category.is_none());
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_format_json() {
        let a: ListTagsArgs = serde_json::from_str(r#"{"format":"json"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn args_parse_category_filter() {
        let a: ListTagsArgs = serde_json::from_str(r#"{"category":"Phase Tags"}"#).unwrap();
        assert_eq!(a.category.as_deref(), Some("Phase Tags"));
    }

    #[test]
    fn project_category_sorts_tags_alphabetically() {
        let cat = serde_json::from_value::<crate::types::tags::TagCategory>(json!({
            "name": "Phase Tags",
            "description": "x",
            "required": true,
            "tags": [
                { "name": "@zed", "description": "Z desc" },
                { "name": "@aaa", "description": "A desc" }
            ]
        })).unwrap();
        let p = project_category(&cat);
        assert_eq!(p.tags.len(), 2);
        assert_eq!(p.tags[0].tag, "@aaa");
        assert_eq!(p.tags[1].tag, "@zed");
    }

    #[test]
    fn project_category_drops_auxiliary_tag_fields() {
        let cat = serde_json::from_value::<crate::types::tags::TagCategory>(json!({
            "name": "Phase Tags",
            "description": "x",
            "required": true,
            "tags": [
                {
                    "name": "@critical",
                    "description": "c",
                    "usage": "rare",
                    "scope": "wide"
                }
            ]
        })).unwrap();
        let p = project_category(&cat);
        let serialised = serde_json::to_value(&p.tags[0]).unwrap();
        assert!(serialised.get("usage").is_none());
        assert!(serialised.get("scope").is_none());
        assert_eq!(serialised["tag"].as_str(), Some("@critical"));
        assert_eq!(serialised["description"].as_str(), Some("c"));
    }

    #[test]
    fn render_text_handles_empty_category() {
        let cats = vec![CategoryEntry {
            name: "Phase Tags".to_string(),
            tags: vec![],
        }];
        let out = render_text(&cats);
        assert!(out.contains("Phase Tags (0 tags)"));
        assert!(out.contains("  No tags registered"));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn render_text_emits_one_line_per_tag() {
        let cats = vec![CategoryEntry {
            name: "Phase Tags".to_string(),
            tags: vec![TagEntry {
                tag: "@critical".to_string(),
                description: "Critical features".to_string(),
            }],
        }];
        let out = render_text(&cats);
        assert!(out.lines().any(|l| l == "  @critical - Critical features"));
        assert!(out.contains("Phase Tags (1 tags)"));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn run_auto_creates_file_when_missing() {
        let tmp = TempDir::new().unwrap();
        assert!(!tmp.path().join("spec/tags.json").exists());
        let out = poll(run(r#"{"format":"json"}"#, tmp.path())).unwrap();
        assert!(tmp.path().join("spec/tags.json").exists());
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["categories"].as_array().unwrap().len(), 9);
    }

    #[test]
    fn run_escalates_malformed_tags_json() {
        let tmp = TempDir::new().unwrap();
        make_tags_file(tmp.path(), "{ not json");
        let err = poll(run("{}", tmp.path())).unwrap_err();
        assert!(
            err.to_string().contains("Failed to parse tags.json"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn run_filters_by_category_with_substring_match_disallowed() {
        let tmp = TempDir::new().unwrap();
        let raw = r#"{
  "categories": [
    { "name": "Phase Tags", "description": "x", "required": true, "tags": [] },
    { "name": "Component Tags", "description": "x", "required": true, "tags": [] }
  ]
}"#;
        make_tags_file(tmp.path(), raw);
        let out = poll(run(
            r#"{"category":"Phase Tags","format":"json"}"#,
            tmp.path(),
        ))
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let cats = v["categories"].as_array().unwrap();
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0]["name"].as_str(), Some("Phase Tags"));
    }

    #[test]
    fn run_unknown_category_returns_canonical_error_substring() {
        let tmp = TempDir::new().unwrap();
        let raw = r#"{
  "categories": [
    { "name": "Phase Tags", "description": "x", "required": true, "tags": [] },
    { "name": "Component Tags", "description": "x", "required": true, "tags": [] }
  ]
}"#;
        make_tags_file(tmp.path(), raw);
        let err = poll(run(r#"{"category":"Nope"}"#, tmp.path())).unwrap_err();
        assert!(
            err.to_string()
                .contains("Category not found: Nope. Available categories: Phase Tags, Component Tags"),
            "unexpected error: {err}"
        );
    }
}
