//! Prefill gate for `update-work-unit-status`: linked feature files must not contain placeholder text.

use std::path::Path;

use thiserror::Error;

use super::transitions::msg;
use crate::error::FspecCoreError;

/// BUG-360/BUG-378: why the feature scan could not read `spec/features`.
/// Every variant names the path, so the status move fails with an
/// actionable message instead of passing its gates vacuously.
#[derive(Debug, Error)]
pub(super) enum FeatureScanError {
    #[error("cannot read spec/features: {source}")]
    Dir { source: std::io::Error },
    #[error("cannot read an entry of spec/features: {source}")]
    Entry { source: std::io::Error },
    #[error("cannot read {path}: {source}")]
    File {
        path: String,
        source: std::io::Error,
    },
}

/// Read every `.feature` file under `spec/features` tagged with `@<id>`.
/// Returns `(path, contents)` pairs. A missing `spec/features` means no
/// feature files; any other directory, entry or file read failure is an
/// error naming the path (BUG-378), so no gate passes vacuously.
pub(super) fn linked_feature_files(
    project_root: &Path,
    id: &str,
) -> Result<Vec<(std::path::PathBuf, String)>, FeatureScanError> {
    let dir = crate::io::spec_paths::features_dir_of(project_root);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(FeatureScanError::Dir { source }),
    };
    let mut out = Vec::new();
    let tag = format!("@{id}");
    for entry in entries {
        let path = entry
            .map_err(|source| FeatureScanError::Entry { source })?
            .path();
        if path.extension().and_then(|e| e.to_str()) != Some("feature") {
            continue;
        }
        let content = std::fs::read_to_string(&path).map_err(|source| FeatureScanError::File {
            path: path.display().to_string(),
            source,
        })?;
        if has_tag(&content, &tag) {
            out.push((path, content));
        }
    }
    Ok(out)
}

/// BUG-434: whether a tag line (a line starting with `@`) carries `tag` as a
/// whole whitespace-separated token, so `@BUG-1` never matches `@BUG-10`.
fn has_tag(content: &str, tag: &str) -> bool {
    content
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with('@'))
        .any(|line| line.split_whitespace().any(|token| token == tag))
}

/// A detected prefill placeholder: 1-based line number + placeholder name.
pub(super) struct PrefillMatch {
    line: usize,
    pattern: String,
}

/// Line-by-line prefill detection mirroring `detectPrefill` in
/// `src/utils/prefill-detection.ts` (case-insensitive substring patterns plus
/// the `TODO:` marker and the `@component` / `@feature-group` tag placeholders).
pub(super) fn detect_prefill(content: &str) -> Vec<PrefillMatch> {
    const SUBSTRING_PATTERNS: &[&str] = &[
        "[role]",
        "[action]",
        "[benefit]",
        "[precondition]",
        "[expected outcome]",
        "[scenario name]",
        "TODO:",
    ];
    let mut matches = Vec::new();
    // Preserve TS ordering: all matches for one pattern before the next.
    for pat in SUBSTRING_PATTERNS {
        let needle = pat.to_lowercase();
        for (i, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                matches.push(PrefillMatch {
                    line: i + 1,
                    pattern: (*pat).to_string(),
                });
            }
        }
    }
    for (name, needle) in [
        ("@component", "@component"),
        ("@feature-group", "@feature-group"),
    ] {
        for (i, line) in content.lines().enumerate() {
            if line.starts_with('@') && tag_placeholder_present(line, needle) {
                matches.push(PrefillMatch {
                    line: i + 1,
                    pattern: name.to_string(),
                });
            }
        }
    }
    matches
}

/// Whether `line` contains `needle` not immediately followed by a word
/// character (mirrors the `(?!\w)` lookahead in the TS tag regexes).
pub(super) fn tag_placeholder_present(line: &str, needle: &str) -> bool {
    if let Some(pos) = line.find(needle) {
        let after = &line[pos + needle.len()..];
        match after.chars().next() {
            Some(c) => !(c.is_alphanumeric() || c == '_'),
            None => true,
        }
    } else {
        false
    }
}

/// Prefill gate: blocks ALL forward transitions when the `@id`-tagged feature
/// file contains prefill placeholders. Mirrors `checkWorkUnitFeatureForPrefill`
/// + the inline message construction in the TS command.
pub(super) fn check_prefill(project_root: &Path, id: &str) -> Result<(), FspecCoreError> {
    let mut features = linked_feature_files(project_root, id)
        .map_err(|err| msg(format!("Cannot check {id} for prefill placeholders: {err}")))?;
    // BUG-384: every linked feature, in path order (read_dir order is
    // unspecified); the first one with placeholders blocks the move.
    features.sort_by(|a, b| a.0.cmp(&b.0));
    let Some((path, matches)) = features.iter().find_map(|(path, content)| {
        let matches = detect_prefill(content);
        (!matches.is_empty()).then_some((path, matches))
    }) else {
        return Ok(());
    };
    let file = path
        .strip_prefix(project_root)
        .unwrap_or(path)
        .display()
        .to_string();
    let detail: String = matches
        .iter()
        .take(3)
        .map(|m| format!("  Line {}: {}", m.line, m.pattern))
        .collect::<Vec<_>>()
        .join("\n");
    let tail = if matches.len() > 3 {
        format!("  ... and {} more\n\n", matches.len() - 3)
    } else {
        "\n".to_string()
    };
    Err(msg(format!(
        r#"Cannot advance work unit status: linked feature file {file} contains prefill placeholders.

Found {} placeholder(s):
{detail}
{tail}Fix prefill using CLI commands:
  - fspec set-user-story {id} --role='...' --action='...' --benefit='...'
  - fspec add-step <feature> <scenario> <keyword> <text>
  - fspec add-tag-to-feature <file> <tag>
  - fspec add-architecture <feature> <text>

DO NOT use Write or Edit tools to replace prefill directly."#,
        matches.len()
    )))
}
