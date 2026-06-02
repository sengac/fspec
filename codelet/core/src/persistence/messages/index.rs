//! Binary index file I/O for the message store (BUG-122 Layer 2).
//!
//! Lifted from `codelet/napi/src/persistence/message_index.rs` in
//! RPC-032 so the on-disk index format is owned by codelet-core and
//! reachable from non-NAPI crates (codelet-rpc-embedded,
//! codelet-sessions, codelet-fspec) without re-introducing the
//! forbidden `rpc → napi` arrow.
//!
//! Format of `messages.idx`:
//! ```text
//! [magic:          4 bytes "MIDX"]
//! [version:        4 bytes u32 LE = 1]
//! [data_file_size: 8 bytes u64 LE]  — for staleness detection
//! [entry_count:    4 bytes u32 LE]
//! [entries:        entry_count × 28 bytes each]
//!   per entry:
//!     [uuid:        16 bytes raw]
//!     [byte_offset:  8 bytes u64 LE]
//!     [byte_length:  4 bytes u32 LE]
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use tracing::warn;
use uuid::Uuid;

use super::StoredMessage;

/// Magic bytes identifying a messages index file
const INDEX_MAGIC: &[u8; 4] = b"MIDX";

/// Current binary format version
const INDEX_VERSION: u32 = 1;

/// Size of the fixed header: magic(4) + version(4) + data_file_size(8) + entry_count(4)
const HEADER_SIZE: usize = 20;

/// Size of one index entry: uuid(16) + byte_offset(8) + byte_length(4)
const ENTRY_SIZE: usize = 28;

/// On-disk index entry for a stored message.
#[derive(Debug, Clone, Copy)]
pub(super) struct IndexEntry {
    pub byte_offset: u64,
    pub byte_length: u32,
}

/// Load the binary index file, returning the index map and the recorded data file size.
///
/// Returns `None` if the file doesn't exist or is corrupt/incompatible.
pub(super) fn load_index(index_path: &Path) -> Option<(HashMap<Uuid, IndexEntry>, u64)> {
    let mut file = File::open(index_path).ok()?;

    let mut header = [0u8; HEADER_SIZE];
    if file.read_exact(&mut header).is_err() {
        return None;
    }

    // Validate magic
    if &header[0..4] != INDEX_MAGIC {
        return None;
    }

    // Validate version
    let version = u32::from_le_bytes(header[4..8].try_into().ok()?);
    if version != INDEX_VERSION {
        return None;
    }

    let data_file_size = u64::from_le_bytes(header[8..16].try_into().ok()?);
    let entry_count = u32::from_le_bytes(header[16..20].try_into().ok()?) as usize;

    let mut entries = HashMap::with_capacity(entry_count);
    let mut buf = [0u8; ENTRY_SIZE];

    for _ in 0..entry_count {
        if file.read_exact(&mut buf).is_err() {
            warn!("Truncated index file, rebuilding");
            return None;
        }
        let uuid = Uuid::from_bytes(buf[0..16].try_into().ok()?);
        let byte_offset = u64::from_le_bytes(buf[16..24].try_into().ok()?);
        let byte_length = u32::from_le_bytes(buf[24..28].try_into().ok()?);
        entries.insert(uuid, IndexEntry { byte_offset, byte_length });
    }

    Some((entries, data_file_size))
}

/// Write the binary index file.
///
/// LOG-006 defence: the parent directory is (re)created with
/// `create_dir_all` before any I/O. `MessageStore::new()` already
/// creates `{data_dir}/messages/` once at construction time, but the
/// directory can disappear between construction and any later
/// `save_index` call (test cleanup, a sibling tool wiping `~/.fspec`,
/// `codelet_common::set_data_directory` swap leaving a stale handle
/// behind, etc.). Without this guard `File::create(temp_path)` or
/// `std::fs::rename` returns `ENOENT` and the message persist surfaces
/// as `Failed to rename index temp file: No such file or directory
/// (os error 2)` in the combined log. `create_dir_all` is a no-op when
/// the directory already exists.
pub(super) fn save_index(
    index_path: &Path,
    index: &HashMap<Uuid, IndexEntry>,
    data_file_size: u64,
) -> Result<(), String> {
    if let Some(parent) = index_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to ensure index parent dir {parent:?}: {e}"))?;
    }

    let temp_path = index_path.with_extension("idx.tmp");
    let mut file = File::create(&temp_path)
        .map_err(|e| format!("Failed to create index temp file: {e}"))?;

    // Header
    file.write_all(INDEX_MAGIC)
        .map_err(|e| format!("Failed to write index magic: {e}"))?;
    file.write_all(&INDEX_VERSION.to_le_bytes())
        .map_err(|e| format!("Failed to write index version: {e}"))?;
    file.write_all(&data_file_size.to_le_bytes())
        .map_err(|e| format!("Failed to write data file size: {e}"))?;
    file.write_all(&(index.len() as u32).to_le_bytes())
        .map_err(|e| format!("Failed to write entry count: {e}"))?;

    // Entries
    for (uuid, entry) in index {
        file.write_all(uuid.as_bytes())
            .map_err(|e| format!("Failed to write uuid: {e}"))?;
        file.write_all(&entry.byte_offset.to_le_bytes())
            .map_err(|e| format!("Failed to write byte_offset: {e}"))?;
        file.write_all(&entry.byte_length.to_le_bytes())
            .map_err(|e| format!("Failed to write byte_length: {e}"))?;
    }

    file.flush()
        .map_err(|e| format!("Failed to flush index file: {e}"))?;

    std::fs::rename(&temp_path, index_path)
        .map_err(|e| format!("Failed to rename index temp file: {e}"))?;

    Ok(())
}

/// Scan a JSONL data file from `start_offset` to EOF, building index entries.
///
/// For each valid JSON line, extracts the `"id"` UUID field and records the
/// byte offset and length. Returns the new entries and the final file position.
pub(super) fn scan_jsonl_range(
    data_path: &Path,
    start_offset: u64,
) -> Result<(HashMap<Uuid, IndexEntry>, u64), String> {
    let file = File::open(data_path)
        .map_err(|e| format!("Failed to open messages file for scanning: {e}"))?;
    let file_len = file.metadata()
        .map_err(|e| format!("Failed to get file metadata: {e}"))?
        .len();

    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(start_offset))
        .map_err(|e| format!("Failed to seek in messages file: {e}"))?;

    let mut entries = HashMap::new();
    let mut current_offset = start_offset;

    let mut line_buf = String::new();
    loop {
        line_buf.clear();
        let bytes_read = reader.read_line(&mut line_buf)
            .map_err(|e| format!("Failed to read line from messages file: {e}"))?;
        if bytes_read == 0 {
            break;
        }
        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            current_offset += bytes_read as u64;
            continue;
        }

        // Extract just the UUID without full deserialization for speed.
        // Fall back to full parse if the fast path fails.
        if let Some(uuid) = extract_uuid_fast(trimmed) {
            entries.insert(uuid, IndexEntry {
                byte_offset: current_offset,
                byte_length: bytes_read as u32,
            });
        } else {
            warn!("Skipping unparseable line at offset {current_offset}");
        }

        current_offset += bytes_read as u64;
    }

    Ok((entries, file_len))
}

/// Fast UUID extraction from a JSON line.
///
/// Looks for `"id":"<uuid>"` pattern to avoid full JSON parsing during scan.
fn extract_uuid_fast(line: &str) -> Option<Uuid> {
    // Try fast string search first
    let needle = "\"id\":\"";
    if let Some(pos) = line.find(needle) {
        let start = pos + needle.len();
        if start + 36 <= line.len() {
            let uuid_str = &line[start..start + 36];
            if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                return Some(uuid);
            }
        }
    }
    // Fallback: full JSON parse
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|v| v.get("id")?.as_str().map(String::from))
        .and_then(|s| Uuid::parse_str(&s).ok())
}

/// Read a single message from the data file at the given offset/length.
pub(super) fn read_message_at(
    data_path: &Path,
    entry: &IndexEntry,
) -> Result<StoredMessage, String> {
    let mut file = File::open(data_path)
        .map_err(|e| format!("Failed to open messages file: {e}"))?;
    file.seek(SeekFrom::Start(entry.byte_offset))
        .map_err(|e| format!("Failed to seek to message: {e}"))?;

    let mut buf = vec![0u8; entry.byte_length as usize];
    file.read_exact(&mut buf)
        .map_err(|e| format!("Failed to read message bytes: {e}"))?;

    serde_json::from_slice(&buf)
        .map_err(|e| format!("Failed to deserialize message: {e}"))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// LOG-006 regression: production logs at
    /// `~/.fspec/logs/fspec-combined.log` repeatedly contained
    ///
    ///   ERROR codelet_agent_loop::agent_loop: Failed to persist user
    ///     message for session …: Failed to rename index temp file:
    ///     No such file or directory (os error 2)
    ///
    /// `save_index` was assuming the parent directory always exists
    /// because `MessageStore::new()` created it once at construction
    /// time. In practice the directory can disappear between
    /// construction and a later persist call (test teardown, sibling
    /// tooling wiping `~/.fspec`, a `set_data_directory` swap leaving
    /// a stale handle behind). The fix is to defensively
    /// `create_dir_all(parent)` at the top of `save_index` so the
    /// `File::create(temp_path)` + `rename(...)` pair always lands in
    /// a directory that exists.
    #[test]
    fn save_index_recreates_missing_parent_directory() {
        // @step Given an index path whose parent directory does NOT exist
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("nested").join("messages");
        let index_path = nested.join("messages.idx");
        assert!(
            !nested.exists(),
            "precondition: the parent directory must not exist yet"
        );

        // @step When save_index is called with an empty index map
        let result = save_index(&index_path, &HashMap::new(), 0);

        // @step Then save_index returns Ok and the index file is written
        assert!(
            result.is_ok(),
            "save_index must create missing parent dirs — regressed LOG-006: {result:?}"
        );
        assert!(
            index_path.exists(),
            "messages.idx must exist after save_index"
        );
        assert!(
            nested.exists(),
            "the parent dir must have been created by save_index"
        );

        // @step And the temp file is no longer present (it was renamed onto the final path)
        let temp_path = index_path.with_extension("idx.tmp");
        assert!(
            !temp_path.exists(),
            "messages.idx.tmp must not linger after a successful save"
        );
    }

    /// LOG-006 follow-on: even if the parent dir was deleted between
    /// MessageStore::new() and a subsequent save_index call (the
    /// observed production pattern), the next save still succeeds
    /// because the `create_dir_all` at the top of `save_index`
    /// recreates the directory in-place.
    #[test]
    fn save_index_succeeds_after_parent_dir_was_deleted() {
        // @step Given a messages dir that already contains a valid index
        let tmp = TempDir::new().expect("tempdir");
        let messages_dir = tmp.path().join("messages");
        std::fs::create_dir_all(&messages_dir).expect("create messages_dir");
        let index_path = messages_dir.join("messages.idx");

        let mut index = HashMap::new();
        let uuid = Uuid::new_v4();
        index.insert(
            uuid,
            IndexEntry { byte_offset: 0, byte_length: 42 },
        );
        save_index(&index_path, &index, 42).expect("initial save");
        assert!(index_path.exists());

        // @step And the messages dir is deleted out from under the store
        std::fs::remove_dir_all(&messages_dir).expect("remove messages_dir");
        assert!(!messages_dir.exists());

        // @step When save_index is invoked again with the same path
        let result = save_index(&index_path, &index, 42);

        // @step Then save_index recreates the dir and writes the index
        assert!(
            result.is_ok(),
            "save_index must self-heal a deleted parent dir — regressed LOG-006: {result:?}"
        );
        assert!(messages_dir.exists(), "messages dir must be recreated");
        assert!(index_path.exists(), "messages.idx must be written");

        // @step And the on-disk header reports the supplied data_file_size
        let (loaded_index, data_size) =
            load_index(&index_path).expect("freshly written index must be loadable");
        assert_eq!(data_size, 42);
        assert_eq!(loaded_index.len(), 1);
        assert!(loaded_index.contains_key(&uuid));
    }
}
