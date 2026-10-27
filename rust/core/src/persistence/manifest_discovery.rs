//! Shared, non-destructive manifest discovery for the store and RPC facade.
use super::manifest::SessionManifest;
use codelet_rpc_types::SessionManifestIssue;
use std::{fs, path::Path};
use uuid::Uuid;

/// A fresh, non-draining snapshot; it never initializes or repairs message storage.
pub fn session_manifest_issues() -> Result<Vec<SessionManifestIssue>, String> {
    let directory = codelet_common::get_data_dir()?.join("sessions");
    discover(&directory).map(|(_, issues)| issues)
}

pub(super) fn discover(
    directory: &Path,
) -> Result<(Vec<SessionManifest>, Vec<SessionManifestIssue>), String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((vec![], vec![])),
        Err(error) => {
            return Err(format!(
                "Failed to read sessions directory {directory:?}: {error}"
            ))
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| {
                format!("Failed to enumerate sessions directory {directory:?}: {error}")
            })?
            .path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
    }
    // Preserve the existing store traversal order; only issue snapshots are sorted.
    let mut sessions = Vec::new();
    let mut issues = Vec::new();
    for path in paths {
        match read_candidate(&path) {
            Ok(session) => sessions.push(session),
            Err(issue) => issues.push(issue),
        }
    }
    issues.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((sessions, issues))
}

fn read_candidate(path: &Path) -> Result<SessionManifest, SessionManifestIssue> {
    let bytes = fs::read(path).map_err(|error| issue(path, "unreadable", &error.to_string()))?;
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Err(issue(
            path,
            "empty",
            "Session manifest is empty or whitespace-only",
        ));
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        let category = if error.is_eof() {
            "truncated"
        } else if error.is_data() {
            "incompatible"
        } else {
            "malformed"
        };
        issue(path, category, &error.to_string())
    })
}

fn issue(path: &Path, category: &str, reason: &str) -> SessionManifestIssue {
    SessionManifestIssue {
        path: path.to_owned(),
        session_id: path.file_stem().and_then(|stem| stem.to_str())
            .and_then(|stem| stem.parse::<Uuid>().ok()).map(|id| id.to_string()),
        category: category.into(),
        reason: reason.into(),
        recovery: "Make a backup of this manifest and related session data before repair. Restore a known-good backup or open it with a compatible Fspec version. Preserve original history; do not delete it to dismiss this warning.".into(),
    }
}
