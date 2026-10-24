//! Binary index file I/O for the message store (BUG-122 Layer 2).
//!
//! Shared index format for the NAPI and standalone session stores.
//!
//! Format of `messages.idx`:
//! ```text
//! [magic:          4 bytes "MIDX"]
//! [version:        4 bytes u32 LE = 2]
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
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use tracing::warn;
use uuid::Uuid;

use super::StoredMessage;

mod write;
pub(super) use write::save_index;

/// Magic bytes identifying a messages index file
const INDEX_MAGIC: &[u8; 4] = b"MIDX";

/// Current binary format version
const INDEX_VERSION: u32 = 2;

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
        entries.insert(
            uuid,
            IndexEntry {
                byte_offset,
                byte_length,
            },
        );
    }

    Some((entries, data_file_size))
}

/// Scan a JSONL data file from `start_offset` to EOF, building index entries.
///
/// For each valid JSON line, extracts the `"id"` UUID field and records the
/// byte offset and length. Returns the new entries and the final file position.
pub(super) fn scan_jsonl_range(
    data_path: &Path,
    start_offset: u64,
) -> Result<(HashMap<Uuid, IndexEntry>, u64), String> {
    super::framing::scan_range(data_path, start_offset).map_err(|error| error.to_string())
}

/// Read a single message from the data file at the given offset/length.
pub(super) fn read_message_at(
    data_path: &Path,
    entry: &IndexEntry,
) -> Result<StoredMessage, String> {
    let mut file =
        File::open(data_path).map_err(|e| format!("Failed to open messages file: {e}"))?;
    file.seek(SeekFrom::Start(entry.byte_offset))
        .map_err(|e| format!("Failed to seek to message: {e}"))?;

    let mut buf = vec![0u8; entry.byte_length as usize];
    file.read_exact(&mut buf)
        .map_err(|e| format!("Failed to read message bytes: {e}"))?;

    super::framing::parse_record(&buf, entry.byte_offset).map_err(|error| error.to_string())
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
            IndexEntry {
                byte_offset: 0,
                byte_length: 42,
            },
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
