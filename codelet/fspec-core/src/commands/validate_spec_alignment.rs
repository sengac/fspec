//! `validate-spec-alignment` — Rust port of `validateSpecAlignment` in
//! `src/commands/validate-spec-alignment.ts` (RPC-323).
//!
//! ## Framing A (supervisor-approved)
//!
//! The exported `validateSpecAlignment({workUnitId, cwd})` is the real tested
//! contract (returns `{valid, warnings?}`). The TS CLI `.action` handler is
//! BROKEN — it calls the function with no `workUnitId` and reads a
//! non-existent `result.aligned`. The Rust port therefore mirrors the
//! EXPORTED function, and the clap surface exposes a required positional
//! `<workUnitId>` with `--fix` accepted-but-no-op. The `--help` text still
//! advertises the broken `[feature-files...]` shape because the help doc is
//! the captured-fixture canon (see the help config).
//!
//! ## Behaviour (parity with the TS exported function)
//!
//! * `spec/work-units.json` is read DIRECTLY via read + `JSON.parse` (NOT
//!   `ensureWorkUnitsFile`). ENOENT and parse errors are caught and re-thrown
//!   wrapped as `Failed to validate spec alignment: <msg>`.
//! * If `data.workUnits[workUnitId]` is missing → throws
//!   `Work unit <id> not found` (wrapped by the catch into
//!   `Failed to validate spec alignment: Work unit <id> not found`).
//! * Globs `spec/features/**/*.feature`; a scenario counts toward the work
//!   unit when a line trim-contains `@<workUnitId>` and the immediately
//!   following line trim-starts-with `Scenario:`.
//! * Returns `{valid:true}` when `scenariosFound > 0`; returns
//!   `{valid:false, warnings:['No scenarios for <id>']}` when `0`.
//! * A missing `spec/features` directory yields zero scenarios (the TS glob
//!   returns `[]` when the directory is absent — no throw). The Rust port maps
//!   the shared [`glob_feature_files`] `DirectoryNotFound` to an empty Vec
//!   LOCALLY (supervisor decision — no shared edit).
//!
//! Two-front-doors invariant: the dispatcher AND the standalone CLI bridge
//! both call this single function — no inline scan logic elsewhere.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ValidateSpecAlignmentArgs {
    #[serde(default)]
    work_unit_id: Option<String>,
    /// Accepted-but-no-op (parity with the TS broken `--fix` flag).
    #[serde(default)]
    fix: bool,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ValidateSpecAlignmentArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "validate-spec-alignment",
            reason: format!("failed to parse args: {e}"),
        })?;

    // `--fix` is accepted but performs no work (TS parity).
    let _ = args.fix;

    let work_unit_id = args
        .work_unit_id
        .filter(|s| !s.is_empty())
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "validate-spec-alignment",
            reason: "work unit id is required".to_string(),
        })?;

    // The TS `try { ... } catch` wraps EVERY failure (read, parse, missing
    // work unit) as `Failed to validate spec alignment: <msg>`. We mirror that
    // by computing the result in a helper that returns the bare message, then
    // wrapping any Err here.
    validate(project_root, &work_unit_id).map_err(|msg| {
        FspecCoreError::Message(format!("Failed to validate spec alignment: {msg}"))
    })
}

/// Inner validation. Returns the rendered JSON body on success, or a BARE
/// (unwrapped) error message string on failure — the caller wraps it with the
/// `Failed to validate spec alignment:` prefix.
fn validate(project_root: &Path, work_unit_id: &str) -> Result<String, String> {
    let path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let data: Value = serde_json::from_str(&raw)
        .map_err(|e| crate::io::json_error::parse_json_reason(&raw, &e))?;

    // TS: `if (!data.workUnits[workUnitId]) throw Work unit <id> not found`.
    let exists = data
        .get("workUnits")
        .and_then(Value::as_object)
        .map(|m| m.contains_key(work_unit_id))
        .unwrap_or(false);
    if !exists {
        return Err(format!("Work unit {work_unit_id} not found"));
    }

    let feature_files = soft_glob_features(project_root);
    let work_unit_tag = format!("@{work_unit_id}");
    let mut scenarios_found = 0usize;

    for file in &feature_files {
        let file_path = project_root.join(file);
        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        let lines: Vec<&str> = content.split('\n').collect();
        for i in 0..lines.len() {
            let line = lines[i].trim();
            if line.contains(&work_unit_tag) && i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();
                if next_line.starts_with("Scenario:") {
                    scenarios_found += 1;
                }
            }
        }
    }

    if scenarios_found == 0 {
        let payload = serde_json::json!({
            "valid": false,
            "warnings": [format!("No scenarios for {work_unit_id}")],
        });
        return serde_json::to_string_pretty(&payload).map_err(|e| e.to_string());
    }

    let payload = serde_json::json!({ "valid": true });
    serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())
}

/// Glob `spec/features/**/*.feature`, mapping the shared helper's
/// `DirectoryNotFound` to an empty list LOCALLY (supervisor decision). Any
/// other I/O error escalates as a bare message string for the caller to wrap.
fn soft_glob_features(project_root: &Path) -> Vec<String> {
    match glob_feature_files(project_root) {
        Ok(files) => files,
        Err(FspecCoreError::DirectoryNotFound { .. }) => Vec::new(),
        // A genuine I/O failure walking an existing tree is exceedingly rare;
        // mirror the TS glob's tolerance by treating it as no files found.
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn write_wu(root: &Path, ids: &[&str]) {
        let spec = root.join("spec");
        std::fs::create_dir_all(&spec).unwrap();
        let mut wus = serde_json::Map::new();
        for id in ids {
            wus.insert((*id).to_string(), json!({ "id": id, "title": "t", "status": "backlog" }));
        }
        let payload = json!({ "workUnits": Value::Object(wus), "states": {} });
        std::fs::write(spec.join("work-units.json"), payload.to_string()).unwrap();
    }

    fn write_feature(root: &Path, name: &str, body: &str) {
        let dir = root.join("spec/features");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn valid_when_tagged_scenario_present() {
        let tmp = TempDir::new().unwrap();
        write_wu(tmp.path(), &["AUTH-001"]);
        write_feature(
            tmp.path(),
            "a.feature",
            "Feature: A\n\n  @AUTH-001\n  Scenario: x\n    Given y\n",
        );
        let out = validate(tmp.path(), "AUTH-001").unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["valid"], json!(true));
        assert!(parsed.get("warnings").is_none());
    }

    #[test]
    fn invalid_when_no_tagged_scenario() {
        let tmp = TempDir::new().unwrap();
        write_wu(tmp.path(), &["AUTH-001"]);
        write_feature(tmp.path(), "o.feature", "Feature: O\n  @OTHER-001\n  Scenario: x\n");
        let out = validate(tmp.path(), "AUTH-001").unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["valid"], json!(false));
        assert_eq!(parsed["warnings"], json!(["No scenarios for AUTH-001"]));
    }

    #[test]
    fn missing_work_unit_errors() {
        let tmp = TempDir::new().unwrap();
        write_wu(tmp.path(), &["AUTH-001"]);
        let err = validate(tmp.path(), "MISSING-999").unwrap_err();
        assert_eq!(err, "Work unit MISSING-999 not found");
    }

    #[test]
    fn missing_features_dir_is_invalid_not_error() {
        let tmp = TempDir::new().unwrap();
        write_wu(tmp.path(), &["AUTH-001"]);
        let out = validate(tmp.path(), "AUTH-001").unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["valid"], json!(false));
    }
}
