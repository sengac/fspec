# GIT-013: Gitoxide Migration Research

## Executive Summary

This document captures research findings for migrating fspec from `isomorphic-git` (JavaScript) to `gitoxide/gix` (Rust) via NAPI-RS bindings.

## Current Implementation Analysis

### Files Using isomorphic-git

1. **`src/git/status.ts`** - Core status operations
   - `getStagedFiles()` - Get staged file paths
   - `getUnstagedFiles()` - Get unstaged modified files
   - `getUntrackedFiles()` - Get untracked files
   - `getFileStatus()` - Get status for specific file
   - `getCurrentBranch()` - Get current branch name
   - `getStagedFilesWithChangeType()` - Staged files with A/M/D/R
   - `getUnstagedFilesWithChangeType()` - Unstaged with change types

2. **`src/git/diff.ts`** - Diff operations
   - `getFileDiff()` - Unified diff for file vs HEAD
   - `getCheckpointFileDiff()` - Diff between checkpoint and HEAD
   - Uses `git.resolveRef()`, `git.readBlob()`

3. **`src/utils/git-checkpoint.ts`** - Checkpoint/stash operations
   - `createCheckpoint()` - Create stash-based checkpoint
   - `restoreCheckpoint()` - Restore from checkpoint
   - `listCheckpoints()` - List checkpoints for work unit
   - `deleteCheckpoint()` - Delete checkpoint
   - Uses `git.statusMatrix()`, `git.stash()`, `git.writeRef()`, `git.resolveRef()`, `git.readBlob()`, `git.listFiles()`, `git.walk()`

### isomorphic-git Status Matrix Format

The status matrix is a 2D array with format:
```typescript
type StatusRow = [Filename, HeadStatus, WorkdirStatus, StageStatus]
// HeadStatus: 0 = absent, 1 = present
// WorkdirStatus: 0 = absent, 1 = identical to HEAD, 2 = different
// StageStatus: 0 = absent, 1 = identical to HEAD, 2 = identical to WORKDIR, 3 = different
```

## Gitoxide Architecture

### Core Crates

| Crate | Purpose | Relevance |
|-------|---------|-----------|
| `gix` | High-level repository API | Primary entry point |
| `gix-status` | Index vs worktree comparison | Status operations |
| `gix-diff` | Tree/blob diffing | File diffs |
| `gix-index` | Git index operations | Staging area |
| `gix-object` | Object store access | Read blobs/trees |
| `gix-ref` | Reference management | Branch/HEAD access |
| `gix-worktree` | Worktree state cache | Excludes, attributes |
| `gix-merge` | Three-way merge | Future: merge support |

### Key gitoxide APIs

#### Opening a Repository
```rust
use gix::Repository;

let repo = gix::open(".")?;
// Or with options:
let repo = gix::ThreadSafeRepository::open_opts(path, options)?.into();
```

#### Status Operations
```rust
use gix::status::{Platform, Submodule, UntrackedFiles};

let platform = repo.status(gix::progress::Discard)?;
let iter = platform.into_iter()?;

for item in iter {
    match item? {
        gix::status::Item::IndexWorktree(change) => { /* ... */ }
        gix::status::Item::TreeIndex(change) => { /* ... */ }
    }
}
```

#### Reading Blobs
```rust
let object = repo.find_object(oid)?;
let blob = object.into_blob()?;
let content = blob.data;
```

#### Reference Resolution
```rust
let head = repo.head()?;
let branch_name = head.referent_name(); // Option<&BStr>
let commit = head.peel_to_commit()?;
```

### Status Change Detection

gitoxide uses `gix-status::index_as_worktree()` which compares:
- Index entries against working directory files
- Returns semantic change types similar to `git status`

Change types in gitoxide:
```rust
pub enum Change {
    Removed,
    Modified,
    // etc.
}
```

## NAPI-RS Integration Strategy

### Recommended Approach

1. **Create `codelet-napi/src/git/` module** with Rust implementations
2. **Expose TypeScript-compatible interfaces** matching current API
3. **Use async operations** for non-blocking I/O

### Example NAPI Binding

```rust
// codelet-napi/src/git/status.rs
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
pub struct FileStatusJs {
    pub filepath: String,
    pub staged: bool,
    pub has_unstaged_changes: bool,
    pub untracked: bool,
}

#[napi]
pub async fn get_staged_files(dir: String) -> Result<Vec<String>> {
    let repo = gix::open(&dir)?;
    // ... implementation
}
```

### API Mapping

| isomorphic-git | gitoxide (gix) | Notes |
|---------------|----------------|-------|
| `statusMatrix()` | `repo.status().into_iter()` | Different return format |
| `resolveRef()` | `repo.find_reference()?.peel_to_id()` | |
| `readBlob()` | `repo.find_object(oid)?.into_blob()` | |
| `currentBranch()` | `repo.head()?.referent_name()` | |
| `stash()` | Manual commit creation | No direct stash API |
| `writeRef()` | `repo.refs().transaction()` | |
| `listFiles()` | `repo.index()?.entries()` or tree iteration | |
| `walk()` | `gix_traverse::tree::breadthfirst()` | |

## Migration Challenges

### 1. Stash Operations
gitoxide doesn't have a high-level stash API. Our checkpoint system uses `git.stash({ op: 'create' })` which creates a stash commit without modifying refs.

**Solution**: Implement stash commit creation manually:
1. Create tree from index + worktree changes
2. Create commit with two parents (HEAD, index)
3. Store ref in `refs/fspec-checkpoints/`

### 2. Status Matrix Format
isomorphic-git returns `[filepath, HEAD, WORKDIR, STAGE]` array.
gitoxide returns structured `Change` enums.

**Solution**: Transform gitoxide output to match our existing `FileStatus` interface.

### 3. memfs Testing
Current tests use `memfs` for filesystem mocking.
gitoxide expects real filesystem operations.

**Solution**: Use temp directories for testing, or explore gitoxide's test utilities.

## Performance Expectations

| Operation | isomorphic-git | gitoxide (expected) |
|-----------|---------------|---------------------|
| Status of 1000 files | ~500ms | ~50-100ms |
| Read blob | ~10ms | ~1-2ms |
| Resolve ref | ~5ms | <1ms |

gitoxide is expected to be 5-10x faster due to:
- Native code execution
- Efficient memory management
- Parallel processing support

## Implementation Plan

### Phase 1: Core Status Operations
1. Create `codelet-napi/src/git/status.rs`
2. Implement `get_staged_files()`, `get_unstaged_files()`, `get_untracked_files()`
3. Update `src/git/status.ts` to use NAPI bindings
4. Maintain same TypeScript interface

### Phase 2: Diff Operations
1. Create `codelet-napi/src/git/diff.rs`
2. Implement `get_file_diff()`, `get_checkpoint_file_diff()`
3. Use `gix-diff` for blob comparison

### Phase 3: Checkpoint Operations
1. Implement stash-like commit creation
2. Migrate ref operations
3. Update checkpoint utilities

### Phase 4: TUI Integration
1. Update TUI components using git operations
2. Ensure async/streaming works correctly
3. Test performance improvements

## Testing Strategy

1. **Unit Tests**: Test each NAPI function in isolation
2. **Integration Tests**: Test with real git repositories
3. **Compatibility Tests**: Ensure output matches isomorphic-git format
4. **Performance Tests**: Benchmark against isomorphic-git

## Dependencies to Add

```toml
# codelet-napi/Cargo.toml
[dependencies]
gix = { version = "0.72", features = ["status", "worktree-stream"] }
```

## References

- [gitoxide Repository](https://github.com/GitoxideLabs/gitoxide)
- [gix crate docs](https://docs.rs/gix)
- [NAPI-RS Documentation](https://napi.rs/)
- [isomorphic-git statusMatrix](https://isomorphic-git.org/docs/en/statusMatrix)
