//! `retag` — Rust port of `src/commands/retag.ts` (RPC-293).
//!
//! Bulk-renames a tag (`--from` → `--to`) across every
//! `spec/features/**/*.feature` file. Matching is a whole-token text replace:
//! the `from` tag must be preceded by start-of-line or whitespace and followed
//! by whitespace or end-of-line (parity with the TS
//! `new RegExp('(^|\\s)' + escapedFrom + '(?=\\s|$)', 'gm')`). The preceding
//! whitespace is preserved and only the tag bytes are rewritten (TS `$1${to}`).
//!
//! The recursive `spec/features` walk reuses
//! [`crate::io::feature_glob::glob_feature_files`]; a `DirectoryNotFound`
//! result is mapped to an empty list so the canonical TS `"No feature files
//! found"` message is preserved when `spec/features/` is absent. After each
//! rename the new content is re-parsed with
//! [`crate::io::gherkin::parse_feature_lenient`] (parity with the TS Gherkin
//! re-parse guard) before the file is written.
//!
//! ## Result envelope
//! Like `delete_features.rs` (RPC-218), retag.ts RETURNS a `RetagResult`
//! object (never throws), so EVERY business outcome — including the
//! validation rejections and the not-found case — is returned as the inner
//! JSON envelope `{success, fileCount, occurrenceCount, message?, files?,
//! error?}` via `Ok(json)`. The dispatcher envelope therefore succeeds; the
//! real outcome is carried by the inner `success`/`error` fields, and the CLI
//! bridge owns every rendering decision.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `rust/fspec/src/retag.rs` is JSON marshalling + rendering only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RetagArgs {
    from: Option<String>,
    to: Option<String>,
    dry_run: bool,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RetagArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "retag",
            reason: format!("failed to parse args: {e}"),
        })?;

    let from = args.from.unwrap_or_default();
    let to = args.to.unwrap_or_default();

    // ---- Require both --from and --to (TS parity, lines 30-37) ----
    // TS treats empty strings as falsy via `!from || !to`.
    if from.is_empty() || to.is_empty() {
        return ok(json!({
            "success": false,
            "fileCount": 0,
            "occurrenceCount": 0,
            "error": "Both --from and --to are required",
        }));
    }

    // ---- Validate target tag format (TS parity, lines 40-56) ----
    if !to.starts_with('@') || !valid_to_tag(&to) {
        return ok(json!({
            "success": false,
            "fileCount": 0,
            "occurrenceCount": 0,
            "error": format!(
                "Invalid tag format: \"{to}\". Valid format is @lowercase-with-hyphens"
            ),
        }));
    }

    // ---- Enumerate feature files (DirectoryNotFound → empty list) ----
    let files = match glob_feature_files(project_root) {
        Ok(f) => f,
        Err(FspecCoreError::DirectoryNotFound { .. }) => Vec::new(),
        Err(other) => return Err(other),
    };

    if files.is_empty() {
        return ok(json!({
            "success": true,
            "fileCount": 0,
            "occurrenceCount": 0,
            "message": "No feature files found",
        }));
    }

    // ---- Find all files containing the 'from' tag ----
    let mut matching: Vec<(String, usize)> = Vec::new();
    for file in &files {
        let abs = project_root.join(file);
        let content = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(source) => {
                return Err(FspecCoreError::Io {
                    command: "retag",
                    source,
                })
            }
        };
        let occurrences = count_tag_matches(&content, &from);
        if occurrences > 0 {
            matching.push((file.clone(), occurrences));
        }
    }

    // ---- No files contain the tag ----
    if matching.is_empty() {
        return ok(json!({
            "success": false,
            "fileCount": 0,
            "occurrenceCount": 0,
            "error": format!("Tag {from} not found in any feature files"),
        }));
    }

    let total_occurrences: usize = matching.iter().map(|(_, n)| n).sum();
    let file_count = matching.len();
    let matched_files: Vec<String> = matching.iter().map(|(f, _)| f.clone()).collect();

    // ---- Dry run: report without modifying ----
    if args.dry_run {
        return ok(json!({
            "success": true,
            "fileCount": file_count,
            "occurrenceCount": total_occurrences,
            "message": format!(
                "Would rename {from} to {to} in {file_count} file(s) ({total_occurrences} occurrence(s))"
            ),
            "files": matched_files,
        }));
    }

    // ---- Perform renaming ----
    for (file, _) in &matching {
        let abs = project_root.join(file);
        let content = std::fs::read_to_string(&abs).map_err(|source| FspecCoreError::Io {
            command: "retag",
            source,
        })?;
        let new_content = replace_tags(&content, &from, &to);

        // Validate the new content is still parseable Gherkin (TS re-parse
        // guard, lines 132-146).
        if let Err(e) = parse_feature_lenient(&new_content) {
            return ok(json!({
                "success": false,
                "fileCount": 0,
                "occurrenceCount": 0,
                "error": format!("Validation failed after renaming in {file}: {e}"),
            }));
        }

        std::fs::write(&abs, new_content.as_bytes()).map_err(|source| FspecCoreError::Io {
            command: "retag",
            source,
        })?;
    }

    ok(json!({
        "success": true,
        "fileCount": file_count,
        "occurrenceCount": total_occurrences,
        "message": format!(
            "Renamed {from} to {to} in {file_count} file(s) ({total_occurrences} occurrence(s)). \
             All modified files validated successfully."
        ),
        "files": matched_files,
    }))
}

/// Serialise an inner-envelope value to the `Ok(String)` returned by the
/// dispatcher entry point.
fn ok(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "retag",
        reason: format!("failed to serialise response: {e}"),
    })
}

/// `^@[a-z0-9-#]+$` — the TS target-tag format check.
fn valid_to_tag(to: &str) -> bool {
    let rest = match to.strip_prefix('@') {
        Some(r) => r,
        None => return false,
    };
    !rest.is_empty()
        && rest
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'#')
}

/// Byte-start indices of every whole-token occurrence of `from` in `content`,
/// reproducing the `(^|\s)<from>(?=\s|$)` global multiline regex: each match
/// is preceded by start-of-string or a whitespace char and followed by a
/// whitespace char or end-of-string. The regex restarts at the END of the tag
/// (the trailing-boundary lookahead is non-consuming), so a whitespace char
/// can simultaneously be one match's lookahead and the next match's leading
/// boundary.
fn tag_match_starts(content: &str, from: &str) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    if from.is_empty() {
        return out;
    }
    let flen = from.len();
    let mut i = 0;
    while let Some(rel) = content[i..].find(from) {
        let start = i + rel;
        let end = start + flen;
        let prev_ok = start == 0
            || content[..start]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace);
        let next_ok = end == content.len()
            || content[end..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace);
        if prev_ok && next_ok {
            out.push(start);
            i = end;
        } else {
            // Advance one char past `start` to keep scanning.
            let step = content[start..].chars().next().map_or(1, char::len_utf8);
            i = start + step;
        }
    }
    out
}

/// Count whole-token occurrences of `from`.
fn count_tag_matches(content: &str, from: &str) -> usize {
    tag_match_starts(content, from).len()
}

/// Replace every whole-token occurrence of `from` with `to`, preserving all
/// surrounding bytes (parity with the TS `$1${to}` replacement).
fn replace_tags(content: &str, from: &str, to: &str) -> String {
    let starts = tag_match_starts(content, from);
    if starts.is_empty() {
        return content.to_string();
    }
    let flen = from.len();
    let mut result = String::with_capacity(content.len());
    let mut last = 0;
    for &start in &starts {
        result.push_str(&content[last..start]);
        result.push_str(to);
        last = start + flen;
    }
    result.push_str(&content[last..]);
    result
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: RetagArgs =
            serde_json::from_str(r#"{"from":"@a","to":"@b","dryRun":true}"#).unwrap();
        assert_eq!(a.from.as_deref(), Some("@a"));
        assert_eq!(a.to.as_deref(), Some("@b"));
        assert!(a.dry_run);
    }

    #[test]
    fn valid_to_tag_accepts_and_rejects() {
        assert!(valid_to_tag("@in-progress"));
        assert!(valid_to_tag("@phase-1"));
        assert!(valid_to_tag("@a#b"));
        assert!(!valid_to_tag("WIP"));
        assert!(!valid_to_tag("@"));
        assert!(!valid_to_tag("@UPPER"));
    }

    #[test]
    fn counts_whole_token_only() {
        // "@wip" appears as a token twice, plus "@wipe" should NOT match.
        let content = "@wip\nFeature: X\n  @wip\n  @wipe matters not\n";
        assert_eq!(count_tag_matches(content, "@wip"), 2);
    }

    #[test]
    fn counts_adjacent_tags_separated_by_single_space() {
        assert_eq!(count_tag_matches("@a @a @a", "@a"), 3);
    }

    #[test]
    fn replace_preserves_surrounding_whitespace() {
        let content = "  @wip @smoke\n";
        assert_eq!(
            replace_tags(content, "@wip", "@in-progress"),
            "  @in-progress @smoke\n"
        );
    }

    #[test]
    fn replace_does_not_touch_substring_matches() {
        let content = "@wipe @wip\n";
        assert_eq!(replace_tags(content, "@wip", "@x"), "@wipe @x\n");
    }
}
