# AST Research: Session Status Module (GIT-026)

## Overview

This document analyzes the existing `codelet/git/src/session_status.rs` module to understand the structures and functions that will be extended for orphan detection and pruning.

## Existing Public Functions

| Function | Location | Purpose |
|----------|----------|---------|
| `get_sessions_dir()` | Line 108 | Returns path to ~/.fspec/git-sessions/ |
| `get_manifest_path(session_id)` | Line 113 | Returns path to manifest JSON file |
| `read_manifest(session_id)` | Line 118 | Read session manifest from disk |
| `write_manifest(manifest)` | Line 135 | Write session manifest to disk |
| `delete_manifest(session_id)` | Line 155 | Delete session manifest from disk |
| `derive_session_status(repo_path, session_id, active_sessions)` | Line 183 | Derive status at query time (Active/PendingMerge/Clean/Orphaned) |
| `complete_session(session_id)` | Line 250 | Mark session as completed |
| `create_session_manifest(...)` | Line 274 | Create new session manifest |
| `terminate_session(session_id)` | Line 297 | Mark session as terminated (orphaned) |
| `list_sessions(repo_path, active_sessions, filter)` | Line 364 | List sessions with filter |
| `inspect_session(repo_path, session_id)` | Line 436 | Get session diff without side effects |
| `merge_session(repo_path, session_id)` | Line 504 | Merge session changes to main |
| `discard_session(repo_path, session_id)` | Line 577 | Discard session without applying changes |

## Existing Enums

### DerivedSessionStatus (Line 25)

```rust
pub enum DerivedSessionStatus {
    Active,      // Session is currently active
    PendingMerge, // Worktree has uncommitted changes
    Clean,       // Worktree has no uncommitted changes
    Orphaned,    // No valid session record
}
```

### SessionFilter (Line 321)

```rust
pub enum SessionFilter {
    All,         // Return all sessions
    Active,      // Only active sessions
    PendingMerge, // Only pending merge sessions
    Clean,       // Only clean sessions
    Orphaned,    // Only orphaned sessions
}
```

## Existing Structs

### SessionManifest (Line 54)

```rust
pub struct SessionManifest {
    pub session_id: String,
    pub project_root: PathBuf,
    pub worktree_path: Option<PathBuf>,
    pub base_commit: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub terminated: bool,  // true = orphaned
}
```

### SessionInfo (Line 338)

```rust
pub struct SessionInfo {
    pub session_id: String,
    pub status: DerivedSessionStatus,
    pub base_commit: String,
    pub files_changed: usize,
    pub created_at: DateTime<Utc>,
    pub worktree_path: PathBuf,
}
```

### MergeResult (Line 452)

```rust
pub struct MergeResult {
    pub session_id: String,
    pub files_modified: Vec<String>,
    pub files_added: Vec<String>,
    pub files_deleted: Vec<String>,
}
```

### DiscardResult (Line 535)

```rust
pub struct DiscardResult {
    pub session_id: String,
    pub files_discarded: usize,
    pub previous_status: DerivedSessionStatus,
}
```

## Implementation Plan for GIT-026

### New Struct: PruneResult

```rust
#[derive(Debug, Clone)]
pub struct PruneResult {
    /// Number of orphaned worktrees that were pruned
    pub count: usize,
    /// List of session IDs that were pruned
    pub pruned: Vec<String>,
}
```

### New Function: is_orphaned

```rust
/// Check if a session is orphaned
///
/// A session is orphaned if:
/// 1. NOT in the active sessions set
/// 2. AND (manifest doesn't exist OR manifest.terminated == true)
pub fn is_orphaned(
    session_id: &str,
    active_sessions: &HashSet<String>,
) -> Result<bool>
```

This is a simplified version of `derive_session_status()` that only checks for orphan state, without needing `repo_path` for diff detection.

### New Function: prune_orphaned

```rust
/// Prune all orphaned worktrees
///
/// Removes worktrees that have no valid session record.
/// Active sessions are never pruned.
pub fn prune_orphaned(
    repo_path: impl AsRef<Path>,
    active_sessions: &HashSet<String>,
) -> Result<PruneResult>
```

## Dependencies

Uses existing functions from same module:
- `read_manifest()` - To check manifest existence and terminated state
- `delete_manifest()` - To clean up manifest files

Uses functions from worktree module:
- `list_worktrees()` - To get all worktree session IDs
- `remove_worktree()` - To remove the worktree directory and git metadata

## Test File Location

Tests will be in: `codelet/git/tests/session_orphan_pruning_tests.rs`

Following the pattern of existing test files:
- `session_status_tests.rs`
- `session_result_tests.rs`
- `session_list_inspect_tests.rs`
- `session_merge_tests.rs`
- `session_discard_tests.rs`
