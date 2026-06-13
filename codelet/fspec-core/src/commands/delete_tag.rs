//! `delete-tag` — Rust port of `src/commands/delete-tag.ts` (RPC-222).
//!
//! Locates a tag by exact-match name across all categories, optionally
//! scans `spec/features/**/*.feature` for in-use references, then either
//! blocks the deletion (default path), emits a non-blocking warning
//! (`--force`), or reports the intended deletion without mutating disk
//! (`--dry-run`). On the non-dry-run write path, splices the tag out of
//! its category, atomically writes `spec/tags.json` via
//! [`crate::io::locked_file::write_json_atomic`], and regenerates
//! `spec/TAGS.md` from the in-memory `TagsData`.
//!
//! Both invocation paths — the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand — call this single function
//! (RPC-003 §7/§11 two-front-doors invariant).
//!
//! Divergences from the TS implementation (`src/commands/delete-tag.ts`):
//!
//! * Ajv schema validation against `src/schemas/tags.schema.json` is
//!   NOT replicated. Upstream gates (tag lookup + `TagsData` serde
//!   shape) collectively enforce the schema invariants.
//! * `statistics.lastUpdated` is NOT bumped — TS code has no
//!   `lastUpdated` mutation in `delete-tag.ts`. Only `register-tag`
//!   bumps it.
//! * Outer try/catch is flattened into direct
//!   [`FspecCoreError`] returns for parse / IO failures.
//! * `fileManager.transaction()` is replaced with
//!   [`crate::io::locked_file::write_json_atomic`].
//! * The TS implementation uses `tinyglobby` over
//!   `spec/features/**/*.feature`. The Rust port uses a hand-rolled
//!   recursive `std::fs::read_dir` walk restricted to `*.feature`
//!   under `spec/features/`. Glob / IO failures are SWALLOWED
//!   (best-effort scan) to mirror TS `catch {}` blocks.
//! * Inline minimal `generate_tags_md` helper duplicated from
//!   `register_tag.rs` / `update_tag.rs`. Will be promoted to a shared
//!   `generators` module in a follow-up batch.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::types::tags::TagsData;

/// CLI arguments accepted by `delete-tag`. Mirrors the TS Commander.js
/// registration at `src/commands/delete-tag.ts:199-206`:
/// - positional `<tag>` (required)
/// - `--force` (boolean flag)
/// - `--dry-run` (boolean flag)
/// - `--format` is dispatcher-only (not advertised by the clap surface).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteTagArgs {
    tag: String,
    #[serde(default)]
    force: bool,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    format: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call
/// `std::env::current_dir()` so the same binary can serve multiple
/// sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DeleteTagArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "delete-tag",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Gate 1: spec/tags.json must exist ----
    // Mirrors src/commands/delete-tag.ts:35-40 — delete-tag does NOT
    // auto-create (opposite of register-tag).
    let tags_json_path = project_root.join("spec").join("tags.json");
    if !tags_json_path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-tag",
            reason: "spec/tags.json not found".to_string(),
        });
    }

    // ---- Load tags.json ----
    let raw = std::fs::read_to_string(&tags_json_path).map_err(|source| FspecCoreError::Io {
        command: "delete-tag",
        source,
    })?;
    let mut tags_data: TagsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "tags.json".to_string(),
            reason: e.to_string(),
        })?;

    // ---- Locate the tag (linear scan, case-sensitive exact-match) ----
    // Mirrors L48-58.
    let mut current_cat_index: Option<usize> = None;
    let mut tag_index: Option<usize> = None;
    for (ci, cat) in tags_data.categories.iter().enumerate() {
        if let Some(ti) = cat.tags.iter().position(|t| t.name == args.tag) {
            current_cat_index = Some(ci);
            tag_index = Some(ti);
            break;
        }
    }
    let (current_cat_index, tag_index) = match (current_cat_index, tag_index) {
        (Some(c), Some(t)) => (c, t),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "delete-tag",
                reason: format!("Tag {} not found in registry", args.tag),
            });
        }
    };

    // Capture the canonical category name for the dry-run message
    // (mirrors L123 — `Would delete tag X from category "Y"`).
    let current_category_name = tags_data.categories[current_cat_index].name.clone();

    // ---- Usage scan (default + --force paths; skipped on --dry-run) ----
    // Mirrors L68-117. Glob / IO failures are SWALLOWED — mirror TS
    // `catch {}` blocks (L89-91, L114-116).
    let files_using_tag: Vec<String> = if args.dry_run {
        Vec::new()
    } else {
        scan_feature_files_for_tag(project_root, &args.tag).unwrap_or_default()
    };

    // Default path: block deletion if any matches. Mirrors L83-88.
    if !args.force && !args.dry_run && !files_using_tag.is_empty() {
        let csv = files_using_tag.join("\n  ");
        let count = files_using_tag.len();
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-tag",
            reason: format!(
                "Tag {} is used in {} feature file(s):\n  {}\n\nUse --force to delete anyway",
                args.tag, count, csv
            ),
        });
    }

    // --force path with matches: emit a non-blocking warning.
    // Mirrors L94-117.
    let warning: Option<String> = if args.force && !files_using_tag.is_empty() {
        let csv = files_using_tag.join("\n  ");
        let count = files_using_tag.len();
        Some(format!(
            "Warning: Tag {} is still used in {} file(s):\n  {}",
            args.tag, count, csv
        ))
    } else {
        None
    };

    // ---- Dry-run: report intent, no disk mutation. Mirrors L119-125 ----
    if args.dry_run {
        let message = format!(
            "Would delete tag {} from category \"{}\"",
            args.tag, current_category_name
        );
        return render_result(&args, &message, None, /* mutated_disk = */ false);
    }

    // ---- Mutate: splice from current category. Mirrors L128 ----
    tags_data.categories[current_cat_index]
        .tags
        .remove(tag_index);

    // ---- Atomic write spec/tags.json ----
    write_json_atomic(&tags_json_path, &tags_data).map_err(|e| match e {
        FspecCoreError::Io { source, .. } => FspecCoreError::Io {
            command: "delete-tag",
            source,
        },
        other => other,
    })?;

    // ---- Regenerate spec/TAGS.md ----
    let tags_md_path = project_root.join("spec").join("TAGS.md");
    let markdown = generate_tags_md(&tags_data);
    std::fs::write(&tags_md_path, markdown).map_err(|source| FspecCoreError::Io {
        command: "delete-tag",
        source,
    })?;

    // ---- Build success message ----
    let message = format!("Successfully deleted tag {} from registry", args.tag);
    render_result(
        &args,
        &message,
        warning.as_deref(),
        /* mutated_disk = */ true,
    )
}

/// Format the result either as JSON (`--format json`) or text. The
/// `mutated_disk` flag controls whether the trailing `Updated:` /
/// `Regenerated:` lines are emitted; they are suppressed for the
/// dry-run path (TS `if (!options.dryRun)` at L187-190).
fn render_result(
    args: &DeleteTagArgs,
    message: &str,
    warning: Option<&str>,
    mutated_disk: bool,
) -> Result<String, FspecCoreError> {
    match args.format.as_deref() {
        Some("json") => {
            #[derive(Serialize)]
            struct JsonResult<'a> {
                success: bool,
                message: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                warning: Option<&'a str>,
            }
            let result = JsonResult {
                success: true,
                message,
                warning,
            };
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "delete-tag",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        _ => Ok(render_text(message, warning, mutated_disk)),
    }
}

/// Render the multi-line success block emitted by the TS CLI wrapper
/// at `src/commands/delete-tag.ts:181-190`:
///
/// ```text
/// [Warning: Tag X is still used in N file(s):
///   spec/features/a.feature]
/// ✓ <message>
///   Updated: spec/tags.json
///   Regenerated: spec/TAGS.md
/// ```
///
/// When `mutated_disk` is false (dry-run path), the trailing
/// `Updated:` / `Regenerated:` lines are suppressed.
fn render_text(message: &str, warning: Option<&str>, mutated_disk: bool) -> String {
    let mut out = String::new();
    if let Some(w) = warning {
        out.push_str(w);
        if !w.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str(&format!("✓ {message}\n"));
    if mutated_disk {
        out.push_str("  Updated: spec/tags.json\n");
        out.push_str("  Regenerated: spec/TAGS.md\n");
    }
    out
}

/// Hand-rolled best-effort recursive scan of `spec/features/**/*.feature`
/// looking for byte-substring matches of `tag`. Returns paths relative
/// to `project_root` using forward slashes (mirrors TS `tinyglobby`
/// output with `absolute:false`). Order is alphabetical to keep the
/// rendered error / warning message deterministic.
///
/// Any IO failure (missing directory, permission error, unreadable
/// file) is SWALLOWED and treated as "no matches" — mirrors the TS
/// `try / catch {}` blocks at `delete-tag.ts:89-91` and `L114-116`.
fn scan_feature_files_for_tag(project_root: &Path, tag: &str) -> Option<Vec<String>> {
    let features_root = project_root.join("spec").join("features");
    if !features_root.is_dir() {
        return Some(Vec::new());
    }

    let mut hits: Vec<String> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![features_root];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(it) => it,
            Err(_) => continue, // SWALLOW — best-effort scan
        };
        for entry_result in entries {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            if path.extension().and_then(std::ffi::OsStr::to_str) != Some("feature") {
                continue;
            }
            let contents = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if contents.contains(tag) {
                let rel = path
                    .strip_prefix(project_root)
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|_| path.clone());
                // Normalise to forward slashes for cross-platform parity
                // with the TS glob output.
                let rel_str = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                hits.push(rel_str);
            }
        }
    }

    // Deterministic ordering for the rendered message.
    hits.sort();
    Some(hits)
}

/// Minimal `TAGS.md` renderer. Duplicated from `register_tag.rs` /
/// `update_tag.rs` — shared `generators` module will absorb all three
/// in a follow-up batch.
fn generate_tags_md(tags: &TagsData) -> String {
    let mut out = String::new();
    out.push_str("<!-- THIS FILE IS AUTO-GENERATED FROM spec/tags.json -->\n");
    out.push_str("<!-- DO NOT EDIT THIS FILE DIRECTLY -->\n");
    out.push_str("<!-- Edit spec/tags.json and run: fspec generate-tags -->\n");
    out.push('\n');
    out.push_str("# fspec Feature File Tag Registry\n");
    out.push('\n');

    if !tags.categories.is_empty() {
        out.push_str("## Tag Categories\n");
        out.push('\n');
        for cat in &tags.categories {
            let suffix = if cat.required { " (Required)" } else { "" };
            out.push_str(&format!("### {}{}\n", cat.name, suffix));
            out.push('\n');
            out.push_str(&cat.description);
            out.push_str("\n\n");
            if !cat.tags.is_empty() {
                out.push_str("| Tag | Description |\n");
                out.push_str("|-----|-------------|\n");
                for t in &cat.tags {
                    out.push_str(&format!("| `{}` | {} |\n", t.name, t.description));
                }
                out.push('\n');
            }
            if let Some(rule) = &cat.rule {
                out.push_str(&format!("**Rule**: {rule}\n"));
                out.push('\n');
            }
        }
        out.push_str("---\n");
        out.push('\n');
    }

    if let Some(stats) = tags.extra.get("statistics") {
        if let Some(last_updated) = stats.get("lastUpdated").and_then(serde_json::Value::as_str) {
            out.push_str(&format!("_Last updated: {last_updated}_\n"));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;
    use crate::types::tags::Tag;
    use serde_json::json;
    use tempfile::TempDir;

    fn make_args(tag: &str, force: bool, dry_run: bool) -> DeleteTagArgs {
        DeleteTagArgs {
            tag: tag.to_string(),
            force,
            dry_run,
            format: None,
        }
    }

    #[test]
    fn render_text_emits_success_block_with_trailing_lines_on_mutated_disk() {
        let out = render_text(
            "Successfully deleted tag @critical from registry",
            None,
            true,
        );
        assert!(out.contains("✓ Successfully deleted tag @critical from registry"));
        assert!(out.contains("  Updated: spec/tags.json"));
        assert!(out.contains("  Regenerated: spec/TAGS.md"));
        assert!(!out.contains("Warning:"));
    }

    #[test]
    fn render_text_suppresses_trailing_lines_for_dry_run() {
        let out = render_text(
            "Would delete tag @critical from category \"Status Tags\"",
            None,
            false,
        );
        assert!(out.contains("✓ Would delete tag @critical"));
        assert!(!out.contains("Updated: spec/tags.json"));
        assert!(!out.contains("Regenerated: spec/TAGS.md"));
    }

    #[test]
    fn render_text_prepends_warning_prefix_when_provided() {
        let warning = "Warning: Tag @critical is still used in 2 file(s):\n  spec/features/auth.feature\n  spec/features/billing.feature";
        let out = render_text(
            "Successfully deleted tag @critical from registry",
            Some(warning),
            true,
        );
        assert!(out.starts_with("Warning: Tag @critical is still used in 2 file(s):"));
        assert!(out.contains("✓ Successfully deleted tag @critical from registry"));
    }

    #[test]
    fn make_args_defaults_force_and_dry_run_to_false() {
        let a = make_args("@x", false, false);
        assert!(!a.force);
        assert!(!a.dry_run);
    }

    #[test]
    fn scan_feature_files_returns_empty_when_directory_missing() {
        let tmp = TempDir::new().expect("tempdir");
        let hits = scan_feature_files_for_tag(tmp.path(), "@x").expect("scan returns Some");
        assert!(hits.is_empty());
    }

    #[test]
    fn scan_feature_files_finds_matches_and_normalises_slashes() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("spec/features/nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            tmp.path().join("spec/features/a.feature"),
            "@critical\nFeature: A\n",
        )
        .unwrap();
        std::fs::write(nested.join("b.feature"), "@critical\nFeature: B\n").unwrap();
        std::fs::write(
            tmp.path().join("spec/features/c.feature"),
            "@other\nFeature: C\n",
        )
        .unwrap();
        let hits = scan_feature_files_for_tag(tmp.path(), "@critical").expect("scan returns Some");
        // Note: nested b.feature path should use forward slashes.
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().any(|h| h == "spec/features/a.feature"));
        assert!(hits.iter().any(|h| h == "spec/features/nested/b.feature"));
    }

    #[test]
    fn generate_tags_md_renders_header_and_table_when_present() {
        let mut data = TagsData::initial();
        data.categories[0].tags.push(Tag {
            name: "@critical".to_string(),
            description: "Critical".to_string(),
            extra: serde_json::Map::new(),
        });
        let md = generate_tags_md(&data);
        assert!(md.contains("# fspec Feature File Tag Registry"));
        assert!(md.contains("### Phase Tags (Required)"));
        assert!(md.contains("| `@critical` | Critical |"));
    }

    #[test]
    fn generate_tags_md_renders_last_updated_when_present() {
        let mut data = TagsData::initial();
        data.extra.insert(
            "statistics".to_string(),
            json!({"lastUpdated": "2026-06-10T00:00:00.000Z"}),
        );
        let md = generate_tags_md(&data);
        assert!(md.contains("_Last updated: 2026-06-10T00:00:00.000Z_"));
    }
}
