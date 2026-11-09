//! BOARD-024: canonical full work-unit record reader.
//!
//! Feature: spec/features/board-card-details-canonical-backend.feature
//!
//! Reads the complete raw `workUnits` record from the daemon's attached
//! workspace on every request (never the cached board snapshot, the CLI
//! projection or the client's current directory) and returns the
//! `{id, workspace, workUnit}` envelope as a JSON string so the value
//! crosses the bincode wire unchanged. Every failure is an explicit,
//! contextual error naming the category, the store and the card.

use std::path::Path;

use serde_json::{json, Value};

/// Relative location of the canonical work-unit store inside a workspace.
const STORE: &str = "spec/work-units.json";

/// Load the canonical record for `id` from `workspace` and return the
/// serialized `{id, workspace, workUnit}` envelope.
pub fn load(workspace: Option<&Path>, id: &str) -> Result<String, String> {
    if id.trim().is_empty() {
        return Err("invalid request: work unit id is empty".to_owned());
    }
    let workspace = workspace.ok_or_else(|| {
        format!("daemon workspace unavailable for work unit {id}: no workspace is attached")
    })?;
    let workspace = workspace.canonicalize().map_err(|error| {
        format!(
            "daemon workspace unavailable for work unit {id}: {}: {error}",
            workspace.display()
        )
    })?;
    let store = workspace.join(STORE);
    let text = std::fs::read_to_string(&store).map_err(|error| {
        let category = if error.kind() == std::io::ErrorKind::NotFound {
            "work unit store missing (not found)"
        } else {
            "work unit store read failed"
        };
        format!(
            "{category} for {id}: cannot read {}: {error}",
            store.display()
        )
    })?;
    let root: Value = serde_json::from_str(&text).map_err(|error| {
        format!(
            "invalid JSON (parse failed) in work unit store for {id}: {}: {error}",
            store.display()
        )
    })?;
    let invalid = |detail: &str| format!("invalid store for {id}: {}: {detail}", store.display());
    let units = root
        .as_object()
        .ok_or_else(|| invalid("root is not an object"))?
        .get("workUnits")
        .ok_or_else(|| invalid("workUnits is missing"))?
        .as_object()
        .ok_or_else(|| invalid("workUnits is not an object"))?;
    let record = units
        .get(id)
        .ok_or_else(|| format!("work unit {id} missing (not found) in {}", store.display()))?;
    if !record.is_object() {
        return Err(invalid(&format!("work unit {id} is not an object")));
    }
    let workspace = workspace.to_str().ok_or_else(|| {
        format!("daemon workspace unavailable for work unit {id}: path is not UTF-8")
    })?;
    let envelope = json!({"id": id, "workspace": workspace, "workUnit": record});
    serde_json::to_string(&envelope)
        .map_err(|error| format!("cannot serialize work unit {id} details: {error}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn empty_identifier_and_missing_workspace_are_explicit() {
        assert!(load(None, " ").unwrap_err().contains("empty"));
        let error = load(None, "AUTH-001").unwrap_err();
        assert!(error.contains("workspace unavailable") && error.contains("AUTH-001"));
    }

    #[test]
    fn complete_record_round_trips_inside_the_envelope() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("spec")).unwrap();
        let record = json!({"id": "A-1", "future": {"x": [1, null]}});
        std::fs::write(
            root.path().join(STORE),
            json!({"workUnits": {"A-1": record}}).to_string(),
        )
        .unwrap();
        let envelope: Value =
            serde_json::from_str(&load(Some(root.path()), "A-1").unwrap()).unwrap();
        assert_eq!(envelope["workUnit"], record);
        assert_eq!(envelope["id"], "A-1");
    }
}
