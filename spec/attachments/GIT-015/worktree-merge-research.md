# GIT-015: Worktree Merge Management Research

## Executive Summary

This document captures research for enabling merge operations from agent worktrees back to the main working directory, including conflict detection, resolution strategies, and safe merge workflows.

## Merge Scenarios

### Typical Agent Workflow

```
1. Parent agent working on main branch
2. Child agent spawned → creates worktree with new branch
3. Child agent completes work → commits to worktree branch
4. Child agent ready to merge → needs to merge back to parent
5. Parent reviews/merges → changes integrated
```

### Merge Types Required

| Type | Description | Use Case |
|------|-------------|----------|
| Fast-forward | No divergence, just move ref | Child based on latest HEAD |
| Recursive | Three-way merge | Concurrent changes in parent |
| Ours/Theirs | Conflict resolution strategy | Automated resolution |
| Squash | Combine commits | Clean history |
| Cherry-pick | Select specific commits | Partial integration |

## gitoxide Merge Capabilities

### gix-merge Crate Analysis

From `/tmp/gitoxide/gix-merge/src/lib.rs`:

```rust
//! Provide facilities to merge *blobs*, *trees* and *commits*.
//!
//! * [blob-merges](blob) look at file content.
//! * [tree-merges](tree) look at trees and merge them structurally.
//! * [commit-merges](commit) are like tree merges, but compute merge-base.

pub mod blob;
pub mod commit;
pub mod tree;
```

### Blob Merge (Content Merge)

```rust
// gix-merge/src/blob/mod.rs
pub enum Resolution {
    /// Everything resolved, no conflict
    Complete,
    /// Conflicts auto-resolved (e.g., by strategy)
    CompleteWithAutoResolvedConflict,
    /// Conflict markers present
    Conflict,
}

pub enum BuiltinDriver {
    /// Text merge with conflict markers
    Text,
    /// Choose ours or theirs
    Binary,
    /// Add all lines without markers
    Union,
}
```

### Tree Merge

```rust
// gix-merge/src/tree/mod.rs
pub struct Outcome<'a> {
    /// The merged tree (unwritten)
    pub tree: gix_object::tree::Editor<'a>,
    /// Conflicts encountered
    pub conflicts: Vec<Conflict>,
    /// If stopped on first conflict
    pub failed_on_first_unresolved_conflict: bool,
}

pub struct Conflict {
    /// Resolution or failure info
    pub resolution: Result<Resolution, ResolutionFailure>,
    /// Our change
    pub ours: Change,
    /// Their change  
    pub theirs: Change,
    /// Index entries for stages 1/2/3
    pub entries: [Option<ConflictIndexEntry>; 3],
}
```

### Resolution Strategies

```rust
pub enum ResolveWith {
    /// Keep ancestor state on conflict
    Ancestor,
    /// Prefer our side on conflict
    Ours,
}

pub struct Options {
    /// Enable rename tracking
    pub rewrites: Option<Rewrites>,
    /// Blob merge options
    pub blob_merge: blob::platform::merge::Options,
    /// Stop on first conflict
    pub fail_on_conflict: Option<TreatAsUnresolved>,
    /// Symlink conflict resolution
    pub symlink_conflicts: Option<binary::ResolveWith>,
    /// Tree conflict resolution
    pub tree_conflicts: Option<ResolveWith>,
}
```

## Merge Implementation Strategy

### Core Merge Operations

```rust
use gix::Repository;
use gix_merge::tree::{self, Options, Outcome};

pub async fn merge_worktree_into_main(
    main_repo: &Repository,
    worktree_branch: &str,
    options: MergeOptions,
) -> Result<MergeResult, Error> {
    // 1. Get the commits to merge
    let main_commit = main_repo.head()?.peel_to_commit()?;
    let worktree_commit = main_repo
        .find_reference(&format!("refs/heads/{}", worktree_branch))?
        .peel_to_commit()?;
    
    // 2. Find merge base
    let merge_bases = main_repo.merge_bases(main_commit.id, worktree_commit.id)?;
    let base_commit = merge_bases.first()
        .ok_or(Error::NoMergeBase)?;
    
    // 3. Check for fast-forward possibility
    if is_ancestor(&main_repo, main_commit.id, worktree_commit.id)? {
        // Fast-forward possible
        return fast_forward_merge(main_repo, worktree_commit);
    }
    
    // 4. Perform three-way merge
    let outcome = gix_merge::tree::function::tree(
        base_commit.tree()?,
        main_commit.tree()?,
        worktree_commit.tree()?,
        tree::Options {
            rewrites: Some(Default::default()),
            fail_on_conflict: Some(tree::TreatAsUnresolved::git()),
            ..Default::default()
        },
        &main_repo.objects,
    )?;
    
    // 5. Handle conflicts
    if outcome.has_unresolved_conflicts(tree::TreatAsUnresolved::git()) {
        return Ok(MergeResult::Conflicts(extract_conflicts(&outcome)));
    }
    
    // 6. Write merged tree
    let merged_tree_id = outcome.tree.write(&main_repo.objects)?;
    
    // 7. Create merge commit (if not squashing)
    let merge_commit = create_merge_commit(
        main_repo,
        merged_tree_id,
        &[main_commit.id, worktree_commit.id],
        &format!("Merge {} into main", worktree_branch),
    )?;
    
    Ok(MergeResult::Success(merge_commit))
}
```

### Conflict Detection API

```typescript
interface MergeConflict {
  filepath: string;
  type: 'content' | 'rename' | 'delete' | 'type-mismatch';
  base?: BlobInfo;
  ours: BlobInfo;
  theirs: BlobInfo;
  resolution?: 'ours' | 'theirs' | 'manual';
}

interface MergePreview {
  canFastForward: boolean;
  conflictsDetected: boolean;
  conflicts: MergeConflict[];
  changedFiles: string[];
  addedFiles: string[];
  deletedFiles: string[];
}

// NAPI function
async function previewMerge(
  repoDir: string,
  sourceRef: string,
  targetRef: string
): Promise<MergePreview>;
```

### Conflict Resolution Strategies

```typescript
type MergeStrategy = 
  | 'fast-forward'      // Move ref only (no new commit)
  | 'recursive'         // Standard three-way merge
  | 'ours'              // Keep target version on conflict
  | 'theirs'            // Keep source version on conflict
  | 'squash'            // Squash all commits into one
  | 'cherry-pick';      // Select specific commits

interface MergeOptions {
  strategy: MergeStrategy;
  squashMessage?: string;
  allowUnrelatedHistories?: boolean;
  noCommit?: boolean;  // Prepare merge but don't commit
  autoResolve?: boolean;  // Auto-resolve using strategy
}
```

### Cherry-Pick Support

```rust
pub async fn cherry_pick_commits(
    repo: &Repository,
    commits: &[ObjectId],
    target_branch: &str,
) -> Result<Vec<ObjectId>, Error> {
    let mut new_commits = Vec::new();
    
    for commit_id in commits {
        let commit = repo.find_commit(*commit_id)?;
        let parent = commit.parent(0)?;
        
        // Merge the commit's changes onto target
        let outcome = gix_merge::tree::function::tree(
            parent.tree()?,
            repo.head()?.peel_to_commit()?.tree()?,
            commit.tree()?,
            Default::default(),
            &repo.objects,
        )?;
        
        // Create new commit
        let new_commit = create_commit(
            repo,
            outcome.tree.write(&repo.objects)?,
            commit.message().to_string(),
        )?;
        
        new_commits.push(new_commit);
    }
    
    Ok(new_commits)
}
```

## Multi-Worktree Coordination

### Detecting Parallel Conflicts

When multiple agents are working in worktrees simultaneously:

```typescript
interface WorktreeConflictCheck {
  worktreeId: string;
  branch: string;
  baseCommit: string;
  headCommit: string;
  changedPaths: string[];
}

async function detectParallelConflicts(
  repoDir: string,
  worktrees: WorktreeConflictCheck[]
): Promise<{
  conflicting: Array<{
    worktree1: string;
    worktree2: string;
    conflictingPaths: string[];
  }>;
}>;
```

### Merge Ordering Strategy

When multiple worktrees need to merge:

```typescript
interface MergeQueue {
  worktrees: string[];
  order: 'fifo' | 'priority' | 'dependency';
  conflictPolicy: 'abort' | 'skip' | 'manual';
}

async function queueMerges(
  repoDir: string,
  queue: MergeQueue
): Promise<MergeQueueResult>;
```

## Proposed TypeScript API

```typescript
interface MergeResult {
  success: boolean;
  type: 'fast-forward' | 'merge' | 'squash';
  newCommit?: string;
  conflicts?: MergeConflict[];
  message?: string;
}

// Main merge function
async function mergeWorktree(
  repoDir: string,
  worktreeId: string,
  options?: MergeOptions
): Promise<MergeResult>;

// Preview without executing
async function previewWorktreeMerge(
  repoDir: string,
  worktreeId: string
): Promise<MergePreview>;

// Resolve conflicts
async function resolveConflict(
  repoDir: string,
  filepath: string,
  resolution: 'ours' | 'theirs' | 'manual',
  content?: Uint8Array
): Promise<void>;

// Complete merge after resolution
async function finalizeMerge(
  repoDir: string,
  message: string
): Promise<string>; // Returns commit id

// Abort in-progress merge
async function abortMerge(
  repoDir: string
): Promise<void>;

// Squash merge
async function squashMergeWorktree(
  repoDir: string,
  worktreeId: string,
  message: string
): Promise<MergeResult>;

// Cherry-pick specific commits
async function cherryPickFromWorktree(
  repoDir: string,
  worktreeId: string,
  commits: string[]
): Promise<MergeResult>;

// Auto-prune worktree after successful merge
async function mergeAndPrune(
  repoDir: string,
  worktreeId: string,
  options?: MergeOptions
): Promise<MergeResult>;
```

## Agent Integration

### Merge Workflow for Agents

```typescript
// When child agent completes work
async function handleAgentCompletion(
  parentSession: AgentSession,
  childSession: AgentSession
): Promise<void> {
  const worktree = childSession.worktreeInfo;
  if (!worktree) return;
  
  // Preview merge first
  const preview = await previewWorktreeMerge(repoDir, worktree.id);
  
  if (preview.conflictsDetected) {
    // Notify parent about conflicts
    await notifyParentOfConflicts(parentSession, preview.conflicts);
    
    // Options:
    // 1. Let parent resolve manually
    // 2. Auto-resolve with strategy
    // 3. Keep worktree for later resolution
  } else {
    // Clean merge possible
    const result = await mergeAndPrune(repoDir, worktree.id, {
      strategy: 'squash',
      squashMessage: `[${childSession.workUnitId}] ${getWorkUnitTitle()}`,
    });
    
    if (result.success) {
      await cleanupSessionWorktree(childSession);
    }
  }
}
```

### Conflict Resolution UI Integration

```typescript
// For TUI conflict resolution view
interface ConflictViewData {
  filepath: string;
  baseContent: string;
  oursContent: string;
  theirsContent: string;
  mergedContent?: string;
}

async function getConflictViewData(
  repoDir: string,
  filepath: string
): Promise<ConflictViewData>;
```

## Safety Considerations

### Pre-Merge Checks

1. **Clean working directory** - No uncommitted changes
2. **Valid refs** - Both source and target refs exist
3. **No lock conflicts** - Worktree not locked by another process
4. **Backup checkpoint** - Create checkpoint before destructive ops

### Rollback Capability

```typescript
async function createMergeCheckpoint(
  repoDir: string,
  worktreeId: string
): Promise<string>; // Returns checkpoint name

async function rollbackMerge(
  repoDir: string,
  checkpointName: string
): Promise<void>;
```

## Testing Strategy

### Unit Tests
- Three-way merge with various conflict types
- Fast-forward detection
- Strategy application (ours/theirs)
- Cherry-pick operations

### Integration Tests
- Full merge workflow with worktrees
- Conflict detection and resolution
- Multi-worktree coordination
- Rollback scenarios

### Edge Cases
- Binary file conflicts
- Rename + modify conflicts
- Directory/file conflicts
- Empty merges (no changes)

## Performance Considerations

- Use gitoxide's parallel processing for large merges
- Stream large blob content instead of loading all in memory
- Cache merge bases for repeated operations
- Batch conflict detection for multiple files

## Dependencies

```toml
[dependencies]
gix = { version = "0.72", features = ["merge", "worktree-mutation"] }
gix-merge = "0.1"  # Direct dependency for advanced merge options
```

## References

- [gitoxide gix-merge crate](https://docs.rs/gix-merge)
- [Git Merge Internals](https://git-scm.com/docs/git-merge)
- [Three-Way Merge Algorithm](https://en.wikipedia.org/wiki/Merge_(version_control)#Three-way_merge)
- [Git Conflict Resolution](https://git-scm.com/book/en/v2/Git-Tools-Advanced-Merging)
