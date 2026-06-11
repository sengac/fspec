//! `add-tag-to-scenario` — Rust port of `src/commands/add-tag-to-scenario.ts` (RPC-194).
//!
//! Adds one or more tags above a specific `Scenario:` line in a Gherkin
//! feature file. The mutation is line-based to mirror the TypeScript
//! implementation byte-for-byte (parser-driven re-emit would reformat the
//! whole file and lose comments / blank lines / indentation choices).
//!
//! ## Algorithm
//!
//! 1. Parse `args_json` → `{file, scenario, tags, validateRegistry?}`.
//! 2. Read the file. ENOENT → `Err(InvalidArgs{"File not found: <rel>"})`.
//! 3. Validate every tag's format:
//!    - Must start with `@` (else `"Invalid tag format. Tags must start with @"`).
//!    - Must match `is_work_unit_tag` (`@[A-Z]{2,6}-\d+`) OR
//!      `is_regular_tag` (`@[a-z0-9-#]+`) else
//!      `"Invalid tag format. Regular tags must use lowercase-with-hyphens, ..."`.
//! 4. Parse via [`crate::io::gherkin::parse_feature_lenient`] to locate the
//!    scenario by exact-name match. Missing → `Err("Scenario '<name>' not
//!    found in <rel>")`.
//! 5. Reject duplicates: any incoming tag already present on the scenario →
//!    `Err("Tag <tag> already exists on this scenario")`.
//! 6. When `validateRegistry=true`, load `spec/tags.json` and require every
//!    tag to be registered (`"<tag> is not registered in spec/tags.json"`).
//! 7. Walk the raw lines to find the `Scenario: <name>` line; compute the
//!    insertion index (immediately after any existing tag block above it,
//!    else immediately above the Scenario line). Splice the new
//!    `<indent><tag>` lines in.
//! 8. Re-parse the mutated text with the strict `gherkin` crate to set
//!    `valid` true/false. Write the file regardless (parity with TS — TS
//!    *also* writes even when `valid=false`).
//!
//! ## Returned JSON shape
//!
//! ```json
//! {"success":true,"valid":true,"message":"Added @smoke to scenario 'Login'"}
//! ```
//!
//! The dispatcher stores this verbatim in `DispatchResult.data` and the
//! CLI bridge parses it to extract `message` for the `✓ ...` stdout line.
//!
//! ## is_work_unit_tag / is_regular_tag
//!
//! The TypeScript impl imports `isWorkUnitTag` from
//! `src/utils/work-unit-tags.ts` and inlines the regular-tag regex. The
//! Rust port inlines BOTH predicates as private helpers in this file
//! (extraction to a shared `tags.rs` module is deferred — see RPC-194
//! port-notes).

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

/// CLI arguments accepted by `add-tag-to-scenario`. Mirrors the TS
/// signature `(featureFilePath, scenarioName, tags, options)` at
/// `src/commands/add-tag-to-scenario.ts:22-27`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddTagToScenarioArgs {
    file: String,
    scenario: String,
    tags: Vec<String>,
    #[serde(default)]
    validate_registry: bool,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddTagToScenarioArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-tag-to-scenario",
            reason: format!("failed to parse args: {e}"),
        })?;

    let rel = args.file.clone();
    let full = project_root.join(&rel);

    // Read file. ENOENT → canonical "File not found:" message.
    let content = match std::fs::read_to_string(&full) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-tag-to-scenario",
                reason: format!("File not found: {rel}"),
            });
        }
        Err(e) => {
            return Err(FspecCoreError::Io {
                command: "add-tag-to-scenario",
                source: e,
            });
        }
    };

    // Validate every tag's format.
    for tag in &args.tags {
        if !tag.starts_with('@') {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-tag-to-scenario",
                reason: "Invalid tag format. Tags must start with @".to_string(),
            });
        }
        let is_wu = is_work_unit_tag(tag);
        let is_reg = is_regular_tag(tag);
        if !is_wu && !is_reg {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-tag-to-scenario",
                reason: "Invalid tag format. Regular tags must use lowercase-with-hyphens, \
                         work unit tags must match @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001)"
                    .to_string(),
            });
        }
    }

    // Parse Gherkin to find the scenario by exact name.
    let feature = parse_feature_lenient(&content).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-tag-to-scenario",
        reason: format!("Invalid Gherkin syntax: {e}"),
    })?;

    // Search only top-level scenarios (mirror TS
    // `gherkinDocument.feature.children.filter(c => c.scenario && c.scenario.keyword === 'Scenario')`
    // at src/commands/add-tag-to-scenario.ts:94-100).
    let target = find_scenario(&feature, &args.scenario);
    let scenario_ref = match target {
        Some(s) => s,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-tag-to-scenario",
                reason: format!("Scenario '{}' not found in {}", args.scenario, rel),
            });
        }
    };

    // Existing tag list (names, normalised to include the leading `@`).
    // The `gherkin` crate strips the `@` during parse — re-add it so
    // duplicate detection compares apples to apples with the
    // user-supplied `@`-prefixed input.
    let existing_tags: Vec<String> = scenario_ref
        .tags
        .iter()
        .map(|t| {
            if t.starts_with('@') {
                t.clone()
            } else {
                format!("@{t}")
            }
        })
        .collect();

    // Duplicate guard.
    for tag in &args.tags {
        if existing_tags.iter().any(|e| e == tag) {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-tag-to-scenario",
                reason: format!("Tag {tag} already exists on this scenario"),
            });
        }
    }

    // Registry validation (optional, opt-in).
    if args.validate_registry {
        let registered = load_registered_tags(project_root).map_err(|reason| {
            FspecCoreError::InvalidArgs {
                command: "add-tag-to-scenario",
                reason: format!("Failed to validate against registry: {reason}"),
            }
        })?;
        for tag in &args.tags {
            if !registered.iter().any(|r| r == tag) {
                return Err(FspecCoreError::InvalidArgs {
                    command: "add-tag-to-scenario",
                    reason: format!("Tag {tag} is not registered in spec/tags.json"),
                });
            }
        }
    }

    // Locate the literal `Scenario: <name>` line in the raw content.
    let lines: Vec<&str> = content.split('\n').collect();
    let scenario_target = format!("Scenario: {}", args.scenario);
    let scenario_line_idx = match lines
        .iter()
        .position(|l| l.trim() == scenario_target.as_str())
    {
        Some(i) => i,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-tag-to-scenario",
                reason: format!("Could not find Scenario line for \"{}\"", args.scenario),
            });
        }
    };

    // Compute insert index. TS algorithm:
    //   - default insertIndex = scenario_line_idx (insert just above)
    //   - walk upward skipping `@`-lines and blank lines
    //   - if there are existing tags, find the LAST `@`-line and insert
    //     immediately AFTER it (so new tags append to the tag block)
    let mut insert_index = scenario_line_idx;
    {
        let mut i = scenario_line_idx as isize - 1;
        while i >= 0 {
            let trimmed = lines[i as usize].trim();
            if !trimmed.starts_with('@') && !trimmed.is_empty() {
                insert_index = (i + 1) as usize;
                break;
            }
            if i == 0 || trimmed.is_empty() {
                insert_index = scenario_line_idx;
                break;
            }
            i -= 1;
        }
    }
    if !existing_tags.is_empty() {
        let mut i = scenario_line_idx as isize - 1;
        while i >= 0 {
            let trimmed = lines[i as usize].trim();
            if trimmed.starts_with('@') {
                insert_index = (i + 1) as usize;
                break;
            }
            i -= 1;
        }
    }

    // Indentation of the Scenario: line — reused for tag indentation.
    let scenario_line = lines[scenario_line_idx];
    let indent: String = scenario_line
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();
    let indent_str = if indent.is_empty() { "  ".to_string() } else { indent };

    // Splice new tag lines in.
    let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
    for (offset, tag) in args.tags.iter().enumerate() {
        new_lines.insert(insert_index + offset, format!("{indent_str}{tag}"));
    }
    let new_content = new_lines.join("\n");

    // Determine `valid` flag (parity with TS).
    let valid = gherkin::Feature::parse(&new_content, gherkin::GherkinEnv::default()).is_ok();

    // Write file (TS writes even when `valid=false`).
    std::fs::write(&full, &new_content).map_err(|e| FspecCoreError::Io {
        command: "add-tag-to-scenario",
        source: e,
    })?;

    let tag_list = args.tags.join(", ");
    let result = json!({
        "success": true,
        "valid": valid,
        "message": format!("Added {tag_list} to scenario '{}'", args.scenario),
    });
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-tag-to-scenario",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// True when `tag` matches the work-unit tag pattern `@[A-Z]{2,6}-\d+`.
/// Inlined as a hand-rolled scanner here to avoid pulling `regex` into
/// `codelet-fspec-core`'s runtime dependencies (it currently ships as a
/// dev-dependency only). See RPC-194 port-notes for the deferred
/// extraction into a shared `tags` module.
fn is_work_unit_tag(tag: &str) -> bool {
    let Some(rest) = tag.strip_prefix('@') else {
        return false;
    };
    let Some((prefix, num)) = rest.split_once('-') else {
        return false;
    };
    if prefix.len() < 2 || prefix.len() > 6 {
        return false;
    }
    if !prefix.chars().all(|c| c.is_ascii_uppercase()) {
        return false;
    }
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    true
}

/// True when `tag` matches the regular-tag pattern `@[a-z0-9-#]+`.
/// Hand-rolled scanner — see [`is_work_unit_tag`] for rationale.
fn is_regular_tag(tag: &str) -> bool {
    let Some(rest) = tag.strip_prefix('@') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    rest.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '#')
}

/// Locate a top-level scenario by exact name — mirrors TS
/// `feature.children.filter(c => c.scenario && c.scenario.keyword === 'Scenario')`
/// at src/commands/add-tag-to-scenario.ts:94-100. Scenarios nested under
/// `Rule:` blocks are intentionally NOT searched, matching TS behaviour
/// exactly (the TS filter uses the flat `feature.children` array which
/// only contains top-level children).
fn find_scenario<'a>(
    feature: &'a gherkin::Feature,
    name: &str,
) -> Option<&'a gherkin::Scenario> {
    for s in &feature.scenarios {
        if s.name == name {
            return Some(s);
        }
    }
    None
}

/// Load all registered tag names from `spec/tags.json`. Returns
/// `Err(reason)` on read or parse failure. The reason string is
/// surfaced verbatim through the registry-validation error message
/// (parity with TS `"Failed to validate against registry: <msg>"`).
fn load_registered_tags(project_root: &Path) -> Result<Vec<String>, String> {
    let path = project_root.join("spec").join("tags.json");
    let body =
        std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let v: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let mut tags: Vec<String> = Vec::new();
    if let Some(cats) = v.get("categories").and_then(|c| c.as_array()) {
        for cat in cats {
            if let Some(arr) = cat.get("tags").and_then(|t| t.as_array()) {
                for t in arr {
                    if let Some(n) = t.get("name").and_then(|n| n.as_str()) {
                        tags.push(n.to_string());
                    }
                }
            }
        }
    }
    Ok(tags)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: AddTagToScenarioArgs = serde_json::from_str(
            r#"{"file":"x.feature","scenario":"S","tags":["@smoke"],"validateRegistry":true}"#,
        )
        .unwrap();
        assert_eq!(a.file, "x.feature");
        assert_eq!(a.scenario, "S");
        assert_eq!(a.tags, vec!["@smoke"]);
        assert!(a.validate_registry);
    }

    #[test]
    fn args_parse_with_optional_validate_registry_default_false() {
        let a: AddTagToScenarioArgs =
            serde_json::from_str(r#"{"file":"x.feature","scenario":"S","tags":["@smoke"]}"#)
                .unwrap();
        assert!(!a.validate_registry);
    }

    #[test]
    fn work_unit_tag_matcher_accepts_canonical_shape() {
        assert!(is_work_unit_tag("@AUTH-001"));
        assert!(is_work_unit_tag("@RPC-194"));
        assert!(!is_work_unit_tag("@auth-001"));
        assert!(!is_work_unit_tag("@smoke"));
    }

    #[test]
    fn regular_tag_matcher_accepts_lowercase_with_hyphens_and_hash() {
        assert!(is_regular_tag("@smoke"));
        assert!(is_regular_tag("@regression"));
        assert!(is_regular_tag("@critical-path"));
        assert!(is_regular_tag("@v1"));
        assert!(!is_regular_tag("@CamelCase"));
    }
}
