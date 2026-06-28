//! `delete-features` — Rust port of `src/commands/delete-features-by-tag.ts`
//! (RPC-218; the registered command name is `delete-features`).
//!
//! Bulk-deletes feature files whose FEATURE-level tags match ALL of the
//! supplied tags (AND logic). The recursive `spec/features` walk reuses
//! [`crate::io::feature_glob::glob_feature_files`] (relative forward-slash
//! paths, alphabetical sort); a `DirectoryNotFound` result is mapped to an
//! empty list so the canonical TS `"No feature files found"` message is
//! preserved when `spec/features/` is absent.
//!
//! Feature-level tags come from `parse_feature_lenient(feature.tags)` with
//! the leading `@` re-prepended (gherkin-0.16 strips it). Files with invalid
//! Gherkin or no Feature are silently skipped. With `dryRun=true` nothing is
//! unlinked; otherwise every matching file is removed. Coverage sidecars are
//! NOT deleted.
//!
//! ## Result envelope
//! ALL outcomes (including the empty-tag-list rejection) are returned as the
//! inner JSON envelope `{success, deletedCount, message?, files?, error?}`
//! via `Ok(json)` — the dispatcher derives `success=false` from the payload
//! itself (the test reads `data["error"]`), and the CLI bridge owns every
//! rendering decision (dry-run / real / empty / error).
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/delete_features.rs` is JSON marshalling + rendering.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DeleteFeaturesArgs {
    tags: Vec<String>,
    dry_run: bool,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DeleteFeaturesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "delete-features",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Require at least one tag (TS parity, lines 29-36) ----
    if args.tags.is_empty() {
        return ok(json!({
            "success": false,
            "deletedCount": 0,
            "error": "At least one --tag is required",
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
            "deletedCount": 0,
            "message": "No feature files found",
        }));
    }

    // ---- Find files matching ALL tags (AND logic) ----
    let mut matching: Vec<String> = Vec::new();
    for file in &files {
        let abs = project_root.join(file);
        let content = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let feature = match parse_feature_lenient(&content) {
            Ok(f) => f,
            Err(_) => continue, // skip invalid Gherkin
        };
        // gherkin-0.16 strips the leading '@'; re-prepend for comparison.
        let feature_tags: Vec<String> = feature.tags.iter().map(|t| format!("@{t}")).collect();
        let has_all = args.tags.iter().all(|t| feature_tags.contains(t));
        if has_all {
            matching.push(file.clone());
        }
    }

    // ---- No matches ----
    if matching.is_empty() {
        return ok(json!({
            "success": true,
            "deletedCount": 0,
            "message": "No feature files found matching tags",
        }));
    }

    // ---- Match tinyglobby's enumeration order ----
    // `glob_feature_files` returns paths sorted purely lexicographically,
    // but the TS reference iterates `glob(['spec/features/**/*.feature'])`
    // whose tinyglobby ordering is a depth-first walk: within each
    // directory the FILES (sorted) are emitted BEFORE descending into the
    // sorted subdirectories. The `files` array (and the rendered list)
    // preserves that traversal order, so re-sort the matched subset with a
    // comparator that reproduces "files-before-subdirs, alpha within each".
    //
    // Comparator: walk both paths segment-by-segment. At the first segment
    // where they differ, the path that TERMINATES (i.e. the differing
    // segment is its last → it is a file directly in the common parent)
    // sorts before the one that continues into a subdirectory. When both
    // terminate or both continue, compare the differing segment
    // lexicographically.
    matching.sort_by(|a, b| tinyglobby_order(a, b));

    let count = matching.len();

    // ---- Dry-run: report without deleting ----
    if args.dry_run {
        return ok(json!({
            "success": true,
            "deletedCount": count,
            "message": format!("Would delete {count} feature file(s)"),
            "files": matching,
        }));
    }

    // ---- Perform deletions ----
    for file in &matching {
        let abs = project_root.join(file);
        std::fs::remove_file(&abs).map_err(|source| FspecCoreError::Io {
            command: "delete-features",
            source,
        })?;
    }

    ok(json!({
        "success": true,
        "deletedCount": count,
        "message": format!("Deleted {count} feature file(s)"),
        "files": matching,
    }))
}

/// Serialise an inner-envelope value to the `Ok(String)` returned by every
/// dispatcher entry point.
fn ok(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "delete-features",
        reason: format!("failed to serialise response: {e}"),
    })
}

/// Order two forward-slash relative paths the way tinyglobby's depth-first
/// walk emits them: within any directory, files (alphabetical) come BEFORE
/// the directory's subdirectories (alphabetical), recursively.
///
/// We compare segment-by-segment. At the first differing segment, if that
/// segment is the LAST one of path `a` (so `a` is a file in the shared parent)
/// but NOT the last of `b` (so `b` descends into a subdirectory), `a` sorts
/// first, and vice versa. Otherwise (both terminate here, or both continue)
/// the segments are compared lexicographically.
///
/// Equal common prefixes fall through to comparing remaining length.
fn tinyglobby_order(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let a_segs: Vec<&str> = a.split('/').collect();
    let b_segs: Vec<&str> = b.split('/').collect();

    let common = a_segs.len().min(b_segs.len());
    for i in 0..common {
        if a_segs[i] != b_segs[i] {
            let a_is_file_here = i + 1 == a_segs.len();
            let b_is_file_here = i + 1 == b_segs.len();
            return match (a_is_file_here, b_is_file_here) {
                (true, false) => Ordering::Less, // file before subdir
                (false, true) => Ordering::Greater,
                _ => a_segs[i].cmp(b_segs[i]),
            };
        }
    }
    // One path is a prefix of the other (shouldn't happen for distinct
    // files, but stay total): shorter (shallower) first.
    a_segs.len().cmp(&b_segs.len())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: DeleteFeaturesArgs =
            serde_json::from_str(r#"{"tags":["@a","@b"],"dryRun":true}"#).unwrap();
        assert_eq!(a.tags, vec!["@a".to_string(), "@b".to_string()]);
        assert!(a.dry_run);
    }

    #[test]
    fn args_default_empty() {
        let a: DeleteFeaturesArgs = serde_json::from_str("{}").unwrap();
        assert!(a.tags.is_empty());
        assert!(!a.dry_run);
    }

    #[test]
    fn tinyglobby_order_files_before_subdirs() {
        // Top-level file `mmm` sorts before subdir files `aaa/..`, `zzz/..`.
        let mut v = vec![
            "spec/features/aaa/file.feature".to_string(),
            "spec/features/mmm.feature".to_string(),
            "spec/features/zzz/file.feature".to_string(),
        ];
        v.sort_by(|a, b| tinyglobby_order(a, b));
        assert_eq!(
            v,
            vec![
                "spec/features/mmm.feature",
                "spec/features/aaa/file.feature",
                "spec/features/zzz/file.feature",
            ]
        );
    }

    #[test]
    fn tinyglobby_order_dfs_nested() {
        let mut v = vec![
            "spec/features/d1/d2/two.feature".to_string(),
            "spec/features/d1/one.feature".to_string(),
            "spec/features/d1/aaa.feature".to_string(),
            "spec/features/top.feature".to_string(),
        ];
        v.sort_by(|a, b| tinyglobby_order(a, b));
        assert_eq!(
            v,
            vec![
                "spec/features/top.feature",
                "spec/features/d1/aaa.feature",
                "spec/features/d1/one.feature",
                "spec/features/d1/d2/two.feature",
            ]
        );
    }
}
