//! `remove-tag-from-feature` — Rust port of
//! `src/commands/remove-tag-from-feature.ts` (RPC-281).
//!
//! Removes one or more feature-level tags from a Gherkin feature file by
//! filtering out entire lines whose trimmed text exactly equals one of the
//! requested tag names. Mirrors the TypeScript implementation including:
//!
//!   - canonical error envelopes (`File not found`,
//!     `File does not contain a valid Feature`,
//!     `Tag <t> not found on this feature`);
//!   - whole-line equality removal (multi-tag lines like `@a @b` are NOT
//!     split — TS quirk preserved here intentionally so existing call sites
//!     get the same behaviour from either binary);
//!   - a success message of the form `Removed <tags> from <file>`.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `rust/fspec/src/remove_tag_from_feature.rs` is JSON marshalling only.

use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveTagFromFeatureArgs {
    file: String,
    tags: Vec<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveTagFromFeatureArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-tag-from-feature",
            reason: format!("failed to parse args: {e}"),
        })?;

    let file_abs = project_root.join(&args.file);

    // ---- Read feature file ----
    let content = match std::fs::read_to_string(&file_abs) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-tag-from-feature",
                reason: format!("File not found: {}", args.file),
            });
        }
        Err(source) => {
            return Err(FspecCoreError::Io {
                command: "remove-tag-from-feature",
                source,
            });
        }
    };

    // ---- Parse Gherkin (lenient parity with TS @cucumber/gherkin) ----
    let feature = match parse_feature_lenient(&content) {
        Ok(f) => f,
        Err(_) => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-tag-from-feature",
                reason: "File does not contain a valid Feature".to_string(),
            });
        }
    };

    // ---- Existing feature-level tags ----
    // Gherkin 0.16's parser strips the leading `@` from each tag, so we
    // re-prepend it to compare against the canonical `@tag` form supplied
    // by callers and used everywhere else in fspec.
    let existing: Vec<String> = feature.tags.iter().map(|t| format!("@{t}")).collect();

    // ---- Existence gate ----
    for tag in &args.tags {
        if !existing.iter().any(|e| e == tag) {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-tag-from-feature",
                reason: format!("Tag {tag} not found on this feature"),
            });
        }
    }

    // ---- Whole-line equality filter (TS parity) ----
    let to_remove: HashSet<&str> = args.tags.iter().map(String::as_str).collect();
    let mut kept: Vec<&str> = Vec::new();
    for line in content.split('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('@') && to_remove.contains(trimmed) {
            continue;
        }
        kept.push(line);
    }
    let new_content = kept.join("\n");

    // ---- Validate result is still parseable Gherkin ----
    let valid = parse_feature_lenient(&new_content).is_ok();

    // ---- Write file ----
    std::fs::write(&file_abs, &new_content).map_err(|source| FspecCoreError::Io {
        command: "remove-tag-from-feature",
        source,
    })?;

    let tag_list = args.tags.join(", ");
    let response = json!({
        "success": true,
        "valid": valid,
        "message": format!("Removed {} from {}", tag_list, args.file),
    });

    serde_json::to_string(&response).map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-tag-from-feature",
        reason: format!("failed to serialise response: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: RemoveTagFromFeatureArgs =
            serde_json::from_str(r#"{"file":"spec/features/x.feature","tags":["@wip"]}"#).unwrap();
        assert_eq!(a.file, "spec/features/x.feature");
        assert_eq!(a.tags, vec!["@wip".to_string()]);
    }
}
