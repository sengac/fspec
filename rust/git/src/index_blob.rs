//! COV-077: files as they are staged — the paths of the git index and the
//! content of their blobs — so a commit gate can read exactly what will be
//! committed, whatever the working tree holds.

use std::collections::BTreeMap;
use std::path::Path;

use crate::{open_repo, GitError, Result};

/// The index of a repository, read once: each staged path (repo-relative,
/// forward slashes) and its blob. A path staged as deleted is absent.
pub struct StagedFiles {
    repo: gix::Repository,
    blobs: BTreeMap<String, gix::ObjectId>,
}

impl StagedFiles {
    /// Read the index of the repository whose working tree is `dir`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let repo = open_repo(dir)?;
        // BUG-637: a working tree with no index file reads as an empty index.
        let index = crate::index_guard::index_or_empty(&repo)?;
        let mut blobs = BTreeMap::new();
        for entry in index.entries() {
            let path = String::from_utf8_lossy(entry.path(&index)).to_string();
            blobs.entry(path).or_insert(entry.id);
        }
        Ok(StagedFiles { repo, blobs })
    }

    /// Every staged path, sorted.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.blobs.keys().map(String::as_str)
    }

    /// Whether `path` is staged.
    pub fn contains(&self, path: &str) -> bool {
        self.blobs.contains_key(path)
    }

    /// The staged content of `path`; `None` when it is not staged.
    pub fn read(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let Some(id) = self.blobs.get(path) else {
            return Ok(None);
        };
        let object = self.repo.find_object(*id).map_err(|e| GitError::ReadBlob {
            path: path.to_string(),
            source: e.into(),
        })?;
        Ok(Some(object.into_blob().data.clone()))
    }
}
