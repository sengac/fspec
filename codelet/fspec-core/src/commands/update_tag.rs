//! `update-tag` — Rust port of `src/commands/update-tag.ts` (RPC-316).
//!
//! Locates a tag by exact-match name across all categories, then either
//! mutates its description in place (description-only update) OR moves it
//! to a different category (with optional description override),
//! preserving auxiliary top-level fields via `#[serde(flatten)] extra`.
//! Atomically writes the merged document via
//! [`crate::io::locked_file::write_json_atomic`] and regenerates
//! `spec/TAGS.md` from the in-memory `TagsData`.
//!
//! Both invocation paths — the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand — call this single function
//! (RPC-003 §7/§11 two-front-doors invariant).
//!
//! Divergences from the TS implementation (`src/commands/update-tag.ts`):
//!
//! * The TS `existsSync` check on `spec/tags.json` is preserved verbatim
//!   — update-tag does NOT auto-create the file (opposite of
//!   `register-tag`). Missing file → "spec/tags.json not found" error.
//! * Ajv schema validation against `src/schemas/tags.schema.json` is
//!   NOT replicated. Upstream gates (tag lookup + category existence +
//!   `TagsData` serde shape) collectively enforce the schema invariants.
//! * `statistics.lastUpdated` is NOT bumped — TS code has no
//!   `lastUpdated` mutation in `update-tag.ts`. Only `register-tag`
//!   bumps it.
//! * Outer try/catch is flattened into direct
//!   [`FspecCoreError`] returns for parse / IO failures.
//! * Inline minimal `generate_tags_md` helper duplicated from
//!   `register_tag.rs`. Will be promoted to a shared `generators`
//!   module when `delete_tag` lands (also Batch 7).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::types::tags::TagsData;

/// CLI arguments accepted by `update-tag`. Mirrors the TS Commander.js
/// registration at `src/commands/update-tag.ts:165-172`:
/// - positional `<tag>` (required)
/// - `--category <category>` (optional)
/// - `--description <description>` (optional)
/// - `--format` is dispatcher-only (not advertised by the clap surface).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTagArgs {
    tag: String,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    format: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call
/// `std::env::current_dir()` so the same binary can serve multiple
/// sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdateTagArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "update-tag",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Gate 1: at-least-one-update check ----
    // Mirrors src/commands/update-tag.ts:33-38.
    let category_arg = args.category.as_deref().filter(|s| !s.is_empty());
    let description_arg = args.description.as_deref().filter(|s| !s.is_empty());
    if category_arg.is_none() && description_arg.is_none() {
        return Err(FspecCoreError::InvalidArgs {
            command: "update-tag",
            reason: "No updates specified. Use --category and/or --description".to_string(),
        });
    }

    // ---- Gate 2: spec/tags.json must exist ----
    // Mirrors L41-46 — update-tag does NOT auto-create (opposite of register-tag).
    let tags_json_path = project_root.join("spec").join("tags.json");
    if !tags_json_path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "update-tag",
            reason: "spec/tags.json not found".to_string(),
        });
    }

    // ---- Load tags.json ----
    let raw =
        std::fs::read_to_string(&tags_json_path).map_err(|source| FspecCoreError::Io {
            command: "update-tag",
            source,
        })?;
    let mut tags_data: TagsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "tags.json".to_string(),
            reason: e.to_string(),
        })?;

    // ---- Locate the tag (linear scan, case-sensitive exact-match) ----
    // Mirrors L53-64.
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
            // Mirrors L66-71.
            return Err(FspecCoreError::InvalidArgs {
                command: "update-tag",
                reason: format!("Tag {} not found in registry", args.tag),
            });
        }
    };

    // Capture the original description before any mutation so we can
    // fall back to it when moving categories without an explicit
    // --description override (mirrors L73 + L93).
    let original_description = tags_data.categories[current_cat_index].tags[tag_index]
        .description
        .clone();

    // ---- Branch: category change vs description-only ----
    let current_category_name = tags_data.categories[current_cat_index].name.clone();

    let moving_category = category_arg
        .map(|c| c != current_category_name)
        .unwrap_or(false);

    if moving_category {
        // Cross-category move. Mirrors L76-97.
        let target_name = category_arg.expect("category_arg present when moving");
        let target_index = tags_data
            .categories
            .iter()
            .position(|c| c.name == target_name);
        let target_index = match target_index {
            Some(i) => i,
            None => {
                // Mirrors L77-85.
                let available = tags_data
                    .categories
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(FspecCoreError::InvalidArgs {
                    command: "update-tag",
                    reason: format!(
                        "Invalid category: {target_name}. Available categories: {available}"
                    ),
                });
            }
        };

        // Remove from current. Mirrors L88.
        let mut removed = tags_data.categories[current_cat_index].tags.remove(tag_index);

        // Decide final description. Mirrors L91-94.
        let final_description = description_arg
            .map(|s| s.to_string())
            .unwrap_or(original_description);
        removed.description = final_description;
        removed.extra = serde_json::Map::new(); // Match TS: TS pushes a fresh
                                                // object literal {name, description} only.

        // Push into target. Mirrors L91-95.
        tags_data.categories[target_index].tags.push(removed);

        // Sort target alphabetically. Mirrors L97.
        tags_data.categories[target_index]
            .tags
            .sort_by(|a, b| a.name.cmp(&b.name));
    } else {
        // Description-only update path. Mirrors L98-103.
        // Only mutate if --description provided (the gate above guarantees
        // at least one of category/description is set; if category was
        // provided but equals current, we still allow description=None
        // here as a no-op — but only if description is also set; this
        // branch only fires when description IS set or category equals
        // current. The TS code is: "if (description) {...}").
        if let Some(new_desc) = description_arg {
            tags_data.categories[current_cat_index].tags[tag_index].description =
                new_desc.to_string();
        }
    }

    // ---- Atomic write spec/tags.json ----
    write_json_atomic(&tags_json_path, &tags_data).map_err(|e| match e {
        FspecCoreError::Io { source, .. } => FspecCoreError::Io {
            command: "update-tag",
            source,
        },
        other => other,
    })?;

    // ---- Regenerate spec/TAGS.md ----
    let tags_md_path = project_root.join("spec").join("TAGS.md");
    let markdown = generate_tags_md(&tags_data);
    std::fs::write(&tags_md_path, markdown).map_err(|source| FspecCoreError::Io {
        command: "update-tag",
        source,
    })?;

    // ---- Build success message ----
    let message = format!("Successfully updated {}", args.tag);

    match args.format.as_deref() {
        Some("json") => {
            #[derive(Serialize)]
            struct JsonResult<'a> {
                success: bool,
                message: &'a str,
                tag: &'a str,
            }
            let result = JsonResult {
                success: true,
                message: &message,
                tag: &args.tag,
            };
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "update-tag",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        _ => Ok(render_text(&message)),
    }
}

/// Render the multi-line success block emitted by the TS CLI wrapper
/// at `src/commands/update-tag.ts:155-157`:
///
/// ```text
/// ✓ <message>
///   Updated: spec/tags.json
///   Regenerated: spec/TAGS.md
/// ```
fn render_text(message: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("✓ {message}\n"));
    out.push_str("  Updated: spec/tags.json\n");
    out.push_str("  Regenerated: spec/TAGS.md\n");
    out
}

/// Minimal `TAGS.md` renderer. Duplicated from `register_tag.rs` —
/// shared `generators` module will absorb both when `delete-tag` lands.
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
        if let Some(last_updated) = stats.get("lastUpdated").and_then(|v| v.as_str()) {
            out.push_str(&format!("_Last updated: {last_updated}_\n"));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use crate::types::tags::Tag;
    use serde_json::json;

    #[test]
    fn render_text_emits_three_lines() {
        let out = render_text("Successfully updated @critical");
        assert!(out.contains("✓ Successfully updated @critical"));
        assert!(out.contains("  Updated: spec/tags.json"));
        assert!(out.contains("  Regenerated: spec/TAGS.md"));
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
