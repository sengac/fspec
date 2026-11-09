//! Cross-process coverage sidecar transaction.
//!
//! BUG-551: a sidecar transaction holds the project's spec write lock
//! (`spec/.fspec-write.lock`, BUG-477), not a lock file of its own. The lock
//! file has to persist (deleting a flock file after unlocking lets a waiter
//! lock the unlinked inode while a newcomer locks a new file, so two writers
//! would run at once), so there is one persistent lock per spec root instead
//! of a `<name>.feature.coverage.lock` beside every coverage file. The OS
//! drops the lock when its holder exits, so a crashed writer never blocks
//! the next one. The sidecar itself is replaced atomically by rename. The
//! spec lock is reentrant per thread, so a command that already holds it for
//! the board takes the transaction without waiting.

use std::path::Path;

use crate::error::FspecCoreError;
use crate::io::spec_lock::{lock_dir, SpecLock};

/// Exclusive transaction over the coverage sidecars of one spec root;
/// released on drop.
pub(crate) struct SidecarTransaction {
    _lock: SpecLock,
}

impl SidecarTransaction {
    /// Block until the spec write lock of the project that owns
    /// `coverage_path` is held.
    pub(crate) fn acquire(
        coverage_path: &Path,
        command: &'static str,
    ) -> Result<Self, FspecCoreError> {
        Ok(Self {
            _lock: lock_dir(spec_dir(coverage_path, command)?, command)?,
        })
    }
}

/// The spec directory that owns a sidecar under `<spec>/features/**`.
fn spec_dir<'a>(
    coverage_path: &'a Path,
    command: &'static str,
) -> Result<&'a Path, FspecCoreError> {
    coverage_path
        .ancestors()
        .skip(1)
        .find(|ancestor| ancestor.file_name().is_some_and(|name| name == "features"))
        .and_then(Path::parent)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command,
            reason: format!(
                "{} is not under a spec/features directory, so it has no spec lock",
                coverage_path.display()
            ),
        })
}
