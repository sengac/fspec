# AST Research: Ghost Commits for Checkpoint Snapshots

## Work Unit: GIT-017

## Overview

This research analyzes the existing checkpoint implementation and Rust infrastructure to guide the implementation of ghost commit-based checkpoints.

---

## 1. Current TypeScript Implementation Analysis

### File: `src/utils/git-checkpoint.ts`

#### Key Functions

| Function | Purpose | Lines |
|----------|---------|-------|
| `createCheckpoint()` | Creates stash-based checkpoint | 184-288 |
| `restoreCheckpoint()` | Restores files from checkpoint | 293-429 |
| `listCheckpoints()` | Lists all checkpoints for work unit | 495-541 |
| `deleteCheckpoint()` | Deletes single checkpoint | 663-714 |
| `deleteAllCheckpoints()` | Deletes all checkpoints for work unit | 719-759 |
| `cleanupAutoCheckpoints()` | Cleans automatic checkpoints on done | 581-648 |
| `getCheckpointChangedFiles()` | Gets files changed in checkpoint | 767-830 |
| `restoreCheckpointFile()` | Restores single file from checkpoint | 897-995 |

#### Current Implementation Pattern

```typescript
// Current: Uses isomorphic-git stash
const stashOid = await git.stash({
  fs,
  dir: cwd,
  op: 'create',
  message,
});

// Stores ref in custom namespace
const checkpointRef = `refs/fspec-checkpoints/${workUnitId}/${checkpointName}`;
await git.writeRef({
  fs,
  dir: cwd,
  ref: checkpointRef,
  value: stashOid,
});
```

#### Key Data Structures

```typescript
export interface Checkpoint {
  name: string;
  workUnitId: string;
  timestamp: string;
  stashRef: string;  // Will become ghostCommitSha
  isAutomatic: boolean;
  message: string;
}

export interface CheckpointOptions {
  workUnitId: string;
  checkpointName: string;
  cwd: string;
  includeUntracked?: boolean;
}

export interface RestoreOptions {
  workUnitId: string;
  checkpointName: string;
  cwd: string;
  force?: boolean;
}
```

---

## 2. Rust Infrastructure Analysis

### File: `codelet/git/src/lib.rs`

The `codelet-git` crate already provides:

1. **Repository opening**: `open_repo()` helper
2. **Status operations**: `get_staged_files()`, `get_unstaged_files()`, `get_untracked_files()`
3. **Diff operations**: `get_file_diff()`
4. **Tree utilities**: `get_tree_files()`, `collect_worktree_files()`
5. **Error handling**: Custom `GitError` enum

### File: `codelet/git/src/status.rs`

Shows pattern for:
- Opening repository with `gix::open()`
- Reading index with `repo.index()`
- Comparing index entries with HEAD tree
- Walking working directory for untracked files
- Using `gix::objs::compute_hash()` for content hashing

### File: `codelet/git/src/tree_utils.rs`

Shows pattern for:
- Collecting files from worktree directory
- Reading tree entries from git commits
- Recursively walking tree structures

### File: `codelet/git/Cargo.toml`

Dependencies available:
```toml
gix = { version = "0.72", features = [
    "status",
    "revision",
    "blob-diff",
    "attributes",
    "excludes",
    "index",
] }
```

**Note**: May need additional gix features for commit creation.

---

## 3. NAPI Binding Pattern

### File: `codelet/napi/src/git.rs`

Pattern for exposing Rust functions to TypeScript:

```rust
#[napi]
pub fn get_staged_files(dir: String) -> napi::Result<Vec<String>> {
    codelet_git::get_staged_files(&dir)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi(object)]
pub struct WorktreeCreateResultJs {
    pub session_id: String,
    pub path: String,
    // ...
}
```

---

## 4. Ghost Commit Implementation Plan

### New Rust Module: `codelet/git/src/ghost_commit.rs`

```rust
//! Ghost commit operations for fspec checkpoints
//!
//! Ghost commits are detached commits that:
//! - Capture complete working tree state
//! - Have no branch reference (invisible to git log)
//! - Preserve parent relationship to HEAD

use crate::error::{GitError, Result};
use crate::open_repo;
use std::path::Path;

pub struct GhostCheckpoint {
    pub sha: String,
    pub parent_sha: String,
    pub files: Vec<String>,
}

/// Create a ghost commit capturing current working tree state
///
/// Uses temporary index to avoid disturbing user's staging area.
pub fn create_ghost_commit(
    dir: &Path,
    work_unit_id: &str,
    checkpoint_name: &str,
) -> Result<GhostCheckpoint> {
    // Implementation:
    // 1. Open repository
    // 2. Create temporary index file
    // 3. Read HEAD tree into temp index
    // 4. Add all working tree changes to temp index
    // 5. Write tree from temp index
    // 6. Create commit with tree and HEAD as parent
    // 7. Store ref at refs/fspec-checkpoints/{work_unit_id}/{checkpoint_name}
    todo!()
}

/// Restore working tree from ghost commit
pub fn restore_ghost_commit(
    dir: &Path,
    work_unit_id: &str,
    checkpoint_name: &str,
) -> Result<RestoreResult> {
    // Implementation:
    // 1. Resolve ref to get ghost commit SHA
    // 2. Read tree from ghost commit
    // 3. For each file in tree, write to working directory
    // 4. Delete files that don't exist in checkpoint tree
    todo!()
}
```

### New NAPI Bindings

```rust
// codelet/napi/src/git.rs additions

#[napi(object)]
pub struct GhostCheckpointJs {
    pub sha: String,
    pub parent_sha: String,
    pub files: Vec<String>,
}

#[napi]
pub fn create_ghost_checkpoint(
    dir: String,
    work_unit_id: String,
    checkpoint_name: String,
) -> napi::Result<GhostCheckpointJs> {
    codelet_git::ghost_commit::create_ghost_commit(
        Path::new(&dir),
        &work_unit_id,
        &checkpoint_name,
    )
    .map(|r| GhostCheckpointJs {
        sha: r.sha,
        parent_sha: r.parent_sha,
        files: r.files,
    })
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn restore_ghost_checkpoint(
    dir: String,
    work_unit_id: String,
    checkpoint_name: String,
    force: Option<bool>,
) -> napi::Result<RestoreResultJs> {
    // ...
}
```

### TypeScript Migration

Update `src/utils/git-checkpoint.ts` to use NAPI bindings:

```typescript
// Replace isomorphic-git imports with:
import {
  createGhostCheckpoint,
  restoreGhostCheckpoint,
  // ...
} from '../codelet';

// Update createCheckpoint to call Rust:
export async function createCheckpoint(options: CheckpointOptions) {
  const result = createGhostCheckpoint(
    options.cwd,
    options.workUnitId,
    options.checkpointName,
  );
  
  return {
    success: true,
    checkpointName: options.checkpointName,
    stashMessage: '', // Legacy compat
    stashRef: `refs/fspec-checkpoints/${options.workUnitId}/${options.checkpointName}`,
    includedUntracked: true, // Ghost commits always include all
    capturedFiles: result.files,
  };
}
```

---

## 5. Key gix APIs Required

Based on research, these gix APIs will be needed:

1. **Index operations**:
   - `gix::index::File::from_state()` - Create index from scratch
   - `index.write()` - Write index to temporary file

2. **Object creation**:
   - `repo.write_object()` - Write blob/tree/commit objects
   - `gix::objs::Tree::new()` - Create tree object
   - `gix::objs::Commit::new()` - Create commit object

3. **Reference operations**:
   - `repo.refs.transaction()` - Create/update refs
   - `repo.find_reference()` - Read existing refs

4. **Tree reading**:
   - `repo.find_object()` - Get tree/blob by OID
   - `tree.traverse()` - Walk tree entries

---

## 6. Testing Strategy

### Rust Unit Tests: `codelet/git/tests/ghost_commit_tests.rs`

```rust
#[test]
fn test_create_ghost_commit_captures_all_states() {
    // Setup: Create temp repo with staged, unstaged, untracked files
    // Action: create_ghost_commit()
    // Assert: All files captured, staging area preserved
}

#[test]
fn test_restore_ghost_commit_replaces_working_tree() {
    // Setup: Create checkpoint, modify files
    // Action: restore_ghost_commit()
    // Assert: Files match checkpoint state
}

#[test]
fn test_ghost_commit_preserves_parent_relationship() {
    // Setup: Create checkpoint
    // Action: Read ghost commit
    // Assert: Parent is HEAD at creation time
}
```

### TypeScript Integration Tests

Existing test patterns in `src/commands/__tests__/` show:
- Use `tempfile` for isolated test directories
- Mock file system for unit tests
- Integration tests with real git repos

---

## 7. Migration Checklist

- [ ] Add `ghost_commit` module to `codelet-git`
- [ ] Implement `create_ghost_commit()` in Rust
- [ ] Implement `restore_ghost_commit()` in Rust
- [ ] Add NAPI bindings for new functions
- [ ] Update TypeScript `createCheckpoint()` to call Rust
- [ ] Update TypeScript `restoreCheckpoint()` to call Rust
- [ ] Update TypeScript `listCheckpoints()` (may work as-is with refs)
- [ ] Update TypeScript `getCheckpointChangedFiles()` to call Rust
- [ ] Write Rust unit tests
- [ ] Update TypeScript integration tests
- [ ] Verify TUI CheckpointViewer still works

---

## 8. API Surface Compatibility

The TypeScript API surface MUST remain unchanged:

| Function | Signature Change |
|----------|------------------|
| `createCheckpoint()` | None (returns same structure) |
| `restoreCheckpoint()` | None (returns same structure) |
| `listCheckpoints()` | None (reads same refs namespace) |
| `deleteCheckpoint()` | None (deletes same refs) |
| `getCheckpointChangedFiles()` | None (works with any commit OID) |
| `restoreCheckpointFile()` | None (reads blob from commit) |

---

## Conclusion

The implementation path is clear:
1. Create `ghost_commit.rs` module in codelet-git
2. Implement create/restore using gix APIs
3. Add NAPI bindings
4. Update TypeScript to call Rust instead of isomorphic-git
5. Maintain API compatibility for all existing consumers
