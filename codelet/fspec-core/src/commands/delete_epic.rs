//! `delete-epic` — Rust port of `src/commands/delete-epic.ts` (RPC-217).
//!
//! Reads `spec/epics.json` (auto-creating an empty store if missing, to
//! match the TS `fileManager.transaction` side-effect that creates the
//! file before the lookup); if the requested epic id is absent, returns
//! the canonical wrapped error `"Failed to delete epic: Epic <id> not found"`.
//! On hit, removes the epic and additionally dereferences:
//!
//! * Every `Prefix` in `spec/prefixes.json` whose `epicId` matches the
//!   deleted id loses that field (TS `delete prefix.epicId` at
//!   `src/commands/delete-epic.ts:62-63`).
//! * Every `WorkUnit` in `spec/work-units.json` whose `epic` matches the
//!   deleted id loses that field (TS `delete workUnit.epic` at
//!   `src/commands/delete-epic.ts:74-75`).
//!
//! Both side-effect files are wrapped in their own `try/catch` in the TS
//! source — any read error (ENOENT or parse-error) is silently swallowed
//! so a missing or malformed side-effect file does not block the epic
//! deletion. We mirror that bare-catch behaviour exactly.
//!
//! All writes go through [`crate::io::locked_file::write_json_atomic`]
//! (write-then-rename with `fs2` exclusive lock). Two-front-doors: both
//! the CLI bridge and the LLM dispatcher route through this single `run`.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

/// CLI arguments accepted by `delete-epic`. Mirrors the TS Commander.js
/// registration at `src/commands/delete-epic.ts:92-108`:
/// - positional `<epicId>` (required, kebab-case marshalled to camelCase)
/// - optional `--force` (parsed by clap but unused by the TS impl; we
///   accept it for parity but ignore it)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteEpicArgs {
    /// Epic ID to delete.
    epic_id: String,
    /// Parsed for parity with the TS Commander surface (`--force`) but
    /// never read by the implementation — TS treats `--force` as a
    /// declared-yet-unused flag at `src/commands/delete-epic.ts:97-99`.
    #[serde(default)]
    #[allow(dead_code)]
    force: Option<bool>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call `std::env::current_dir()`.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DeleteEpicArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "delete-epic",
            reason: format!("failed to parse args: {e}"),
        })?;

    let epics_path = project_root.join("spec").join("epics.json");

    // Read+modify+write spec/epics.json. TS uses `fileManager.transaction`
    // which auto-initializes a missing file with `{ epics: {} }` BEFORE
    // calling the mutator. We replicate this: read existing epics OR an
    // empty store, ensure the file exists on disk (matching the TS
    // side-effect that the file is created even when the lookup fails),
    // and then perform the lookup.
    let mut epics_top = read_json_object(&epics_path).unwrap_or_else(|| {
        let mut m = Map::new();
        m.insert("epics".to_string(), Value::Object(Map::new()));
        m
    });
    // Ensure top-level "epics" exists and is an object.
    if !epics_top
        .get("epics")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        epics_top.insert("epics".to_string(), Value::Object(Map::new()));
    }

    // If the file didn't exist, materialize the empty store on disk so
    // the TS transaction-creates-file side effect is preserved even on
    // the "not found" error path.
    if !epics_path.exists() {
        write_json_atomic(&epics_path, &Value::Object(epics_top.clone())).map_err(|e| {
            FspecCoreError::InvalidArgs {
                command: "delete-epic",
                reason: format!("Failed to delete epic: {e}"),
            }
        })?;
    }

    let epics_obj = epics_top
        .get_mut("epics")
        .and_then(Value::as_object_mut)
        .expect("ensured object above");

    if !epics_obj.contains_key(&args.epic_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-epic",
            reason: format!("Failed to delete epic: Epic {} not found", args.epic_id),
        });
    }

    // Remove the epic.
    epics_obj.remove(&args.epic_id);

    // Write epics.json back.
    write_json_atomic(&epics_path, &Value::Object(epics_top)).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "delete-epic",
            reason: format!("Failed to delete epic: {e}"),
        }
    })?;

    // Side-effect: dereference matching prefixes. Wrapped in best-effort
    // try/catch parity — read errors (ENOENT or parse-error) are
    // silently swallowed.
    let prefixes_path = project_root.join("spec").join("prefixes.json");
    let _ = mutate_dereference(
        &prefixes_path,
        "prefixes",
        "epicId",
        &args.epic_id,
    );

    // Side-effect: dereference matching work units. Same bare-catch
    // parity as above.
    let work_units_path = project_root.join("spec").join("work-units.json");
    let _ = mutate_dereference(
        &work_units_path,
        "workUnits",
        "epic",
        &args.epic_id,
    );

    Ok(format!("✓ Epic {} deleted successfully\n", args.epic_id))
}

/// Read a JSON file at `path` and return its root object. Returns `None`
/// if the file does not exist OR cannot be parsed as a JSON object (TS
/// bare-catch parity).
fn read_json_object(path: &Path) -> Option<Map<String, Value>> {
    let raw = std::fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    match v {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

/// Best-effort side-effect: open the JSON file at `path`, walk every
/// child of the top-level `outer_key` object, and `delete` the
/// `field_key` whenever its value equals `match_value`. Writes back
/// atomically only when at least one mutation occurred.
///
/// Returns `Ok(())` on success, an error if the write fails. The caller
/// is expected to ignore the result so any failure is silently swallowed
/// — matching the TS bare `catch {}` wrappers at
/// `src/commands/delete-epic.ts:57-68` and `:70-81`.
fn mutate_dereference(
    path: &PathBuf,
    outer_key: &str,
    field_key: &str,
    match_value: &str,
) -> Result<(), FspecCoreError> {
    let mut top = match read_json_object(path) {
        Some(t) => t,
        None => return Ok(()), // File missing or unparseable → silent skip.
    };

    let outer = match top.get_mut(outer_key).and_then(Value::as_object_mut) {
        Some(o) => o,
        None => return Ok(()), // Outer key absent or wrong shape → silent skip.
    };

    let mut mutated = false;
    for (_id, entry) in outer.iter_mut() {
        if let Some(entry_obj) = entry.as_object_mut() {
            if let Some(current) = entry_obj.get(field_key).and_then(Value::as_str) {
                if current == match_value {
                    entry_obj.remove(field_key);
                    mutated = true;
                }
            }
        }
    }

    if mutated {
        write_json_atomic(path, &Value::Object(top)).map_err(|e| FspecCoreError::Io {
            command: "delete-epic",
            source: std::io::Error::other(format!("write {}: {e}", path.display())),
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn args_parse_camel_case() {
        let a: DeleteEpicArgs =
            serde_json::from_str(r#"{"epicId":"auth"}"#).unwrap();
        assert_eq!(a.epic_id, "auth");
    }

    #[test]
    fn args_parse_accepts_force_for_parity() {
        let a: DeleteEpicArgs =
            serde_json::from_str(r#"{"epicId":"auth","force":true}"#).unwrap();
        assert_eq!(a.epic_id, "auth");
        assert_eq!(a.force, Some(true));
    }

    #[test]
    fn args_parse_fails_without_epic_id() {
        let err = serde_json::from_str::<DeleteEpicArgs>("{}").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("epicid"),
            "missing-field must mention epicId; got: {msg}"
        );
    }

    #[test]
    fn read_json_object_returns_none_on_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nope.json");
        assert!(read_json_object(&path).is_none());
    }

    #[test]
    fn read_json_object_returns_none_on_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(read_json_object(&path).is_none());
    }

    #[test]
    fn mutate_dereference_silently_skips_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("missing.json");
        let res = mutate_dereference(&path, "prefixes", "epicId", "auth");
        assert!(res.is_ok());
        assert!(!path.exists(), "must not create the file");
    }

    #[test]
    fn mutate_dereference_removes_matching_fields_and_writes_back() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("prefixes.json");
        std::fs::write(
            &path,
            r#"{
  "prefixes": {
    "AUTH": {"prefix":"AUTH","description":"x","epicId":"auth"},
    "OTHER": {"prefix":"OTHER","description":"x","epicId":"dash"}
  }
}"#,
        )
        .unwrap();
        mutate_dereference(&path, "prefixes", "epicId", "auth").unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(&raw).unwrap();
        assert!(v["prefixes"]["AUTH"].get("epicId").is_none());
        assert_eq!(v["prefixes"]["OTHER"]["epicId"].as_str(), Some("dash"));
    }
}
