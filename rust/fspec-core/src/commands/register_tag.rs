//! `register-tag` — Rust port of `src/commands/register-tag.ts` (RPC-265).
//!
//! Validates a tag name (`@lowercase-with-hyphens`), normalises uppercase
//! input to lowercase, ensures `spec/tags.json` exists (load-or-init via
//! [`crate::io::ensure::ensure_tags_file`]), rejects duplicates across
//! ALL categories, finds the target category case-insensitively, inserts
//! the new `Tag` record into that category's tags array, alphabetises
//! the array, bumps `statistics.lastUpdated`, then atomically writes
//! the merged document via [`crate::io::locked_file::write_json_atomic`].
//! Finally regenerates `spec/TAGS.md` from the in-memory `TagsData`.
//!
//! Both invocation paths — the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand — call this single function
//! (RPC-003 §7/§11 two-front-doors invariant).
//!
//! Divergences from the TS implementation (`src/commands/register-tag.ts`):
//!
//! * The TS `created` flag was a dead branch (always `false` —
//!   `ensureTagsFile` handles file creation silently). The Rust port
//!   omits it entirely.
//! * The TS rollback try/catch around `generateTagsMd` is NOT replicated.
//!   The Rust port renders the markdown deterministically from the
//!   in-memory `TagsData`; failure paths reduce to filesystem write
//!   errors, which we escalate as [`FspecCoreError::Io`].
//! * Schema validation (Ajv against `src/schemas/tags.schema.json`) is
//!   NOT replicated. The upstream gates (tag-format regex + duplicate
//!   check + category existence + `TagsData` serde shape) collectively
//!   enforce the schema's invariants.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_tags_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::tags::{Tag, TagsData};

/// CLI arguments accepted by `register-tag`. Mirrors the TS Commander.js
/// registration at `src/commands/register-tag.ts:169-176`:
/// - positional `<tag>` (required)
/// - positional `<category>` (required)
/// - positional `<description>` (required)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterTagArgs {
    /// Tag name, e.g. `"@my-tag"`. Must start with `@` and (after
    /// normalisation) match `^@[a-z0-9-]+$`.
    tag: String,
    /// Category display name (case-insensitive match against
    /// `tagsData.categories[*].name`).
    category: String,
    /// Tag description string.
    description: String,
    /// `"text"` (default) or `"json"`. Dispatcher-only — the shell CLI
    /// does not advertise `--format` (matching the flag-less TS
    /// Commander.js registration).
    #[serde(default)]
    format: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call
/// `std::env::current_dir()` so the same binary can serve multiple
/// sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RegisterTagArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "register-tag",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Tag format validation (two gates) ----

    // Gate 1: must start with '@'. Mirrors src/commands/register-tag.ts:37-41.
    // Note: uses the ORIGINAL (pre-normalisation) input in the error msg.
    if !args.tag.starts_with('@') {
        return Err(FspecCoreError::InvalidArgs {
            command: "register-tag",
            reason: format!(
                "Invalid tag format: \"{}\". Valid format is @lowercase-with-hyphens",
                args.tag
            ),
        });
    }

    // Normalise uppercase characters to lowercase. Mirrors L44-47.
    let normalised = args.tag.to_lowercase();
    let converted = normalised != args.tag;

    // Gate 2: post-normalisation regex `^@[a-z0-9-]+$`.
    // Mirrors L50-54 — error uses the ORIGINAL input.
    if !is_valid_tag(&normalised) {
        return Err(FspecCoreError::InvalidArgs {
            command: "register-tag",
            reason: format!(
                "Invalid tag format: \"{}\". Valid format is @lowercase-with-hyphens",
                args.tag
            ),
        });
    }

    // ---- Load tags.json (load-or-init) ----
    let mut tags_data: TagsData = ensure_tags_file(project_root)?;

    // ---- Duplicate detection (scan ALL categories) ----
    // Mirrors L61-68.
    for cat in &tags_data.categories {
        if cat.tags.iter().any(|t| t.name == normalised) {
            return Err(FspecCoreError::InvalidArgs {
                command: "register-tag",
                reason: format!("Tag {} is already registered in {}", normalised, cat.name),
            });
        }
    }

    // ---- Category lookup (case-insensitive) ----
    // Mirrors L71-80.
    let target_index = tags_data
        .categories
        .iter()
        .position(|c| c.name.to_lowercase() == args.category.to_lowercase());

    let target_index = match target_index {
        Some(i) => i,
        None => {
            let available = tags_data
                .categories
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(FspecCoreError::InvalidArgs {
                command: "register-tag",
                reason: format!(
                    "Invalid category: \"{}\". Available categories: {}",
                    args.category, available
                ),
            });
        }
    };

    // ---- Insert and sort within target category ----
    // Mirrors L83-91.
    let canonical_category_name = tags_data.categories[target_index].name.clone();
    let new_tag = Tag {
        name: normalised.clone(),
        description: args.description.clone(),
        extra: serde_json::Map::new(),
    };
    tags_data.categories[target_index].tags.push(new_tag);
    tags_data.categories[target_index]
        .tags
        .sort_by(|a, b| a.name.cmp(&b.name));

    // ---- Bump statistics.lastUpdated ----
    // Mirrors L94. The `statistics` object lives in the `extra` map
    // (TagsData only models `categories` explicitly; everything else
    // round-trips through `#[serde(flatten)] extra`).
    let now = iso8601_now();
    bump_statistics_last_updated(&mut tags_data.extra, &now);

    // ---- Atomic write spec/tags.json ----
    let tags_json_path = project_root.join("spec").join("tags.json");
    write_json_atomic(&tags_json_path, &tags_data).map_err(|e| match e {
        FspecCoreError::Io { source, .. } => FspecCoreError::Io {
            command: "register-tag",
            source,
        },
        other => other,
    })?;

    // ---- Regenerate spec/TAGS.md ----
    let tags_md_path = project_root.join("spec").join("TAGS.md");
    let markdown = generate_tags_md(&tags_data);
    std::fs::write(&tags_md_path, markdown).map_err(|source| FspecCoreError::Io {
        command: "register-tag",
        source,
    })?;

    // ---- Build the success message ----
    let message = if converted {
        format!(
            "Successfully registered {} (converted from {}) in {}",
            normalised, args.tag, canonical_category_name
        )
    } else {
        format!("Successfully registered {normalised} in {canonical_category_name}")
    };

    // ---- Render result ----
    match args.format.as_deref() {
        Some("json") => {
            #[derive(Serialize)]
            struct JsonResult<'a> {
                success: bool,
                message: &'a str,
                tag: &'a str,
                category: &'a str,
                converted: bool,
            }
            let result = JsonResult {
                success: true,
                message: &message,
                tag: &normalised,
                category: &canonical_category_name,
                converted,
            };
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "register-tag",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        _ => Ok(render_text(&message, &args.tag, converted)),
    }
}

/// Validate the normalised tag against `^@[a-z0-9-]+$`. Hand-rolled to
/// avoid a `regex` runtime dependency.
fn is_valid_tag(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some('@') => {}
        _ => return false,
    }
    let mut count = 0_usize;
    for c in chars {
        count += 1;
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return false;
        }
    }
    count >= 1
}

/// Mutate `extra["statistics"]["lastUpdated"]` to `now`. If
/// `statistics` is missing, create a minimal object containing only
/// `lastUpdated` (the TS code expects the field to exist; canonical
/// initial tags.json always carries it).
fn bump_statistics_last_updated(extra: &mut serde_json::Map<String, Value>, now: &str) {
    let stats_entry = extra
        .entry("statistics".to_string())
        .or_insert_with(|| json!({}));
    if let Value::Object(map) = stats_entry {
        map.insert("lastUpdated".to_string(), Value::String(now.to_string()));
    } else {
        // Statistics is present but not an object — overwrite with a
        // single-field object so we don't lose the bump.
        *stats_entry = json!({ "lastUpdated": now });
    }
}

/// Render the multi-line success block emitted by the TS CLI wrapper
/// at `src/commands/register-tag.ts:147-161`:
///
/// ```text
/// [Note: Tag converted to lowercase: <orig> → <lower>]
/// ✓ <message>
///   Updated: spec/tags.json
///   Regenerated: spec/TAGS.md
/// ```
fn render_text(message: &str, original: &str, converted: bool) -> String {
    let mut out = String::new();
    if converted {
        out.push_str(&format!(
            "Note: Tag converted to lowercase: {} → {}\n",
            original,
            original.to_lowercase()
        ));
    }
    out.push_str(&format!("✓ {message}\n"));
    out.push_str("  Updated: spec/tags.json\n");
    out.push_str("  Regenerated: spec/TAGS.md\n");
    out
}

/// Minimal `TAGS.md` renderer. Covers the auto-generation warning
/// header, the `## Tag Categories` block (one section per category with
/// per-tag table), and a trailing `_Last updated: <ts>_` line when
/// present in `statistics`. Auxiliary sections (combinationExamples,
/// usageGuidelines, addingNewTags, queries, validation, references) are
/// intentionally omitted — for the canonical-default tags.json all such
/// sections render empty in the TS implementation, so omission is
/// byte-equivalent for the fresh-init case.
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
        if let Some(last_updated) = stats.get("lastUpdated").and_then(Value::as_str) {
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

    #[test]
    fn is_valid_tag_accepts_basic_at_lowercase() {
        assert!(is_valid_tag("@api"));
        assert!(is_valid_tag("@a"));
        assert!(is_valid_tag("@api-integration"));
        assert!(is_valid_tag("@a1"));
        assert!(is_valid_tag("@x-y-z"));
    }

    #[test]
    fn is_valid_tag_rejects_missing_at_or_uppercase_or_underscore() {
        assert!(!is_valid_tag("api"));
        assert!(!is_valid_tag("@"));
        assert!(!is_valid_tag("@API"));
        assert!(!is_valid_tag("@x_y"));
        assert!(!is_valid_tag("@x y"));
        assert!(!is_valid_tag("@x.y"));
    }

    #[test]
    fn render_text_omits_note_when_not_converted() {
        let out = render_text(
            "Successfully registered @api in Technical Tags",
            "@api",
            false,
        );
        assert!(!out.contains("Note: Tag converted"));
        assert!(out.contains("✓ Successfully registered @api in Technical Tags"));
        assert!(out.contains("  Updated: spec/tags.json"));
        assert!(out.contains("  Regenerated: spec/TAGS.md"));
    }

    #[test]
    fn render_text_includes_note_when_converted() {
        let out = render_text(
            "Successfully registered @api-integration (converted from @API-Integration) in Technical Tags",
            "@API-Integration",
            true,
        );
        assert!(
            out.contains("Note: Tag converted to lowercase: @API-Integration → @api-integration")
        );
    }

    #[test]
    fn bump_statistics_creates_object_if_missing() {
        let mut extra = serde_json::Map::new();
        bump_statistics_last_updated(&mut extra, "2026-06-10T00:00:00.000Z");
        let stats = extra.get("statistics").unwrap().as_object().unwrap();
        assert_eq!(
            stats.get("lastUpdated").unwrap().as_str(),
            Some("2026-06-10T00:00:00.000Z")
        );
    }

    #[test]
    fn bump_statistics_preserves_other_fields() {
        let mut extra = serde_json::Map::new();
        extra.insert(
            "statistics".to_string(),
            json!({
                "lastUpdated": "1999-01-01T00:00:00.000Z",
                "phaseStats": [],
                "updateCommand": "fspec tag-stats"
            }),
        );
        bump_statistics_last_updated(&mut extra, "2026-06-10T00:00:00.000Z");
        let stats = extra.get("statistics").unwrap().as_object().unwrap();
        assert_eq!(
            stats.get("lastUpdated").unwrap().as_str(),
            Some("2026-06-10T00:00:00.000Z")
        );
        assert!(stats.get("phaseStats").is_some());
        assert_eq!(
            stats.get("updateCommand").unwrap().as_str(),
            Some("fspec tag-stats")
        );
    }

    #[test]
    fn generate_tags_md_renders_header_and_categories() {
        let data = TagsData::initial();
        let md = generate_tags_md(&data);
        assert!(md.contains("<!-- THIS FILE IS AUTO-GENERATED FROM spec/tags.json -->"));
        assert!(md.contains("# fspec Feature File Tag Registry"));
        assert!(md.contains("## Tag Categories"));
        assert!(md.contains("### Phase Tags (Required)"));
        assert!(md.contains("### Automation Tags"));
        // Empty categories render no table.
        assert!(!md.contains("| Tag | Description |"));
    }

    #[test]
    fn generate_tags_md_renders_tag_table_when_tags_present() {
        let mut data = TagsData::initial();
        data.categories[0].tags.push(Tag {
            name: "@critical".to_string(),
            description: "Critical features".to_string(),
            extra: serde_json::Map::new(),
        });
        let md = generate_tags_md(&data);
        assert!(md.contains("| Tag | Description |"));
        assert!(md.contains("| `@critical` | Critical features |"));
    }
}
