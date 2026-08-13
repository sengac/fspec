//! `tag-stats` — Rust port of `src/commands/tag-stats.ts` (RPC-310).
//!
//! Aggregates feature-level tag usage across every `*.feature` file under
//! `spec/features/`, projects the counts into the categories declared by
//! the project tag registry (when present), and emits either a pretty-
//! printed JSON payload (dispatcher path) or a plain-text summary
//! (CLI path).
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.
//!
//! ## TS parity rules
//!
//! * **No auto-create** of either the tag registry or `spec/features/`.
//!   Missing OR malformed registry → `tagsFileFound=false`.
//! * **Feature-level tags only**: parity with the TS
//!   `gherkinDocument.feature.tags` projection. Scenario, scenario-outline,
//!   background and rule-nested tags are excluded.
//! * **Invalid Gherkin → invalidFiles**: a feature file whose contents
//!   contain no `Feature:` keyword is recorded under `invalidFiles` and
//!   contributes no counts.
//! * **Categories preserve declaration order**: the registry array on
//!   disk is the render order. Tags WITHIN each category sort by count
//!   descending (stable for ties).
//! * **Unregistered bucket**: tags observed in feature files that do
//!   not appear in any registered category collect under a synthetic
//!   `Unregistered` category appended after the registered ones, sorted
//!   desc-by-count.
//! * **unusedTags**: registered tags with zero occurrences,
//!   alphabetically sorted. Only populated when the registry parsed.
//! * **JSON field order**: `success, totalFiles, uniqueTags,
//!   totalOccurrences, categories, unusedTags, tagsFileFound,
//!   invalidFiles` (declaration order, NOT alphabetical).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::types::tags::TagsData;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct TagStatsArgs {
    #[serde(default)]
    format: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TagStatsResult {
    success: bool,
    total_files: usize,
    unique_tags: usize,
    total_occurrences: usize,
    categories: Vec<CategoryStats>,
    unused_tags: Vec<String>,
    tags_file_found: bool,
    invalid_files: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CategoryStats {
    name: String,
    tags: Vec<TagCount>,
}

#[derive(Debug, Serialize)]
struct TagCount {
    tag: String,
    count: usize,
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: TagStatsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "tag-stats",
            reason: format!("failed to parse args: {e}"),
        })?;

    let result = compute_stats(project_root);

    match args.format.as_deref() {
        Some("text") => Ok(render_text(&result)),
        _ => serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
            command: "tag-stats",
            reason: format!("failed to serialize result: {e}"),
        }),
    }
}

fn compute_stats(project_root: &Path) -> TagStatsResult {
    let (tags_data, tags_file_found) = load_tags_registry(project_root);

    let files = match glob_feature_files(project_root) {
        Ok(v) => v,
        Err(FspecCoreError::DirectoryNotFound { .. }) => Vec::new(),
        Err(_) => Vec::new(),
    };

    if files.is_empty() {
        return TagStatsResult {
            success: true,
            total_files: 0,
            unique_tags: 0,
            total_occurrences: 0,
            categories: Vec::new(),
            unused_tags: Vec::new(),
            tags_file_found,
            invalid_files: Vec::new(),
        };
    }

    let mut tag_counts: Vec<(String, usize)> = Vec::new();
    let mut invalid_files: Vec<String> = Vec::new();

    for rel in &files {
        let abs = project_root.join(rel);
        let content = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(_) => {
                invalid_files.push(rel.clone());
                continue;
            }
        };

        match parse_feature_tags(&content) {
            Some(tags) => {
                for t in tags {
                    bump_count(&mut tag_counts, &t);
                }
            }
            None => {
                invalid_files.push(rel.clone());
            }
        }
    }

    let unique_tags = tag_counts.len();
    let total_occurrences: usize = tag_counts.iter().map(|(_, c)| *c).sum();

    let (categories, unused_tags) =
        build_categories(&tag_counts, tags_data.as_ref(), tags_file_found);

    TagStatsResult {
        success: true,
        total_files: files.len(),
        unique_tags,
        total_occurrences,
        categories,
        unused_tags,
        tags_file_found,
        invalid_files,
    }
}

fn load_tags_registry(project_root: &Path) -> (Option<TagsData>, bool) {
    let path = project_root.join("spec").join("tags.json");
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return (None, false),
    };
    match serde_json::from_str::<TagsData>(&raw) {
        Ok(d) => (Some(d), true),
        Err(_) => (None, false),
    }
}

fn bump_count(counts: &mut Vec<(String, usize)>, tag: &str) {
    if let Some(entry) = counts.iter_mut().find(|(k, _)| k == tag) {
        entry.1 += 1;
    } else {
        counts.push((tag.to_string(), 1));
    }
}

fn build_categories(
    tag_counts: &[(String, usize)],
    tags_data: Option<&TagsData>,
    tags_file_found: bool,
) -> (Vec<CategoryStats>, Vec<String>) {
    let mut categories: Vec<CategoryStats> = Vec::new();
    let mut unused_tags: Vec<String> = Vec::new();

    let registered: Vec<String> = match tags_data {
        Some(td) => td
            .categories
            .iter()
            .flat_map(|c| c.tags.iter().map(|t| t.name.clone()))
            .collect(),
        None => Vec::new(),
    };

    if tags_file_found {
        if let Some(td) = tags_data {
            for cat in &td.categories {
                let mut category_tags: Vec<TagCount> = Vec::new();
                for tag_def in &cat.tags {
                    let count = lookup_count(tag_counts, &tag_def.name);
                    if count > 0 {
                        category_tags.push(TagCount {
                            tag: tag_def.name.clone(),
                            count,
                        });
                    }
                }
                category_tags.sort_by_key(|t| std::cmp::Reverse(t.count));
                if !category_tags.is_empty() {
                    categories.push(CategoryStats {
                        name: cat.name.clone(),
                        tags: category_tags,
                    });
                }
            }
        }

        let mut unregistered_tags: Vec<TagCount> = tag_counts
            .iter()
            .filter(|(name, _)| !registered.iter().any(|r| r == name))
            .map(|(name, count)| TagCount {
                tag: name.clone(),
                count: *count,
            })
            .collect();
        if !unregistered_tags.is_empty() {
            unregistered_tags.sort_by_key(|t| std::cmp::Reverse(t.count));
            categories.push(CategoryStats {
                name: "Unregistered".to_string(),
                tags: unregistered_tags,
            });
        }

        if let Some(td) = tags_data {
            for cat in &td.categories {
                for tag_def in &cat.tags {
                    if lookup_count(tag_counts, &tag_def.name) == 0 {
                        unused_tags.push(tag_def.name.clone());
                    }
                }
            }
            unused_tags.sort();
        }
    } else if !tag_counts.is_empty() {
        let mut all_tags: Vec<TagCount> = tag_counts
            .iter()
            .map(|(name, count)| TagCount {
                tag: name.clone(),
                count: *count,
            })
            .collect();
        all_tags.sort_by_key(|t| std::cmp::Reverse(t.count));
        categories.push(CategoryStats {
            name: "Unregistered".to_string(),
            tags: all_tags,
        });
    }

    (categories, unused_tags)
}

fn lookup_count(tag_counts: &[(String, usize)], tag: &str) -> usize {
    tag_counts
        .iter()
        .find(|(k, _)| k == tag)
        .map(|(_, c)| *c)
        .unwrap_or(0)
}

fn render_text(result: &TagStatsResult) -> String {
    let mut out = String::new();
    out.push('\n');
    out.push_str("Tag Usage Statistics\n");
    out.push_str(&"\u{2500}".repeat(50));
    out.push('\n');
    out.push_str(&format!("Total feature files: {}\n", result.total_files));
    out.push_str(&format!("Unique tags used: {}\n", result.unique_tags));
    out.push_str(&format!(
        "Total tag occurrences: {}\n",
        result.total_occurrences
    ));

    if !result.tags_file_found {
        out.push('\n');
        out.push_str("\u{26A0} Warning: spec/tags.json not found\n");
    }

    if !result.invalid_files.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "\u{26A0} Warning: {} file(s) with invalid syntax skipped:\n",
            result.invalid_files.len()
        ));
        for f in &result.invalid_files {
            out.push_str(&format!("  - {f}\n"));
        }
    }

    if !result.categories.is_empty() {
        out.push_str("\n\n");
        out.push_str("Tag Counts by Category\n");
        out.push_str(&"\u{2500}".repeat(50));
        out.push('\n');

        for category in &result.categories {
            out.push('\n');
            out.push_str(&format!(
                "{} ({} tags)\n",
                category.name,
                category.tags.len()
            ));
            for tc in &category.tags {
                let padded = pad_end(&tc.tag, 30);
                out.push_str(&format!("  {padded} {}\n", tc.count));
            }
        }
    }

    if !result.unused_tags.is_empty() {
        out.push_str("\n\n");
        out.push_str("Unused Registered Tags\n");
        out.push_str(&"\u{2500}".repeat(50));
        out.push('\n');
        out.push_str(&format!(
            "{} registered tag(s) not used in any feature file:\n\n",
            result.unused_tags.len()
        ));
        for tag in &result.unused_tags {
            out.push_str(&format!("  {tag}\n"));
        }
    }

    out.push('\n');
    out
}

fn pad_end(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.to_string()
    } else {
        let pad = width - len;
        let mut out = String::with_capacity(s.len() + pad);
        out.push_str(s);
        for _ in 0..pad {
            out.push(' ');
        }
        out
    }
}

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
            for tok in trimmed.split_whitespace() {
                if tok.starts_with('@') {
                    pending_tags.push(tok.to_string());
                }
            }
            continue;
        }
        if trimmed.starts_with("Feature:") {
            if feature_tags.is_none() {
                feature_tags = Some(std::mem::take(&mut pending_tags));
            } else {
                pending_tags.clear();
            }
            continue;
        }
        pending_tags.clear();
    }

    feature_tags
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
    fn pad_end_pads_short_strings_with_spaces() {
        assert_eq!(pad_end("abc", 5), "abc  ");
        assert_eq!(pad_end("abc", 3), "abc");
        assert_eq!(pad_end("longer", 3), "longer");
    }

    #[test]
    fn bump_count_preserves_insertion_order() {
        let mut counts: Vec<(String, usize)> = Vec::new();
        bump_count(&mut counts, "@b");
        bump_count(&mut counts, "@a");
        bump_count(&mut counts, "@b");
        assert_eq!(counts[0].0, "@b");
        assert_eq!(counts[0].1, 2);
        assert_eq!(counts[1].0, "@a");
        assert_eq!(counts[1].1, 1);
    }

    #[test]
    fn parse_feature_tags_extracts_feature_level_only() {
        let body = "@critical\nFeature: X\n\n  @smoke\n  Scenario: A\n    Given x\n";
        let tags = parse_feature_tags(body).expect("Some for valid Feature");
        assert_eq!(tags, vec!["@critical".to_string()]);
    }

    #[test]
    fn parse_feature_tags_none_when_no_feature_header() {
        assert!(parse_feature_tags("This is not gherkin at all\n").is_none());
    }
}
