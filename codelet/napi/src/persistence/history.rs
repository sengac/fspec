//! Command history tracking
//!
//! Stores command history in an append-only JSONL file.
//! History is ordered by timestamp and can be filtered by project.

use super::types::HistoryEntry;
use super::{ensure_directories, get_data_dir};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Command history store
pub struct HistoryStore {
    history_file: PathBuf,
    /// In-memory cache of history entries
    entries: Vec<HistoryEntry>,
}

impl HistoryStore {
    /// Create a new history store
    pub fn new() -> Result<Self, String> {
        ensure_directories()?;
        let history_file = get_data_dir()?.join("history.jsonl");
        let mut store = Self {
            history_file,
            entries: Vec::new(),
        };
        store.load()?;
        Ok(store)
    }

    /// Load history from disk
    fn load(&mut self) -> Result<(), String> {
        if !self.history_file.exists() {
            return Ok(());
        }

        let file = File::open(&self.history_file)
            .map_err(|e| format!("Failed to open history file: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<HistoryEntry>(&line) {
                Ok(entry) => self.entries.push(entry),
                Err(e) => {
                    // Log but don't fail on corrupted entries
                    tracing::warn!("Skipping corrupted history entry: {}", e);
                }
            }
        }

        // Sort by timestamp descending (most recent first)
        self.entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(())
    }

    /// Add a new history entry
    pub fn add(&mut self, entry: HistoryEntry) -> Result<(), String> {
        // Append to file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.history_file)
            .map_err(|e| format!("Failed to open history file: {}", e))?;

        let json = serde_json::to_string(&entry)
            .map_err(|e| format!("Failed to serialize history entry: {}", e))?;

        writeln!(file, "{}", json).map_err(|e| format!("Failed to write history entry: {}", e))?;

        // Insert at the beginning (most recent first)
        self.entries.insert(0, entry);

        Ok(())
    }

    /// Get history entries, optionally filtered by project
    ///
    /// Returns entries in reverse chronological order (most recent first).
    pub fn get(&self, project: Option<&Path>, limit: Option<usize>) -> Vec<&HistoryEntry> {
        let iter = self.entries.iter();

        let filtered: Vec<_> = if let Some(proj) = project {
            iter.filter(|e| e.project == proj).collect()
        } else {
            iter.collect()
        };

        match limit {
            Some(n) => filtered.into_iter().take(n).collect(),
            None => filtered,
        }
    }

    /// Search history entries by query string
    ///
    /// Returns entries where the display text contains the query.
    pub fn search(&self, query: &str, project: Option<&Path>) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();

        self.entries
            .iter()
            .filter(|e| {
                let matches_query = e.display.to_lowercase().contains(&query_lower);
                let matches_project = project.is_none_or(|p| e.project == p);
                matches_query && matches_project
            })
            .collect()
    }

    /// Get the number of history entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all history entries
    pub fn clear(&mut self) -> Result<(), String> {
        self.entries.clear();

        // Truncate the history file
        File::create(&self.history_file)
            .map_err(|e| format!("Failed to clear history file: {}", e))?;

        Ok(())
    }

    /// Get an iterator for navigating history
    pub fn iter(&self) -> impl Iterator<Item = &HistoryEntry> {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_entry(display: &str, project: &str) -> HistoryEntry {
        HistoryEntry::new(display.to_string(), PathBuf::from(project), Uuid::new_v4())
    }

    #[test]
    fn test_history_entry_new() {
        let entry = make_entry("test command", "/test/project");
        assert_eq!(entry.display, "test command");
        assert_eq!(entry.project, PathBuf::from("/test/project"));
    }
}
