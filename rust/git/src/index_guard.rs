//! BUG-696: gix-index 0.40.1 `File::at` slices the trailing checksum
//! (`data[data.len() - hash_len..]`) before it checks the file length, so a
//! `.git/index` shorter than its checksum panics with subtract-with-overflow.
//! Every codelet-git call that reads the index checks its length here first.
//! Upstream gix-index main (0.58.0) checks the length itself: remove this
//! guard when fspec upgrades gix.

use crate::{GitError, Result};
use std::path::Path;

/// The index header: signature, version and entry count, 4 bytes each.
const HEADER_LEN: u64 = 12;

/// Refuses an index file too short to hold its header and checksum with a
/// CorruptedIndex error naming the path and its length. A missing file
/// passes: each caller gives a missing index its own meaning.
pub(crate) fn ensure_index_not_truncated(path: &Path, hash: gix::hash::Kind) -> Result<()> {
    let len = match std::fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let minimum = HEADER_LEN + hash.len_in_bytes() as u64;
    if len < minimum {
        return Err(GitError::CorruptedIndex {
            message: format!(
                "the git index '{}' is {len} bytes, shorter than the {minimum} bytes of an \
                 index header and checksum; it is truncated",
                path.display()
            ),
        });
    }
    Ok(())
}

/// The repository's index, or an empty one when it has no index file
/// (BUG-637), after the BUG-696 length check.
pub(crate) fn index_or_empty(
    repo: &gix::Repository,
) -> Result<gix::worktree::Index> {
    ensure_index_not_truncated(&repo.index_path(), repo.object_hash())?;
    repo.index_or_empty()
        .map_err(|e| GitError::Status(e.to_string()))
}
