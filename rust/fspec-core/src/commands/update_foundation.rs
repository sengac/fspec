//! `update-foundation` — Rust port of `src/commands/update-foundation.ts`
//! (RPC-312).
//!
//! Updates a single field in `spec/foundation.json` (or
//! `spec/foundation.json.draft` when a draft exists). The command:
//!
//! 1. Validates the section name is non-empty.
//! 2. Runs fail-fast field-specific validation BEFORE the generic
//!    empty-content guard:
//!      * `projectType` — freeform 1-30 character length rule.
//!      * `problemImpact` — strict `high|medium|low` enum.
//! 3. Rejects empty content for every other section.
//! 4. Detects whether `spec/foundation.json.draft` exists — if so, the draft
//!    is the write target (draft takes precedence over the final foundation).
//! 5. Loads the target document (the draft is read directly; the final
//!    foundation is loaded-or-initialised via [`ensure_foundation_file`]).
//! 6. Maps the section alias to its nested JSON path and writes the value.
//! 7. On the FINAL path, regenerates `spec/FOUNDATION.md` after the write.
//!
//! ## D1 parity (draft systemReminder chaining)
//!
//! On the draft path the TS command chains to
//! `discoverFoundation({scanOnly})` which scans the draft for the next field
//! still containing a `[QUESTION:` / `[DETECTED:` placeholder and emits a
//! field-specific `<system-reminder>` (the same field-by-field guidance
//! `discover-foundation` produces). This is now ported here verbatim:
//! [`scan_draft_for_next_field_reminder`] reproduces `scanDraftForNextField`
//! and `generateFieldReminder`, including the agent-aware "ULTRATHINK" vs
//! "Think a lot" branch keyed off the resolved agent's `supportsMetaCognition`
//! capability (FSPEC_AGENT env var > spec/fspec-config.json `agent` field >
//! safe default — only `claude` and `antigravity` enable meta-cognition).
//!
//! ## D2 parity (final-path schema gate)
//!
//! On the FINAL path the TS command writes `foundation.json` first, then runs
//! `validateFoundationJson` (Ajv, `allErrors:true`) against it. On failure it
//! returns `"Updated foundation.json failed schema validation: <messages>"`
//! (each Ajv error's `.message` joined by `", "`, no instancePath) WITHOUT
//! regenerating `FOUNDATION.md`. The written file is left in place. On success
//! it regenerates `FOUNDATION.md`. This is now ported using the in-crate
//! native validator [`crate::generators::foundation_schema::validate_foundation`]
//! (the same one `generate-foundation-md` uses).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function. The CLI bridge at
//! `rust/fspec/src/update_foundation.rs` is JSON marshalling + stdout
//! rendering only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_foundation_file;
use crate::io::locked_file::write_json_atomic;

/// CLI arguments accepted by `update-foundation`. Mirrors the TS
/// `UpdateFoundationOptions` positional arguments (`section`, `content`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateFoundationArgs {
    #[serde(default)]
    section: String,
    #[serde(default)]
    content: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdateFoundationArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "update-foundation",
            reason: format!("failed to parse args: {e}"),
        })?;

    let section = args.section.as_str();
    let content = args.content.as_str();

    // [1] Section name must be non-empty (trimmed). Rejected before any I/O.
    if section.trim().is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "update-foundation",
            reason: "Section name cannot be empty".to_string(),
        });
    }

    // [2a] projectType has its own length-based rule (freeform 1-30 chars)
    // enforced BEFORE the generic empty-content guard, so empty/over-long
    // values surface an actionable length error.
    if section == "projectType" {
        if let Some(err) = validate_project_type_length(content) {
            return Err(FspecCoreError::InvalidArgs {
                command: "update-foundation",
                reason: err,
            });
        }
    }

    // [2b] problemImpact must be one of the fixed enum values.
    if section == "problemImpact" {
        if let Some(err) = validate_problem_impact(content) {
            return Err(FspecCoreError::InvalidArgs {
                command: "update-foundation",
                reason: err,
            });
        }
    }

    // [3] Generic empty-content guard for all other sections.
    if content.trim().is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "update-foundation",
            reason: "Section content cannot be empty".to_string(),
        });
    }

    // [4] Detect draft. The draft takes precedence over the final foundation.
    let draft_path = project_root.join("spec").join("foundation.json.draft");
    let final_path = project_root.join("spec").join("foundation.json");
    let is_draft = draft_path.exists();

    // [5] Load the target document.
    let mut foundation: Value = if is_draft {
        let raw = std::fs::read_to_string(&draft_path).map_err(|source| FspecCoreError::Io {
            command: "update-foundation",
            source,
        })?;
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "foundation.json.draft".to_string(),
            reason: crate::io::json_error::parse_json_reason(&raw, &e),
        })?
    } else {
        ensure_foundation_file(project_root)?
    };

    // [6] Map section → nested JSON path and apply the value. On an unknown
    // section we return WITHOUT writing — the file stays byte-identical.
    if !update_json_field(&mut foundation, section, content) {
        return Err(FspecCoreError::InvalidArgs {
            command: "update-foundation",
            reason: format!(
                "Unknown section: \"{section}\". Use field names like: projectOverview, problemDefinition, etc."
            ),
        });
    }

    // Write the updated document to the resolved target (draft or final).
    let target_path = if is_draft { &draft_path } else { &final_path };
    write_json_atomic(target_path, &foundation)?;

    let message = if is_draft {
        format!("Updated \"{section}\" in foundation.json.draft")
    } else {
        // D2 parity: the TS final branch validates the freshly-written
        // foundation.json against the bundled generic-foundation schema
        // (Ajv, allErrors:true). On failure it returns
        //   "Updated foundation.json failed schema validation: <m1>, <m2>, ..."
        // (each Ajv error's `.message`, joined by ", ", NO instancePath) and
        // does NOT regenerate FOUNDATION.md. The file itself stays written
        // (TS writes BEFORE validating). On success it regenerates
        // FOUNDATION.md. (src/commands/update-foundation.ts:136-154.)
        if let Err(errs) = crate::generators::foundation_schema::validate_foundation(&foundation) {
            let joined = errs
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(FspecCoreError::InvalidArgs {
                command: "update-foundation",
                reason: format!("Updated foundation.json failed schema validation: {joined}"),
            });
        }
        crate::commands::generate_foundation_md::regenerate(project_root);
        format!("Updated \"{section}\" section in FOUNDATION.md")
    };

    // D1 parity: on the draft path the TS command chains to
    // `discoverFoundation({scanOnly})` to surface the NEXT unfilled placeholder
    // field via a `<system-reminder>`. Replicate that here so the CLI bridge
    // can print it after the "Updated:" line (matches
    // `updateFoundationCommand`, src/commands/update-foundation.ts:296-314).
    let system_reminder = if is_draft {
        crate::foundation::guidance::next_field_reminder(&foundation, project_root)
    } else {
        None
    };

    let mut result = json!({ "success": true, "message": message });
    if let Some(reminder) = system_reminder {
        result["systemReminder"] = Value::String(reminder);
    }
    // DISC-003 rule 4: universal progress trailer on the draft path (the
    // final foundation is complete by definition, so no trailer there).
    if is_draft {
        result["nextSteps"] =
            Value::String(crate::foundation::guidance::draft_trailer(&foundation));
    }

    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "update-foundation",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Map a section alias to its nested JSON path and write `content`. Returns
/// `true` when the section is recognised, `false` otherwise. Intermediate
/// objects are created on demand (mirrors the TS `foundation.x = foundation.x
/// || {}` idiom). Generic schema v2.0.0 — verbatim port of `updateJsonField`
/// (`src/commands/update-foundation.ts:165-237`).
fn update_json_field(foundation: &mut Value, section: &str, content: &str) -> bool {
    let value = Value::String(content.to_string());
    match section {
        // Project fields
        "projectName" | "name" => {
            set_nested(foundation, &["project", "name"], value);
            true
        }
        "projectVision" | "vision" => {
            set_nested(foundation, &["project", "vision"], value);
            true
        }
        "projectType" => {
            set_nested(foundation, &["project", "projectType"], value);
            true
        }
        // Problem space fields
        "problemTitle" => {
            set_nested(
                foundation,
                &["problemSpace", "primaryProblem", "title"],
                value,
            );
            true
        }
        "problemDefinition" | "problemDescription" => {
            set_nested(
                foundation,
                &["problemSpace", "primaryProblem", "description"],
                value,
            );
            true
        }
        "problemImpact" => {
            set_nested(
                foundation,
                &["problemSpace", "primaryProblem", "impact"],
                value,
            );
            true
        }
        // Solution space fields
        "solutionOverview" | "projectOverview" => {
            set_nested(foundation, &["solutionSpace", "overview"], value);
            true
        }
        // Legacy mappings for backward compatibility — map to
        // solutionSpace.overview (old schema fields).
        "testingStrategy"
        | "developmentTools"
        | "architecturePattern"
        | "painPoints"
        | "methodology" => {
            set_nested(foundation, &["solutionSpace", "overview"], value);
            true
        }
        _ => false,
    }
}

/// Write `value` at the nested object path `keys`, creating intermediate
/// objects as needed. The root is coerced to an object if it is not already.
fn set_nested(root: &mut Value, keys: &[&str], value: Value) {
    if !root.is_object() {
        *root = Value::Object(serde_json::Map::new());
    }
    let mut cur = root;
    for (i, key) in keys.iter().enumerate() {
        let is_last = i == keys.len() - 1;
        let map = match cur.as_object_mut() {
            Some(m) => m,
            None => return,
        };
        if is_last {
            map.insert((*key).to_string(), value);
            return;
        }
        let entry = map
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if !entry.is_object() {
            *entry = Value::Object(serde_json::Map::new());
        }
        cur = entry;
    }
}

/// Validate `projectType` against the freeform 1-30 character rule. Returns an
/// actionable error string when invalid, `None` when acceptable. Verbatim port
/// of `validateProjectTypeLength` (`src/commands/update-foundation.ts:247-259`).
fn validate_project_type_length(content: &str) -> Option<String> {
    let length = content.chars().count();
    if length == 0 {
        return Some(
            "Invalid projectType: \"\" (must be 1-30 characters, got 0). Fix: fspec update-foundation projectType \"<short-descriptor>\""
                .to_string(),
        );
    }
    if length > 30 {
        return Some(format!(
            "Invalid projectType: too long (must be 1-30 characters, got {length}). Fix: fspec update-foundation projectType \"<short-descriptor>\""
        ));
    }
    None
}

/// Validate `problemImpact` against the fixed enum (high, medium, low).
/// Returns an actionable error string when invalid, `None` when acceptable.
/// Verbatim port of `validateProblemImpact`
/// (`src/commands/update-foundation.ts:268-277`).
fn validate_problem_impact(content: &str) -> Option<String> {
    const VALID: [&str; 3] = ["high", "medium", "low"];
    if content.is_empty() {
        return Some(
            "Invalid value for problemImpact: \"\". Valid values: high, medium, low. Fix: fspec update-foundation problemImpact \"<valid-value>\""
                .to_string(),
        );
    }
    if !VALID.contains(&content) {
        return Some(format!(
            "Invalid value for problemImpact: \"{content}\". Valid values: high, medium, low. Fix: fspec update-foundation problemImpact \"<valid-value>\""
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::foundation::guidance;

    #[test]
    fn args_parse_minimal() {
        let a: UpdateFoundationArgs =
            serde_json::from_str(r#"{"section":"projectName","content":"x"}"#).unwrap();
        assert_eq!(a.section, "projectName");
        assert_eq!(a.content, "x");
    }

    #[test]
    fn set_nested_creates_intermediate_objects() {
        let mut v = json!({});
        set_nested(
            &mut v,
            &["problemSpace", "primaryProblem", "title"],
            Value::String("T".to_string()),
        );
        assert_eq!(
            v["problemSpace"]["primaryProblem"]["title"].as_str(),
            Some("T")
        );
    }

    #[test]
    fn update_json_field_known_aliases() {
        let mut v = json!({});
        assert!(update_json_field(&mut v, "projectName", "Acme"));
        assert_eq!(v["project"]["name"].as_str(), Some("Acme"));
        assert!(update_json_field(&mut v, "solutionOverview", "O"));
        assert_eq!(v["solutionSpace"]["overview"].as_str(), Some("O"));
    }

    #[test]
    fn update_json_field_unknown_section_returns_false() {
        let mut v = json!({});
        assert!(!update_json_field(&mut v, "bogusSection", "x"));
    }

    #[test]
    fn project_type_length_bounds() {
        assert!(validate_project_type_length("").is_some());
        assert!(validate_project_type_length("cli-tool").is_none());
        let too_long = "x".repeat(31);
        assert!(validate_project_type_length(&too_long).is_some());
    }

    #[test]
    fn problem_impact_enum() {
        assert!(validate_problem_impact("high").is_none());
        assert!(validate_problem_impact("urgent").is_some());
        assert!(validate_problem_impact("").is_some());
    }

    #[test]
    fn scan_picks_first_placeholder_field() {
        let draft = json!({
            "project": { "name": "n", "vision": "[QUESTION: vision?]", "projectType": "cli-tool" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
            "solutionSpace": { "overview": "o", "capabilities": ["c"] },
            "personas": [ { "name": "p", "description": "d", "goals": ["g"] } ]
        });
        let r = guidance::next_field_reminder(&draft, Path::new("/nonexistent")).unwrap();
        assert!(r.starts_with("<system-reminder>\n"));
        assert!(r.ends_with("\n</system-reminder>"));
        assert!(r.contains("Field 2/8: project.vision (elevator pitch)"));
        // default agent (no FSPEC_AGENT, no config) → "Think a lot"
        assert!(r.contains("Think a lot about the entire codebase."));
        assert!(!r.contains("ULTRATHINK"));
    }

    #[test]
    fn scan_all_complete_returns_none() {
        let draft = json!({
            "project": { "name": "n", "vision": "v", "projectType": "cli-tool" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
            "solutionSpace": { "overview": "o", "capabilities": ["c"] },
            "personas": [ { "name": "p", "description": "d", "goals": ["g"] } ]
        });
        assert!(guidance::next_field_reminder(&draft, Path::new("/nonexistent")).is_none());
    }

    #[test]
    fn scan_project_type_detected_prefix() {
        let draft = json!({
            "project": { "name": "n", "vision": "v", "projectType": "[DETECTED: web-app]" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
            "solutionSpace": { "overview": "o", "capabilities": ["c"] },
            "personas": [ { "name": "p", "description": "d", "goals": ["g"] } ]
        });
        let r = guidance::next_field_reminder(&draft, Path::new("/nonexistent")).unwrap();
        assert!(r.contains("Field 3/8: project.projectType"));
        assert!(r.contains("[DETECTED: web-app] Analyze codebase"));
    }

    #[test]
    fn missing_field_is_skipped_not_placeholder() {
        // project.name absent → skipped; project.vision has placeholder → next.
        let draft = json!({
            "project": { "vision": "[QUESTION: vision?]", "projectType": "cli-tool" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
            "solutionSpace": { "overview": "o", "capabilities": ["c"] },
            "personas": [ { "name": "p", "description": "d", "goals": ["g"] } ]
        });
        let r = guidance::next_field_reminder(&draft, Path::new("/nonexistent")).unwrap();
        assert!(r.contains("Field 2/8: project.vision"));
    }

    #[test]
    fn detected_value_extraction_works() {
        assert_eq!(
            guidance::extract_detected_value("[DETECTED: web-app]").as_deref(),
            Some("web-app")
        );
        assert_eq!(
            guidance::extract_detected_value("[DETECTED:  cli-tool ]").as_deref(),
            Some("cli-tool")
        );
        assert_eq!(guidance::extract_detected_value("no marker"), None);
    }

    #[test]
    fn unknown_agent_id_falls_through_to_default() {
        // is_known_agent rejects unknowns so resolver returns false.
        assert!(!guidance::is_known_agent("bogus"));
        assert!(guidance::is_known_agent("claude"));
        assert!(guidance::is_known_agent("antigravity"));
    }
}
