//! Command history tracking.
//!
//! Lifted in RPC-025 from `codelet/napi/src/persistence/history.rs` so
//! both the NAPI surface and codelet_rpc can delegate to a single
//! implementation. The on-disk JSONL file at
//! `codelet_common::get_data_dir().join("history.jsonl")` is the SINGLE
//! source of truth — the existing NAPI exports
//! (`persistence_add_history`, `persistence_get_history`,
//! `persistence_search_history`) and the new FspecService RPC methods
//! both call into this module.

use chrono::{DateTime, Utc};
use codelet_rpc_types::{HistoryMatch, SessionId};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

/// Content that may be pasted by user. Lifted alongside `HistoryEntry`
/// so the JSONL on-disk format stays byte-identical with the pre-RPC-025
/// NAPI-only world.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PastedContent {
    /// Small pasted content, stored inline.
    Inline(String),
    /// Large pasted content, stored as a blob reference.
    BlobRef { hash: String, size_bytes: u64 },
}

/// A command history entry.
///
/// Lifted unchanged from `codelet/napi/src/persistence/types.rs` so the
/// existing JSONL on-disk format and JSON serialisation stay
/// byte-identical.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// The command/input that was entered.
    pub display: String,
    /// When the command was entered.
    pub timestamp: DateTime<Utc>,
    /// Which project this was entered in.
    pub project: PathBuf,
    /// Which session this was entered in.
    pub session_id: Uuid,
    /// Any pasted content (stored separately if large).
    #[serde(default)]
    pub pasted_content: Option<PastedContent>,
    /// RPC-025: preserves the original string form of `SessionId` for
    /// non-UUID callers (e.g. the RPC frontend). `None` for entries
    /// written via the legacy NAPI surface or read from the historical
    /// JSONL file — in which case `to_history_match` falls back to the
    /// Uuid's string representation. Skipped on serialisation when
    /// `None` so the JSONL on-disk format stays byte-identical for
    /// legacy callers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id_str: Option<String>,
}

impl HistoryEntry {
    /// Create a new history entry.
    pub fn new(display: String, project: PathBuf, session_id: Uuid) -> Self {
        Self {
            display,
            timestamp: Utc::now(),
            project,
            session_id,
            pasted_content: None,
            session_id_str: None,
        }
    }

    /// Create a new history entry with pasted content.
    pub fn with_pasted_content(
        display: String,
        project: PathBuf,
        session_id: Uuid,
        pasted: PastedContent,
    ) -> Self {
        Self {
            display,
            timestamp: Utc::now(),
            project,
            session_id,
            pasted_content: Some(pasted),
            session_id_str: None,
        }
    }

    /// RPC-025: alternative constructor that preserves the original
    /// `SessionId` string form for non-UUID callers. The `session_id`
    /// argument is the SessionId string as supplied by the RPC caller;
    /// the `uuid_seed` is the Uuid representation used for legacy
    /// JSONL compatibility (e.g. parsed-from-string-or-nil).
    pub fn with_session_id_str(
        display: String,
        project: PathBuf,
        uuid_seed: Uuid,
        session_id: String,
    ) -> Self {
        Self {
            display,
            timestamp: Utc::now(),
            project,
            session_id: uuid_seed,
            pasted_content: None,
            session_id_str: Some(session_id),
        }
    }

    /// RPC-025: convert into a transport-portable `HistoryMatch` for use
    /// across the FspecService boundary. The timestamp is serialised via
    /// RFC3339 so non-Rust transport consumers don't need a chrono dep.
    pub fn to_history_match(&self) -> HistoryMatch {
        let sid = self
            .session_id_str
            .clone()
            .unwrap_or_else(|| self.session_id.to_string());
        HistoryMatch {
            session_id: SessionId::new(&sid),
            text: self.display.clone(),
            timestamp_iso: self.timestamp.to_rfc3339(),
        }
    }
}

/// Command history store. Backed by an append-only JSONL file.
pub struct HistoryStore {
    history_file: PathBuf,
    entries: Vec<HistoryEntry>,
}

impl HistoryStore {
    /// Create a new history store rooted at
    /// `codelet_common::get_data_dir().join("history.jsonl")`.
    pub fn new() -> Result<Self, String> {
        let history_file = codelet_common::get_data_dir()?.join("history.jsonl");
        let mut store = Self {
            history_file,
            entries: Vec::new(),
        };
        store.load()?;
        Ok(store)
    }

    fn load(&mut self) -> Result<(), String> {
        if !self.history_file.exists() {
            return Ok(());
        }
        let file = File::open(&self.history_file)
            .map_err(|e| format!("Failed to open history file: {e}"))?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {e}"))?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<HistoryEntry>(&line) {
                Ok(entry) => self.entries.push(entry),
                Err(e) => {
                    tracing::warn!("Skipping corrupted history entry: {}", e);
                }
            }
        }
        self.entries.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        Ok(())
    }

    /// Append a new entry — writes to disk and inserts at index 0.
    pub fn add(&mut self, entry: HistoryEntry) -> Result<(), String> {
        if let Some(parent) = self.history_file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create history dir: {e}"))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.history_file)
            .map_err(|e| format!("Failed to open history file: {e}"))?;
        let json = serde_json::to_string(&entry)
            .map_err(|e| format!("Failed to serialize history entry: {e}"))?;
        writeln!(file, "{json}").map_err(|e| format!("Failed to write history entry: {e}"))?;
        self.entries.insert(0, entry);
        Ok(())
    }

    /// Return entries in newest-first order, optionally filtered by project + capped at `limit`.
    pub fn get(&self, project: Option<&Path>, limit: Option<usize>) -> Vec<HistoryEntry> {
        let iter = self.entries.iter().cloned();
        let filtered: Vec<HistoryEntry> = if let Some(proj) = project {
            iter.filter(|e| e.project == proj).collect()
        } else {
            iter.collect()
        };
        match limit {
            Some(n) => filtered.into_iter().take(n).collect(),
            None => filtered,
        }
    }

    /// Case-insensitive substring search on `display`, optionally
    /// scoped to one project.
    pub fn search(&self, query: &str, project: Option<&Path>) -> Vec<HistoryEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                let matches_query = e.display.to_lowercase().contains(&q);
                let matches_project = project.is_none_or(|p| e.project == p);
                matches_query && matches_project
            })
            .cloned()
            .collect()
    }
}

// ─── Module-level helpers — same signatures as the lifted NAPI helpers ───

static HISTORY_STORE: Mutex<Option<HistoryStore>> = Mutex::new(None);

fn with_store<R>(f: impl FnOnce(&mut HistoryStore) -> R) -> Result<R, String> {
    let mut guard = HISTORY_STORE.lock().map_err(|e| e.to_string())?;
    // RPC-025: re-init when the configured DATA_DIRECTORY has changed
    // since the last load (e.g. tests swap the data dir between cases).
    // The store's `history_file` is captured once on construction so a
    // mismatch with the live data dir means a stale cache from a
    // previous tempdir would leak into this call.
    let live_path = codelet_common::get_data_dir()?.join("history.jsonl");
    let stale = matches!(&*guard, Some(store) if store.history_file != live_path);
    if stale {
        *guard = None;
    }
    if guard.is_none() {
        *guard = Some(HistoryStore::new()?);
    }
    let store = guard.as_mut().ok_or("history store not initialized")?;
    Ok(f(store))
}

/// Reset the in-memory cache — used by tests that swap the
/// `DATA_DIRECTORY` between cases so a stale cache from a previous temp
/// dir doesn't leak into the next test.
pub fn reset_for_tests() {
    if let Ok(mut guard) = HISTORY_STORE.lock() {
        *guard = None;
    }
}

/// Test-only: return whether HISTORY_STORE has been initialized. Used
/// by lazy-init regression tests in the NAPI crate that need to observe
/// the per-store init semantics across the RPC-025 lift.
pub fn is_initialized_for_tests() -> bool {
    HISTORY_STORE.lock().map(|g| g.is_some()).unwrap_or(false)
}

/// Append a history entry.
pub fn add(entry: HistoryEntry) -> Result<(), String> {
    with_store(|s| s.add(entry))?
}

/// Return entries newest-first, optionally filtered by project + capped.
pub fn get(project: Option<&Path>, limit: Option<usize>) -> Result<Vec<HistoryEntry>, String> {
    with_store(|s| s.get(project, limit))
}

/// Case-insensitive substring search on display, optionally scoped to project.
pub fn search(query: &str, project: Option<&Path>) -> Result<Vec<HistoryEntry>, String> {
    with_store(|s| s.search(query, project))
}
