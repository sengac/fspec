//! Git repository operations (init, add, commit, config, resolve_ref)
//!
//! These operations provide a pure-Rust alternative to isomorphic-git
//! for repository management. Used by production code (resolveRef)
//! and test infrastructure (init, add, commit, setConfig).

use crate::error::{GitError, Result};
use crate::open_repo;
use gix::bstr::BString;
use gix::date::Time;
use smallvec::SmallVec;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Resolve a git ref to its target commit SHA
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `ref_name` - Full ref path (e.g. "refs/fspec-checkpoints/GIT-039/baseline")
///
/// # Returns
/// Hex string of the resolved commit SHA
pub fn resolve_ref(dir: impl AsRef<Path>, ref_name: &str) -> Result<String> {
    let repo = open_repo(dir)?;
    let mut reference = repo
        .find_reference(ref_name)
        .map_err(|e| GitError::Other(format!("Ref not found '{}': {}", ref_name, e)))?;

    let id = reference
        .peel_to_id_in_place()
        .map_err(|e| GitError::Other(format!("Failed to peel ref '{}': {}", ref_name, e)))?;

    Ok(id.to_string())
}

/// Initialize a new git repository
///
/// # Arguments
/// * `dir` - Path to create the repository at
/// * `default_branch` - Name of the default branch (e.g. "main")
pub fn git_init(dir: impl AsRef<Path>, default_branch: &str) -> Result<()> {
    let path = dir.as_ref();

    // Ensure directory exists
    fs::create_dir_all(path)?;

    // Use non-bare init so gix recognizes it as a worktree
    gix::init(path)
        .map_err(|e| GitError::Other(format!("Failed to init repository: {}", e)))?;

    // Set the default branch by writing HEAD
    let head_path = path.join(".git/HEAD");
    fs::write(
        &head_path,
        format!("ref: refs/heads/{}\n", default_branch),
    )?;

    Ok(())
}

/// Set a git config value
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `key` - Config key (e.g. "user.name")
/// * `value` - Config value
pub fn git_set_config(dir: impl AsRef<Path>, key: &str, value: &str) -> Result<()> {
    let path = dir.as_ref();
    let config_path = path.join(".git/config");

    // Read existing config or create empty one
    let existing = fs::read_to_string(&config_path).unwrap_or_default();

    // Parse key into section and name (e.g. "user.name" -> section "user", key "name")
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    if parts.len() != 2 {
        return Err(GitError::Other(format!("Invalid config key: {}", key)));
    }
    let section = parts[0];
    let name = parts[1];

    // Check if section already exists
    let section_header = format!("[{}]", section);
    let entry = format!("\t{} = {}\n", name, value);

    let new_config = if existing.contains(&section_header) {
        // Check if the key already exists in this section
        let mut result = String::new();
        let mut in_target_section = false;
        let mut key_replaced = false;

        for line in existing.lines() {
            if line.trim() == section_header {
                in_target_section = true;
                result.push_str(line);
                result.push('\n');
            } else if line.starts_with('[') {
                if in_target_section && !key_replaced {
                    result.push_str(&entry);
                    key_replaced = true;
                }
                in_target_section = false;
                result.push_str(line);
                result.push('\n');
            } else if in_target_section
                && (line.trim().starts_with(&format!("{} ", name))
                    || line.trim().starts_with(&format!("{}=", name)))
            {
                // Replace existing key
                result.push_str(&entry);
                key_replaced = true;
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        if in_target_section && !key_replaced {
            result.push_str(&entry);
        }

        result
    } else {
        // Add new section
        format!("{}{}\n{}", existing, section_header, entry)
    };

    fs::write(&config_path, new_config)?;

    Ok(())
}

/// Stage a file (equivalent to `git add <filepath>`)
///
/// If filepath is "." it stages all files in the repository (equivalent to `git add .`).
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `filepath` - Path to the file relative to repository root, or "." for all files
pub fn git_add(dir: impl AsRef<Path>, filepath: &str) -> Result<()> {
    let path = dir.as_ref();

    // Handle "." - add all files
    if filepath == "." {
        return git_add_all(path);
    }

    let repo = open_repo(path)?;

    let full_path = path.join(filepath);
    if !full_path.exists() {
        return Err(GitError::FileNotFound(filepath.to_string()));
    }

    // Read file content and write as blob
    let content = fs::read(&full_path)?;
    let blob_id = repo
        .write_blob(&content)
        .map_err(|e| GitError::Other(format!("Failed to write blob: {}", e)))?;

    // Read the index file directly so we can mutate it
    let index_path = path.join(".git/index");
    let mut index = if index_path.exists() {
        gix::index::File::at(
            &index_path,
            repo.object_hash(),
            false,
            Default::default(),
        )
        .map_err(|e| GitError::Other(format!("Failed to read index: {}", e)))?
    } else {
        gix::index::File::from_state(
            gix::index::State::new(repo.object_hash()),
            index_path.clone(),
        )
    };

    // Get file metadata using gix's Metadata type
    let gix_metadata = gix::index::fs::Metadata::from_path_no_follow(&full_path)
        .map_err(|e| GitError::Other(format!("Failed to get metadata: {}", e)))?;
    let stat = gix::index::entry::Stat::from_fs(&gix_metadata)
        .map_err(|e| GitError::Other(format!("Failed to stat file: {}", e)))?;

    // Remove existing entry if present
    let bstr_path = gix::bstr::BStr::new(filepath.as_bytes());
    if let Ok(pos) = index.entry_index_by_path(bstr_path) {
        index.remove_entries(|idx, _, _| idx == pos);
    }

    // Push new entry
    index.dangerously_push_entry(
        stat,
        blob_id.into(),
        gix::index::entry::Flags::empty(),
        gix::index::entry::Mode::FILE,
        bstr_path,
    );

    // Sort entries to maintain invariant
    index.sort_entries();

    // Write index back
    let options = gix::index::write::Options::default();
    let mut index_file = fs::File::create(path.join(".git/index"))?;
    index
        .write_to(&mut index_file, options)
        .map_err(|e| GitError::Other(format!("Failed to write index: {}", e)))?;

    Ok(())
}

/// Stage all files in the repository (equivalent to `git add .`)
fn git_add_all(dir: &Path) -> Result<()> {
    // Collect all files in the working tree (excluding .git)
    let mut files_to_add: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| e.file_name() != ".git")
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Ok(rel) = entry.path().strip_prefix(dir) {
                files_to_add.push(rel.to_string_lossy().to_string());
            }
        }
    }

    // Add each file individually
    for filepath in &files_to_add {
        git_add(dir, filepath)?;
    }

    Ok(())
}

/// Build a tree from the current index entries
///
/// Uses the same approach as ghost_commit.rs - builds nested tree objects
/// from flat index entries.
fn build_tree_from_index(repo: &gix::Repository, index: &gix::index::File) -> Result<gix::ObjectId> {
    // Collect all entries from index
    let mut file_blobs: BTreeMap<String, (gix::ObjectId, bool)> = BTreeMap::new();

    for entry in index.entries() {
        let path = entry.path(index);
        let path_str = String::from_utf8_lossy(path.as_ref()).to_string();
        let is_executable = entry.mode == gix::index::entry::Mode::FILE_EXECUTABLE;
        file_blobs.insert(path_str, (entry.id, is_executable));
    }

    if file_blobs.is_empty() {
        let empty_tree = gix::objs::Tree { entries: vec![] };
        let tree_id = repo
            .write_object(&empty_tree)
            .map_err(|e| GitError::Other(format!("Failed to write empty tree: {}", e)))?;
        return Ok(tree_id.into());
    }

    // Group files by directory
    let mut dir_entries: BTreeMap<String, Vec<(String, gix::ObjectId, bool)>> = BTreeMap::new();
    dir_entries.insert(String::new(), Vec::new());

    for (path, (blob_id, is_exec)) in &file_blobs {
        let path_obj = std::path::Path::new(path);
        let parent = path_obj
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let filename = path_obj
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        // Ensure all parent directories exist
        let mut current_path = parent.clone();
        while !current_path.is_empty() {
            if !dir_entries.contains_key(&current_path) {
                dir_entries.insert(current_path.clone(), Vec::new());
            }
            current_path = std::path::Path::new(&current_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
        }

        dir_entries
            .entry(parent)
            .or_default()
            .push((filename, *blob_id, *is_exec));
    }

    // Build trees bottom-up
    let mut tree_ids: BTreeMap<String, gix::ObjectId> = BTreeMap::new();

    // Sort directories by depth (deepest first)
    let mut dirs: Vec<String> = dir_entries.keys().cloned().collect();
    dirs.sort_by(|a, b| {
        let depth_a = if a.is_empty() { 0 } else { a.matches('/').count() + 1 };
        let depth_b = if b.is_empty() { 0 } else { b.matches('/').count() + 1 };
        depth_b.cmp(&depth_a)
    });

    for dir in &dirs {
        let mut entries: Vec<gix::objs::tree::Entry> = Vec::new();

        // Add file entries
        if let Some(files) = dir_entries.get(dir) {
            for (name, blob_id, is_exec) in files {
                let mode = if *is_exec {
                    gix::objs::tree::EntryKind::BlobExecutable
                } else {
                    gix::objs::tree::EntryKind::Blob
                };
                entries.push(gix::objs::tree::Entry {
                    mode: mode.into(),
                    filename: BString::from(name.as_str()),
                    oid: *blob_id,
                });
            }
        }

        // Add subdirectory tree entries
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{}/", dir)
        };

        for (subdir, tree_id) in &tree_ids {
            // Check if subdir is a direct child of dir
            if let Some(rest) = subdir.strip_prefix(&prefix) {
                if !rest.contains('/') && !rest.is_empty() {
                    entries.push(gix::objs::tree::Entry {
                        mode: gix::objs::tree::EntryKind::Tree.into(),
                        filename: BString::from(rest),
                        oid: *tree_id,
                    });
                }
            } else if dir.is_empty() && !subdir.contains('/') && !subdir.is_empty() {
                entries.push(gix::objs::tree::Entry {
                    mode: gix::objs::tree::EntryKind::Tree.into(),
                    filename: BString::from(subdir.as_str()),
                    oid: *tree_id,
                });
            }
        }

        // Sort entries by name (git requirement)
        entries.sort_by(|a, b| a.filename.cmp(&b.filename));

        let tree = gix::objs::Tree { entries };
        let tree_id = repo
            .write_object(&tree)
            .map_err(|e| GitError::Other(format!("Failed to write tree: {}", e)))?;
        tree_ids.insert(dir.clone(), tree_id.into());
    }

    tree_ids
        .get("")
        .copied()
        .ok_or_else(|| GitError::Other("Failed to build root tree".to_string()))
}

/// Create a commit from the current index
///
/// # Arguments
/// * `dir` - Path to the repository root
/// * `message` - Commit message
/// * `author_name` - Author name
/// * `author_email` - Author email
///
/// # Returns
/// Hex string of the new commit SHA
pub fn git_commit(
    dir: impl AsRef<Path>,
    message: &str,
    author_name: &str,
    author_email: &str,
) -> Result<String> {
    let path = dir.as_ref();
    let repo = open_repo(path)?;

    // Read current index
    let index_path = path.join(".git/index");
    let index = if index_path.exists() {
        gix::index::File::at(
            &index_path,
            repo.object_hash(),
            false,
            Default::default(),
        )
        .map_err(|e| GitError::Other(format!("Failed to read index: {}", e)))?
    } else {
        gix::index::File::from_state(
            gix::index::State::new(repo.object_hash()),
            index_path.clone(),
        )
    };

    // Build tree from index
    let tree_id = build_tree_from_index(&repo, &index)?;

    // Get parent commit (if any)
    let parent = repo.head_commit().ok().map(|c| c.id);

    // Create signature
    let now = chrono::Utc::now();
    let timestamp = now.timestamp();
    let time = Time::new(timestamp, 0i32);
    let signature = gix::actor::Signature {
        name: BString::from(author_name),
        email: BString::from(author_email),
        time,
    };

    // Build parents
    let parents: SmallVec<[gix::ObjectId; 1]> = if let Some(parent_id) = parent {
        SmallVec::from_buf([parent_id.into()])
    } else {
        SmallVec::new()
    };

    let commit = gix::objs::Commit {
        tree: tree_id,
        parents,
        author: signature.clone(),
        committer: signature,
        encoding: None,
        message: BString::from(message),
        extra_headers: vec![],
    };

    // Write commit object
    let commit_id = repo
        .write_object(gix::objs::Object::Commit(commit))
        .map_err(|e| GitError::Other(format!("Failed to write commit: {}", e)))?;

    // Update HEAD ref
    let head_ref = repo
        .head_ref()
        .map_err(|e| GitError::Head(format!("Failed to read HEAD: {}", e)))?;

    if let Some(mut head_ref) = head_ref {
        // Update existing ref
        head_ref
            .set_target_id(commit_id, "commit")
            .map_err(|e| GitError::Other(format!("Failed to update HEAD: {}", e)))?;
    } else {
        // Create the branch ref (initial commit on unborn branch)
        let head_content = fs::read_to_string(path.join(".git/HEAD"))?;
        if let Some(ref_path) = head_content.strip_prefix("ref: ") {
            let ref_path = ref_path.trim();
            let full_ref_path = path.join(".git").join(ref_path);
            if let Some(parent_dir) = full_ref_path.parent() {
                fs::create_dir_all(parent_dir)?;
            }
            fs::write(full_ref_path, format!("{}\n", commit_id))?;
        }
    }

    Ok(commit_id.to_string())
}
