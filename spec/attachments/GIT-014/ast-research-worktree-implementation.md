# AST Research: Worktree Implementation for GIT-014

## Summary

Analysis of existing codebase patterns to inform worktree implementation.

## 1. SessionManifest Structure (needs modification)

**File:** `codelet/napi/src/persistence/types.rs:171-201`

```rust
pub struct SessionManifest {
    pub id: Uuid,
    pub name: String,
    pub project: PathBuf,
    pub provider: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<MessageRef>,
    pub forked_from: Option<ForkPoint>,
    pub merged_from: Vec<MergeRecord>,
    pub compaction: Option<CompactionState>,
    pub token_usage: TokenUsage,
    pub anchor_points: Vec<PersistedAnchorPoint>,
}
```

**Fields to add:**
```rust
/// Path to worktree directory (None if not isolated)
#[serde(default)]
pub worktree_path: Option<PathBuf>,

/// Commit the worktree was created from
#[serde(default)]
pub worktree_base_commit: Option<String>,

/// When the worktree was created
#[serde(default)]
pub worktree_created_at: Option<DateTime<Utc>>,
```

## 2. Data Directory Pattern (to reuse)

**File:** `codelet/common/src/data_dir.rs`

```rust
pub fn set_data_directory(dir: PathBuf) -> Result<(), String>
pub fn get_data_dir() -> Result<PathBuf, String>
```

**Pattern:** Single source of truth via static Mutex<Option<PathBuf>>

**For worktrees:** Project-local `.fspec/worktrees/` is different - use project path, not global data dir.

## 3. Existing Git Crate Structure

**File:** `codelet/git/src/lib.rs`

```rust
mod diff;
mod error;
mod status;

pub(crate) fn open_repo(dir: impl AsRef<Path>) -> Result<gix::Repository>
```

**Pattern:** 
- Modules for logical groupings (diff, status)
- Shared `open_repo()` helper
- Custom error type via `error.rs`

**New module:** `worktree.rs` following same pattern

## 4. Files to Create/Modify

### Create:
- `codelet/git/src/worktree.rs` - Worktree operations
  - `create_worktree(repo, session_id, commit_ref) -> WorktreeInfo`
  - `remove_worktree(repo, session_id) -> Result`
  - `list_worktrees(repo) -> Vec<WorktreeInfo>`

### Modify:
- `codelet/git/src/lib.rs` - Add `mod worktree` and re-exports
- `codelet/napi/src/persistence/types.rs` - Add worktree fields to SessionManifest
- `codelet/napi/src/persistence/napi_bindings.rs` - Add NapiSessionManifest worktree fields

## 5. gitoxide Worktree API

**Available (read-only):**
- `Repository::worktrees()` - List linked worktrees
- `Repository::worktree()` - Get current worktree
- `worktree::Proxy::base()` - Worktree checkout path
- `worktree::Proxy::is_locked()` - Check lock status

**Not available (must implement manually):**
- `worktree add` - Create new worktree
- `worktree remove` - Remove worktree
- `worktree prune` - Cleanup stale worktrees

## 6. Manual Worktree Creation Steps

1. Create `.fspec/worktrees/<session-id>/` directory
2. Create `.git/worktrees/<session-id>/` metadata directory
3. Write HEAD file (detached: `<commit-sha>`)
4. Write gitdir file (path to worktree's .git file)
5. Write commondir file (relative path `../..`)
6. Create worktree's `.git` file pointing to metadata
7. Checkout files using `gix-worktree-state`

## 7. Error Handling Pattern

**File:** `codelet/git/src/error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Failed to open repository at {path}: {source}")]
    OpenRepository { path: String, source: gix::open::Error },
    // ...
}
```

**Add variants:**
```rust
#[error("Worktree already exists for session {session_id}")]
WorktreeExists { session_id: String },

#[error("Worktree not found for session {session_id}")]
WorktreeNotFound { session_id: String },

#[error("Failed to create worktree: {message}")]
WorktreeCreate { message: String },
```

## 8. AST Search Results

### Functions in git crate:
- `is_binary_content(buffer: &[u8]) -> bool`
- `get_head_file_content(repo: &Repository, filepath: &str) -> Result<String>`
- `generate_unified_diff(_filepath: &str, old_content: &str, new_content: &str) -> String`
- `path_to_string(path: &BStr) -> String`
- `is_git_dir(entry: &DirEntry) -> bool`

### Public structs in persistence:
- `SessionManifest` - Main target for modification
- `SessionStore` - Session CRUD operations
- `MessageStore` - Message storage
- `NapiSessionManifest` - NAPI bindings

### Data directory functions:
- `set_data_directory(dir: PathBuf)` - Init at startup
- `get_data_dir()` - Get base path

## 9. Test Patterns

Existing tests use temp directories:
```rust
#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    
    fn setup_test_repo() -> TempDir {
        // Create temp dir with git init
    }
}
```

Follow same pattern for worktree tests.
