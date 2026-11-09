//! RPC-025 / RPC-455: input history recorded and read per project
//! directory — shared by the start-workspace RPCs and their
//! `*_in_workspace` variants.

use std::path::{Path, PathBuf};

use codelet_core::persistence::{history, HistoryEntry};
use codelet_rpc_types::{HistoryMatch, SessionId};

/// Record `text` for `session` under `project`. The legacy JSONL format
/// stores a Uuid; a non-UUID session id is stored as the nil Uuid (that
/// column's documented legacy encoding) with the real id kept in
/// `session_id_str`, so nothing is lost.
pub(crate) fn add(project: PathBuf, session: SessionId, text: String) -> Result<(), String> {
    let legacy_uuid = uuid::Uuid::parse_str(&session.value).unwrap_or_else(|_| uuid::Uuid::nil());
    history::add(HistoryEntry::with_session_id_str(
        text,
        project,
        legacy_uuid,
        session.value,
    ))
}

/// The display values of the newest `limit` entries of `project` (every
/// project when `None`).
pub(crate) fn get(project: Option<&Path>, limit: u32) -> Result<Vec<String>, String> {
    let limit = usize::try_from(limit).map_err(|error| error.to_string())?;
    history::get(project, Some(limit))
        .map(|entries| entries.into_iter().map(|e| e.display).collect())
}

/// Entries of `project` (every project when `None`) matching `query`.
pub(crate) fn search(query: &str, project: Option<&Path>) -> Result<Vec<HistoryMatch>, String> {
    history::search(query, project)
        .map(|entries| entries.iter().map(HistoryEntry::to_history_match).collect())
}
