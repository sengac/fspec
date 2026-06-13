//! `remove-tag-from-scenario` — Rust port of
//! `src/commands/remove-tag-from-scenario.ts` (RPC-282).
//!
//! Removes one or more tags from above a specific `Scenario:` line in a
//! Gherkin feature file. Mirrors the TS implementation byte-for-byte:
//!
//! * Idempotent for a missing scenario (returns success with
//!   `"Scenario '<name>' not found in <rel> - no changes made"`).
//! * Idempotent when none of the requested tags are present (returns
//!   success with `"No changes made - none of the specified tags found
//!   on scenario '<name>'"`).
//! * File-not-found is a HARD failure (`"File not found: <rel>"`).
//! * On real removal the message is
//!   `"Removed <tag1>, <tag2>, … from scenario '<name>'"` and the file is
//!   re-emitted with the matching `@`-tag lines removed.
//!
//! ## Returned JSON shape
//!
//! ```json
//! {"success":true,"valid":true,"message":"Removed @critical from scenario 'Login'"}
//! ```

use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

/// CLI arguments accepted by `remove-tag-from-scenario`. Mirrors the TS
/// signature `(featureFilePath, scenarioName, tags, options)` at
/// `src/commands/remove-tag-from-scenario.ts:20-25`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveTagFromScenarioArgs {
    file: String,
    scenario: String,
    tags: Vec<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveTagFromScenarioArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-tag-from-scenario",
            reason: format!("failed to parse args: {e}"),
        })?;

    let rel = args.file.clone();
    let full = project_root.join(&rel);

    // Read file. ENOENT → hard failure (parity with TS).
    let content = match std::fs::read_to_string(&full) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-tag-from-scenario",
                reason: format!("File not found: {rel}"),
            });
        }
        Err(e) => {
            return Err(FspecCoreError::Io {
                command: "remove-tag-from-scenario",
                source: e,
            });
        }
    };

    // Parse Gherkin to look up the scenario.
    let feature = parse_feature_lenient(&content).map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-tag-from-scenario",
        reason: format!("Invalid Gherkin syntax: {e}"),
    })?;

    let target = find_scenario(&feature, &args.scenario);

    // Missing scenario → idempotent success.
    let scenario_ref = match target {
        Some(s) => s,
        None => {
            let result = json!({
                "success": true,
                "valid": true,
                "message": format!(
                    "Scenario '{}' not found in {} - no changes made",
                    args.scenario, rel
                ),
            });
            return serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "remove-tag-from-scenario",
                reason: format!("failed to serialize result: {e}"),
            });
        }
    };

    // Existing tag list (names, normalised to include the leading `@`).
    // The `gherkin` crate strips the `@` during parse — re-add it so
    // membership tests compare apples to apples with the user-supplied
    // `@`-prefixed input.
    let existing: Vec<String> = scenario_ref
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
    let to_remove: Vec<String> = args
        .tags
        .iter()
        .filter(|t| existing.iter().any(|e| e == *t))
        .cloned()
        .collect();

    // None present → idempotent success, no file write.
    if to_remove.is_empty() {
        let result = json!({
            "success": true,
            "valid": true,
            "message": format!(
                "No changes made - none of the specified tags found on scenario '{}'",
                args.scenario
            ),
        });
        return serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-tag-from-scenario",
            reason: format!("failed to serialize result: {e}"),
        });
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
                command: "remove-tag-from-scenario",
                reason: format!("Could not find Scenario line for \"{}\"", args.scenario),
            });
        }
    };

    // Walk every line; drop those that are tag lines belonging to OUR
    // target scenario AND whose tag is in `to_remove`. A tag line
    // belongs to the target scenario if scanning forward from it lands
    // on `Scenario: <name>` before any other `Scenario:` or `Feature:`
    // header. Mirrors TS lines 122-162.
    let mut kept: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        if i < scenario_line_idx {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix('@') {
                let tag_with_at = {
                    let mut s = String::from("@");
                    s.push_str(rest);
                    s
                };
                // Look ahead to see what header this tag attaches to.
                let mut belongs_to_target = false;
                for nxt_line in &lines[i + 1..] {
                    let nxt = nxt_line.trim();
                    if nxt == scenario_target.as_str() {
                        belongs_to_target = true;
                        break;
                    }
                    if nxt.starts_with("Scenario:") || nxt.starts_with("Feature:") {
                        break;
                    }
                }
                if belongs_to_target && to_remove.iter().any(|t| t == &tag_with_at) {
                    // Drop this line.
                    i += 1;
                    continue;
                }
            }
        }
        kept.push(line.to_string());
        i += 1;
    }
    let new_content = kept.join("\n");

    // Validity check (parity with TS).
    let valid = gherkin::Feature::parse(&new_content, gherkin::GherkinEnv::default()).is_ok();

    std::fs::write(&full, &new_content).map_err(|e| FspecCoreError::Io {
        command: "remove-tag-from-scenario",
        source: e,
    })?;

    let tag_list = to_remove.join(", ");
    let result = json!({
        "success": true,
        "valid": valid,
        "message": format!("Removed {tag_list} from scenario '{}'", args.scenario),
    });
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-tag-from-scenario",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Locate a top-level scenario by exact name — mirrors TS
/// `feature.children.filter(c => c.scenario && c.scenario.keyword === 'Scenario')`
/// at src/commands/remove-tag-from-scenario.ts. Scenarios nested under
/// `Rule:` blocks are intentionally NOT searched, matching the TS filter
/// which iterates only the flat `feature.children` array.
fn find_scenario<'a>(feature: &'a gherkin::Feature, name: &str) -> Option<&'a gherkin::Scenario> {
    feature.scenarios.iter().find(|s| s.name == name)
}

// (no extra anchor needed)

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: RemoveTagFromScenarioArgs = serde_json::from_str(
            r#"{"file":"x.feature","scenario":"S","tags":["@smoke","@critical"]}"#,
        )
        .unwrap();
        assert_eq!(a.file, "x.feature");
        assert_eq!(a.scenario, "S");
        assert_eq!(a.tags, vec!["@smoke", "@critical"]);
    }
}
