# GIT-026: Orphan Detection and Pruning

## Overview

This story implements orphan detection and bulk pruning of orphaned worktrees. Orphaned worktrees are those that exist but have no corresponding session record (manifest missing or terminated).

## Problem Statement

Worktrees can become orphaned when:
- The process crashes during session creation/completion
- Session manifest is manually deleted
- Session terminated abnormally

These orphaned worktrees consume disk space and clutter the list. Users need a way to identify and clean them up.

## Solution

1. Enhance status derivation to detect orphaned state
2. Implement `prune_orphaned()` that removes all orphaned worktrees
3. Return list of pruned session IDs

## Scenarios Covered

| Scenario | Description |
|----------|-------------|
| Prune all orphaned worktrees | All orphaned worktrees removed |
| Prune returns zero when no orphaned worktrees | Count is 0, empty list |
| Prune returns list of pruned session IDs | Returns what was pruned |
| Detect orphaned worktree when session manifest is missing | No manifest = orphaned |
| Detect orphaned worktree when session manifest is terminated | terminated=true = orphaned |
| Active session with manifest is not orphaned | Active sessions never orphaned |

## Implementation Location

### Add to Session Manager Module

```
codelet/git/src/session_manager.rs
├── is_orphaned(repo_path, session_id, active_sessions) -> bool
├── prune_orphaned(repo_path, active_sessions) -> Result<PruneResult>
└── PruneResult struct
```

### Session Manifest Schema

```
~/.fspec/sessions/<session-id>.json
{
    "session_id": "abc-123",
    "project_root": "/path/to/project",
    "worktree_path": "/path/to/project/.fspec/worktrees/abc-123",
    "base_commit": "abc1234...",
    "created_at": "2024-01-15T10:30:00Z",
    "completed_at": "2024-01-15T11:00:00Z",  // null if still running
    "terminated": false  // true if abnormally terminated
}
```

## API Design

### PruneResult Struct

```rust
#[derive(Debug, Clone)]
pub struct PruneResult {
    /// Number of orphaned worktrees that were pruned
    pub count: usize,
    /// List of session IDs that were pruned
    pub pruned: Vec<String>,
}
```

### Orphan Detection Logic

```rust
/// Check if a session is orphaned
/// 
/// A session is orphaned if:
/// 1. NOT in the active sessions map
/// 2. AND (manifest doesn't exist OR manifest.terminated == true)
fn is_orphaned(
    session_id: &str,
    active_sessions: &HashSet<String>,
) -> Result<bool> {
    // Active sessions are never orphaned
    if active_sessions.contains(session_id) {
        return Ok(false);
    }
    
    // Check manifest
    let manifest_path = get_manifest_path(session_id);
    
    if !manifest_path.exists() {
        return Ok(true); // No manifest = orphaned
    }
    
    let manifest: SessionManifest = read_manifest(&manifest_path)?;
    
    if manifest.terminated {
        return Ok(true); // Terminated = orphaned
    }
    
    // Has valid manifest, not terminated = not orphaned
    Ok(false)
}
```

### Prune Function

```rust
/// Prune all orphaned worktrees
/// 
/// This removes worktrees that have no valid session record.
/// Active sessions are never pruned.
/// 
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `active_sessions` - Set of currently active session IDs
/// 
/// # Returns
/// PruneResult with count and list of pruned session IDs
pub fn prune_orphaned(
    repo_path: &Path,
    active_sessions: &HashSet<String>,
) -> Result<PruneResult> {
    let worktrees = list_worktrees(repo_path)?;
    let mut pruned = Vec::new();
    
    for worktree in worktrees {
        if is_orphaned(&worktree.session_id, active_sessions)? {
            // Remove worktree
            remove_worktree(repo_path, &worktree.session_id)?;
            
            // Remove manifest if it exists
            let _ = remove_session_manifest(&worktree.session_id);
            
            pruned.push(worktree.session_id);
        }
    }
    
    Ok(PruneResult {
        count: pruned.len(),
        pruned,
    })
}
```

### Usage Example

```rust
// List orphaned sessions before pruning
let sessions = list_sessions(repo_path, &active, SessionFilter::Orphaned)?;
println!("Found {} orphaned sessions", sessions.len());

for session in &sessions {
    println!("  - {} (base: {})", session.session_id, session.base_commit);
}

// Prune all orphaned
if user_confirms_prune() {
    let result = prune_orphaned(repo_path, &active)?;
    println!("Pruned {} orphaned worktrees:", result.count);
    for id in &result.pruned {
        println!("  - {}", id);
    }
}
```

## Test Strategy

Tests in `codelet/git/tests/session_manager_orphan_test.rs`:

1. **Prune all orphaned**: Multiple orphaned worktrees removed
2. **Prune returns zero**: No orphans = count 0, empty list
3. **Returns pruned list**: Verify correct IDs returned
4. **Missing manifest**: Worktree without manifest is orphaned
5. **Terminated manifest**: terminated=true is orphaned
6. **Active not orphaned**: Active sessions not pruned

## Dependencies

- **GIT-023** (required): List sessions with orphaned filter

## Downstream Dependencies

- **GIT-027**: NAPI bindings expose prune operation

## Acceptance Criteria Checklist

- [ ] `is_orphaned()` detects orphaned state correctly
- [ ] Missing manifest = orphaned
- [ ] terminated=true = orphaned
- [ ] Active sessions never orphaned
- [ ] `prune_orphaned()` removes all orphaned worktrees
- [ ] Returns count of pruned worktrees
- [ ] Returns list of pruned session IDs
- [ ] Empty result when no orphans exist
- [ ] Session manifests cleaned up during prune
- [ ] All tests pass

---

## Next Steps

GIT-026 is a **parallel story** with GIT-024 and GIT-025. Once complete:

| Action | Description |
|--------|-------------|
| **Mark Done** | Move GIT-026 to `done` status |
| **Verify Integration** | Orphan detection and pruning work correctly |
| **Check GIT-027** | If GIT-024 and GIT-025 are also done, GIT-027 can start |

## Story Dependency Graph

```
GIT-023 (List/Inspect)
    │
    ├── GIT-024 (Merge)      │
    │           │            │
    │           └────────────┤
    │                        │
    ├── GIT-025 (Discard)    │
    │           │            │
    │           └────────────┤
    │                        │
    └── GIT-026 (This Story) ◀── ORPHAN DETECTION/PRUNING
                │            │
                └────────────┤
                             ▼
                     GIT-027 (NAPI) ◀── WAITS FOR ALL 3
```

## Related Stories

| Story | Relationship | Notes |
|-------|--------------|-------|
| GIT-023 | **Depends On** | List sessions with orphaned filter |
| GIT-024 | Parallel | Both depend on GIT-023, can work concurrently |
| GIT-025 | Parallel | Both depend on GIT-023, can work concurrently |
| GIT-027 | **Required By** | NAPI bindings expose prune operation |
