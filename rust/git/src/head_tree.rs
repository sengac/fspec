//! BUG-657: the HEAD tree, read once per capture.
//!
//! A changed-files capture compares every index entry with the HEAD tree.
//! Resolving each index path from the root tree (gix
//! `lookup_entry_by_path`) reads and inflates every tree on the path again,
//! and the repository has no object cache, so in a repository with 13,858
//! index entries one capture opened about 34,500 loose objects. This module
//! walks the HEAD tree once, reading each tree object exactly once (a
//! subtree that occurs at several paths, such as two identical attachment
//! directories, is read once by object id), into a path map the staged pass
//! checks every index entry against.
//!
//! Every tree read goes through [`ObjectSource`], so a test can count the
//! reads exactly; production passes the opened repository.

use std::collections::HashMap;
use std::rc::Rc;

use crate::error::{GitError, Result};

/// Where a capture reads its tree objects from.
pub trait ObjectSource {
    /// Read and decode the tree object `id`.
    fn read_tree(&self, id: &gix::oid) -> Result<gix::objs::Tree>;
}

impl ObjectSource for gix::Repository {
    fn read_tree(&self, id: &gix::oid) -> Result<gix::objs::Tree> {
        let tree = self
            .find_tree(id)
            .map_err(|e| GitError::Head(format!("cannot read tree {id}: {e}")))?;
        let decoded = tree
            .decode()
            .map_err(|e| GitError::Head(format!("cannot decode tree {id}: {e}")))?;
        Ok(decoded.into())
    }
}

/// One non-tree entry of the HEAD tree (a blob, a symlink or a gitlink).
#[derive(Debug, Clone)]
pub(crate) struct HeadEntry {
    /// Repo-relative path.
    pub path: String,
    /// The object the entry points at.
    pub id: gix::ObjectId,
    /// A submodule entry: compared with the index, never listed as a
    /// staged deletion (it is atomic, out of scope for file-level change
    /// detection).
    pub gitlink: bool,
}

/// Every non-tree entry of one HEAD tree, in tree order, with a path map.
pub(crate) struct HeadEntries {
    entries: Vec<HeadEntry>,
    by_path: HashMap<String, usize>,
}

impl HeadEntries {
    /// Walk the tree `root` once, depth-first in tree order, reading each
    /// tree object through `source` exactly once.
    pub(crate) fn read(source: &dyn ObjectSource, root: &gix::oid) -> Result<Self> {
        let mut entries = Vec::new();
        let mut trees = HashMap::new();
        collect(source, &mut trees, root, "", &mut entries)?;
        let by_path = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.path.clone(), index))
            .collect();
        Ok(Self { entries, by_path })
    }

    /// The entry at `path`, if HEAD has a non-tree entry there.
    pub(crate) fn get(&self, path: &str) -> Option<&HeadEntry> {
        self.by_path.get(path).map(|&index| &self.entries[index])
    }

    /// Every entry, in tree order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &HeadEntry> {
        self.entries.iter()
    }
}

/// The trees read so far in one walk, by object id.
type TreeCache = HashMap<gix::ObjectId, Rc<gix::objs::Tree>>;

fn collect(
    source: &dyn ObjectSource,
    trees: &mut TreeCache,
    id: &gix::oid,
    prefix: &str,
    out: &mut Vec<HeadEntry>,
) -> Result<()> {
    let tree = match trees.get(id) {
        Some(tree) => Rc::clone(tree),
        None => {
            let tree = Rc::new(source.read_tree(id)?);
            trees.insert(id.to_owned(), Rc::clone(&tree));
            tree
        }
    };
    for entry in &tree.entries {
        let path = if prefix.is_empty() {
            entry.filename.to_string()
        } else {
            format!("{prefix}/{}", entry.filename)
        };
        match entry.mode.kind() {
            gix::object::tree::EntryKind::Tree => collect(source, trees, &entry.oid, &path, out)?,
            gix::object::tree::EntryKind::Blob
            | gix::object::tree::EntryKind::BlobExecutable
            | gix::object::tree::EntryKind::Link => out.push(HeadEntry {
                path,
                id: entry.oid,
                gitlink: false,
            }),
            gix::object::tree::EntryKind::Commit => out.push(HeadEntry {
                path,
                id: entry.oid,
                gitlink: true,
            }),
        }
    }
    Ok(())
}
