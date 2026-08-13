//! `validate-tags` — Rust port of `src/commands/validate-tags.ts` +
//! `validate-tags-file.ts` + `validate-tags-registry.ts` (RPC-324).
//!
//! Validates that every feature-level and scenario-level tag in the project's
//! `spec/features/**/*.feature` files is registered in `spec/tags.json` and
//! enforces tag-placement rules (work-unit ID tags must live at feature
//! level, required component + feature-group categories must be present).
//!
//! ## Result envelope
//! Returns `{results, validCount, invalidCount}` via `Ok(json)`. Each
//! per-file result is `{file, valid, errors:[{tag, message, suggestion?}]}`.
//! The dispatcher derives `success=true` from the `Ok`; the CLI bridge owns
//! ALL rendering decisions (failures-only / --verbose / --summary) and exit
//! code (1 when `invalidCount > 0`). Rendering helpers live in `render_*`
//! below and are invoked by the CLI bridge through the JSON envelope, so the
//! bridge module embeds no validation or message strings.
//!
//! ## Registry loading
//! `spec/tags.json` is loaded via [`crate::io::ensure::ensure_tags_file`]
//! (auto-create canonical default when missing). `validTags` is the flat set
//! of every tag name; the `Component Tags` and `Feature Group Tags`
//! categories drive the required-category checks.
//!
//! ## work-units.json (best effort)
//! Loaded inline and treated as `Option`: ENOENT or any parse error →
//! `None` (parity with TS `loadWorkUnitsData`'s bare `catch`). Feature files
//! are enumerated via [`crate::io::feature_glob::glob_feature_files`]; a
//! `DirectoryNotFound` maps to an empty list (no spec/features dir → zero
//! counts). A non-Gherkin file is skipped (counts valid).
//!
//! ## Two-front-doors
//! Both the LLM dispatcher AND the clap subcommand call [`run`]
//! (RPC-003 §7/§11).

use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_tags_file;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ValidateTagsArgs {
    /// Validate only this feature file (relative to project root).
    file: Option<String>,
}

/// Derived tag registry: the flat valid-tag set + the required-category lists.
struct Registry {
    valid_tags: HashSet<String>,
    component: Vec<String>,
    feature_group: Vec<String>,
}

/// A single per-tag validation error.
struct TagError {
    tag: String,
    message: String,
    suggestion: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ValidateTagsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "validate-tags",
            reason: format!("failed to parse args: {e}"),
        })?;

    let registry = load_registry(project_root)?;
    let work_units = load_work_units(project_root);

    // ---- Enumerate target files ----
    let files: Vec<String> = match &args.file {
        Some(f) => vec![f.clone()],
        None => match glob_feature_files(project_root) {
            Ok(f) => f,
            Err(FspecCoreError::DirectoryNotFound { .. }) => Vec::new(),
            Err(other) => return Err(other),
        },
    };

    if files.is_empty() {
        return ok(json!({ "results": [], "validCount": 0, "invalidCount": 0 }));
    }

    // ---- Validate each file ----
    let mut results: Vec<Value> = Vec::with_capacity(files.len());
    let mut valid_count = 0u64;
    for file in &files {
        let (valid, errors) = validate_file(file, &registry, work_units.as_ref(), project_root);
        if valid {
            valid_count += 1;
        }
        let err_json: Vec<Value> = errors
            .iter()
            .map(|e| {
                let mut o = json!({ "tag": e.tag, "message": e.message });
                if let Some(s) = &e.suggestion {
                    o["suggestion"] = json!(s);
                }
                o
            })
            .collect();
        results.push(json!({ "file": file, "valid": valid, "errors": err_json }));
    }

    let invalid_count = results.len() as u64 - valid_count;
    ok(json!({
        "results": results,
        "validCount": valid_count,
        "invalidCount": invalid_count,
    }))
}

/// Load `spec/tags.json` (auto-create when missing) and derive the registry.
fn load_registry(project_root: &Path) -> Result<Registry, FspecCoreError> {
    let tags = ensure_tags_file(project_root)?;
    let mut valid_tags = HashSet::new();
    let mut component = Vec::new();
    let mut feature_group = Vec::new();
    for category in &tags.categories {
        for tag in &category.tags {
            valid_tags.insert(tag.name.clone());
            if category.name == "Component Tags" {
                component.push(tag.name.clone());
            } else if category.name == "Feature Group Tags" {
                feature_group.push(tag.name.clone());
            }
        }
    }
    Ok(Registry {
        valid_tags,
        component,
        feature_group,
    })
}

/// Read `spec/work-units.json` best-effort: any failure (ENOENT, I/O, parse)
/// yields `None`, matching TS `loadWorkUnitsData`'s bare `catch`. Returns the
/// set of known work-unit ids when present.
fn load_work_units(project_root: &Path) -> Option<HashSet<String>> {
    let path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let data: Value = serde_json::from_str(&raw).ok()?;
    let map = data.get("workUnits").and_then(Value::as_object)?;
    Some(map.keys().cloned().collect())
}

/// Validate a single feature file. Returns `(valid, errors)`. A file that
/// does not parse as Gherkin (or has no Feature) is treated as valid with no
/// errors (parity with TS early `return result`). A file that cannot be READ
/// (e.g. an explicitly-named non-existent file) is reported as a single
/// violation carrying the Node `ENOENT` message text (parity with TS
/// `validateFileTags`'s outer `catch`, which surfaces `error.message`).
fn validate_file(
    file: &str,
    registry: &Registry,
    work_units: Option<&HashSet<String>>,
    project_root: &Path,
) -> (bool, Vec<TagError>) {
    let abs = project_root.join(file);
    let content = match std::fs::read_to_string(&abs) {
        Ok(s) => s,
        Err(_) => {
            // TS: Node `readFile` throws an ENOENT Error caught by the outer
            // try/catch → `result.valid=false` with one error whose message is
            // `ENOENT: no such file or directory, open '<abs>'` (tag empty, no
            // suggestion). Node surfaces the absolute path it attempted.
            return (
                false,
                vec![TagError {
                    tag: String::new(),
                    message: format!(
                        "ENOENT: no such file or directory, open '{}'",
                        abs.display()
                    ),
                    suggestion: None,
                }],
            );
        }
    };
    let feature = match parse_feature_lenient(&content) {
        Ok(f) => f,
        Err(_) => return (true, Vec::new()),
    };

    // gherkin-0.16 strips the leading '@'; re-prepend for comparison.
    let feature_tags: Vec<String> = feature.tags.iter().map(|t| format!("@{t}")).collect();
    let mut scenario_tags: Vec<String> = Vec::new();
    for s in &feature.scenarios {
        for t in &s.tags {
            scenario_tags.push(format!("@{t}"));
        }
    }
    for rule in &feature.rules {
        for s in &rule.scenarios {
            for t in &s.tags {
                scenario_tags.push(format!("@{t}"));
            }
        }
    }

    let mut errors: Vec<TagError> = Vec::new();
    validate_unregistered_feature_tags(&feature_tags, registry, work_units, file, &mut errors);
    validate_scenario_tags(&scenario_tags, registry, file, &mut errors);
    validate_required_category_tags(&feature_tags, registry, &mut errors);

    (errors.is_empty(), errors)
}

fn validate_unregistered_feature_tags(
    feature_tags: &[String],
    registry: &Registry,
    work_units: Option<&HashSet<String>>,
    file: &str,
    errors: &mut Vec<TagError>,
) {
    for tag in feature_tags {
        if registry.valid_tags.contains(tag) {
            continue;
        }
        if is_work_unit_tag(tag) {
            report_work_unit_tag(tag, work_units, errors);
        } else if looks_like_work_unit_tag(tag) {
            errors.push(TagError {
                tag: tag.clone(),
                message: format!("Invalid work unit tag format: {tag}"),
                suggestion: Some(
                    "Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)"
                        .to_string(),
                ),
            });
        } else if tag == "@component" || tag == "@feature-group" {
            errors.push(TagError {
                tag: tag.clone(),
                message: format!("Placeholder tag: {tag}"),
                suggestion: Some(format!("Replace {tag} with actual tags from tags.json")),
            });
        } else {
            errors.push(TagError {
                tag: tag.clone(),
                message: format!("Unregistered tag: {tag} in {file}"),
                suggestion: Some(
                    "Register this tag in spec/tags.json or use 'fspec register-tag'".to_string(),
                ),
            });
        }
    }
}

fn validate_scenario_tags(
    scenario_tags: &[String],
    registry: &Registry,
    file: &str,
    errors: &mut Vec<TagError>,
) {
    // CRITICAL: reject scenario-level work-unit ID tags (BUG-005).
    for tag in scenario_tags {
        if is_work_unit_tag(tag) {
            errors.push(TagError {
                tag: tag.clone(),
                message: format!(
                    "Work unit ID tag {tag} must be at feature level, not scenario level"
                ),
                suggestion: Some(format!(
                    "Move {tag} to feature-level tags. Use coverage files for fine-grained scenario traceability."
                )),
            });
        }
    }

    for tag in scenario_tags {
        if registry.valid_tags.contains(tag) {
            continue;
        }
        if is_work_unit_tag(tag) {
            continue; // already handled above
        }
        if looks_like_work_unit_tag(tag) {
            errors.push(TagError {
                tag: tag.clone(),
                message: format!("Invalid work unit tag format: {tag}"),
                suggestion: Some(
                    "Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)"
                        .to_string(),
                ),
            });
        } else {
            errors.push(TagError {
                tag: tag.clone(),
                message: format!("Unregistered tag: {tag} in {file}"),
                suggestion: Some(
                    "Register this tag in spec/tags.json or use 'fspec register-tag'".to_string(),
                ),
            });
        }
    }
}

fn report_work_unit_tag(
    tag: &str,
    work_units: Option<&HashSet<String>>,
    errors: &mut Vec<TagError>,
) {
    let id = match extract_work_unit_id(tag) {
        Some(id) => id,
        None => {
            errors.push(TagError {
                tag: tag.to_string(),
                message: format!("Invalid work unit tag format: {tag}"),
                suggestion: Some(
                    "Work unit tags must match pattern @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001, @BACK-123)"
                        .to_string(),
                ),
            });
            return;
        }
    };

    match work_units {
        None => {
            errors.push(TagError {
                tag: tag.to_string(),
                message: format!("Work unit {tag} found but spec/work-units.json does not exist"),
                suggestion: Some("Create spec/work-units.json to define work units".to_string()),
            });
        }
        Some(set) if !set.contains(&id) => {
            errors.push(TagError {
                tag: tag.to_string(),
                message: format!("Work unit {tag} not found in spec/work-units.json"),
                suggestion: Some(format!(
                    "Add work unit {id} to spec/work-units.json or use 'fspec create-story/create-bug/create-task'"
                )),
            });
        }
        Some(_) => {}
    }
}

fn validate_required_category_tags(
    tags: &[String],
    registry: &Registry,
    errors: &mut Vec<TagError>,
) {
    let has_component = tags.iter().any(|t| registry.component.contains(t));
    if !has_component && !tags.iter().any(|t| t == "@component") {
        errors.push(TagError {
            tag: String::new(),
            message: "Missing required component tag".to_string(),
            suggestion: Some(format!("Add one of: {}", registry.component.join(", "))),
        });
    }
    let has_group = tags.iter().any(|t| registry.feature_group.contains(t));
    if !has_group && !tags.iter().any(|t| t == "@feature-group") {
        errors.push(TagError {
            tag: String::new(),
            message: "Missing required feature-group tag".to_string(),
            suggestion: Some(format!("Add one of: {}", registry.feature_group.join(", "))),
        });
    }
}

// ---- Work-unit tag pattern helpers (no regex dependency) ----

/// `/^@([A-Z]{2,6}-\d+)$/` — strict uppercase work-unit tag.
fn is_work_unit_tag(tag: &str) -> bool {
    matches_work_unit_pattern(tag, false)
}

/// `/^@([a-zA-Z]{2,6}-\d+)$/` — looks like a work-unit tag (any case).
fn looks_like_work_unit_tag(tag: &str) -> bool {
    matches_work_unit_pattern(tag, true)
}

/// Extract the id portion (e.g. `AUTH-001`) from a strict work-unit tag.
fn extract_work_unit_id(tag: &str) -> Option<String> {
    if is_work_unit_tag(tag) {
        Some(tag[1..].to_string())
    } else {
        None
    }
}

/// Shared matcher for `@LETTERS-DIGITS` where LETTERS is 2..=6 chars
/// (uppercase only unless `any_case`) and DIGITS is one or more.
fn matches_work_unit_pattern(tag: &str, any_case: bool) -> bool {
    let Some(rest) = tag.strip_prefix('@') else {
        return false;
    };
    let Some((letters, digits)) = rest.split_once('-') else {
        return false;
    };
    let letters_ok = (2..=6).contains(&letters.chars().count())
        && letters.chars().all(|c| {
            if any_case {
                c.is_ascii_alphabetic()
            } else {
                c.is_ascii_uppercase()
            }
        });
    let digits_ok = !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit());
    letters_ok && digits_ok
}

/// Serialise the result envelope to the `Ok(String)` returned to the dispatcher.
fn ok(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "validate-tags",
        reason: format!("failed to serialise response: {e}"),
    })
}

/// Render the CLI output for a `validate-tags` envelope (parity with the TS
/// `renderValidateTagsOutput` at `src/commands/validate-tags-output.ts`).
///
/// Lives in fspec_core so the standalone CLI bridge stays pure marshalling
/// and never embeds validation/rendering strings. `data` is the JSON
/// envelope produced by [`run`]; `verbose`/`summary` are the CLI flags.
///
/// Rules:
///   - `--summary` (wins over `--verbose`): print only the summary count lines.
///   - default: print only `✗` violation blocks; no per-file `✓` lines.
///   - `--verbose`: also print one `✓` line per passing file.
///   - summary section prints whenever `summary` OR more than one file.
pub fn render_cli_output(data: &Value, verbose: bool, summary: bool) -> String {
    let mut lines: Vec<String> = Vec::new();
    let summary_only = summary;
    let verbose = verbose && !summary_only;

    let results = data["results"].as_array().cloned().unwrap_or_default();
    let valid_count = data["validCount"].as_u64().unwrap_or(0);
    let invalid_count = data["invalidCount"].as_u64().unwrap_or(0);

    if !summary_only {
        for r in &results {
            let file = r["file"].as_str().unwrap_or("");
            let valid = r["valid"].as_bool().unwrap_or(true);
            if valid {
                if verbose {
                    lines.push(format!("✓ All tags in {file} are registered"));
                }
            } else {
                lines.push(format!("✗ {file} has tag violations:"));
                if let Some(errs) = r["errors"].as_array() {
                    for e in errs {
                        if let Some(m) = e["message"].as_str() {
                            lines.push(format!("  {m}"));
                        }
                        if let Some(s) = e["suggestion"].as_str() {
                            lines.push(format!("  Suggestion: {s}"));
                        }
                    }
                }
            }
        }
    }

    let should_print_summary = summary_only || results.len() > 1;
    if should_print_summary {
        if invalid_count == 0 {
            lines.push(format!("✓ {valid_count} files passed"));
        } else {
            lines.push(format!("✓ {valid_count} files passed"));
            lines.push(format!("✗ {invalid_count} files have tag violations"));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn work_unit_tag_patterns() {
        assert!(is_work_unit_tag("@AUTH-001"));
        assert!(!is_work_unit_tag("@auth-001"));
        assert!(looks_like_work_unit_tag("@auth-001"));
        assert!(!looks_like_work_unit_tag("@made-up")); // "up" is not digits
        assert_eq!(
            extract_work_unit_id("@AUTH-001").as_deref(),
            Some("AUTH-001")
        );
        assert_eq!(extract_work_unit_id("@auth-001"), None);
    }
}
