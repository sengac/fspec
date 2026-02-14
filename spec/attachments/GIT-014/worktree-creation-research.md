# GIT-014: Git Worktree Creation Research

## Executive Summary

This document captures research for enabling git worktree creation for spawned agent sessions, allowing concurrent agents to work in isolated working directories without conflicts.

## Git Worktree Internals

### Directory Structure

When `git worktree add` creates a new worktree:

```
main-repo/
├── .git/                          # Main repository
│   ├── worktrees/                 # Linked worktree metadata
│   │   └── feature-branch/        # One folder per worktree
│   │       ├── HEAD               # Worktree's HEAD
│   │       ├── index              # Worktree's staging area
│   │       ├── gitdir             # Path to worktree's .git file
│   │       ├── commondir          # Path to shared .git
│   │       └── locked             # (optional) Lock file with reason
│   └── ...
│
worktree-path/                     # The actual worktree directory
├── src/
├── README.md
└── .git                           # File (not directory!) pointing back
                                   # Contains: "gitdir: /path/to/main/.git/worktrees/feature-branch"
```

### Key Concepts

1. **Main Worktree**: The original working directory (created by `git init` or `git clone`)
2. **Linked Worktree**: Additional worktrees created by `git worktree add`
3. **Common Directory**: Shared git data (`objects/`, `refs/`, `config`)
4. **Private Data**: Per-worktree (`HEAD`, `index`, `logs/HEAD`)

### gitdir File Format

The `.git` file in a linked worktree contains a single line:
```
gitdir: /absolute/path/to/main-repo/.git/worktrees/<worktree-id>
```

## gitoxide Worktree Support

### Current API Analysis

From `/tmp/gitoxide/gix/src/repository/worktree.rs`:

```rust
impl Repository {
    /// Return a list of all **linked** worktrees sorted by private git dir path
    pub fn worktrees(&self) -> std::io::Result<Vec<worktree::Proxy<'_>>>
    
    /// Return the worktree identified by the given `id`
    pub fn worktree_proxy_by_id<'a>(&self, id: impl Into<&'a BStr>) -> Option<worktree::Proxy<'_>>
    
    /// Return the currently set worktree
    pub fn worktree(&self) -> Option<Worktree<'_>>
    
    /// Return true if this repository is bare
    pub fn is_bare(&self) -> bool
}
```

From `/tmp/gitoxide/gix/src/worktree/mod.rs`:

```rust
pub struct Proxy<'repo> {
    parent: &'repo Repository,
    git_dir: PathBuf,
}

impl Proxy<'_> {
    /// Read the location of the checkout, the base of the work tree
    pub fn base(&self) -> std::io::Result<PathBuf>
    
    /// The git directory for the work tree
    pub fn git_dir(&self) -> &Path
    
    /// The name of the worktree
    pub fn id(&self) -> &BStr
    
    /// Return true if the worktree is locked
    pub fn is_locked(&self) -> bool
    
    /// Provide a reason for the locking
    pub fn lock_reason(&self) -> Option<BString>
    
    /// Transform this proxy into a Repository
    pub fn into_repo(self) -> Result<Repository, into_repo::Error>
}
```

### Missing Functionality

gitoxide currently **does NOT** have a high-level API for:
- `worktree add` (creating new worktrees)
- `worktree remove` (removing worktrees)
- `worktree prune` (cleaning up stale worktrees)

**These need to be implemented manually or by calling git CLI as fallback.**

## Implementation Strategy

### Worktree Creation Flow

```
1. Agent spawn request received
2. Generate unique worktree ID (agent-session-<uuid>)
3. Create worktree directory: .fspec/worktrees/<agent-id>/
4. Create branch or checkout existing
5. Initialize worktree structure:
   a. Create .git/worktrees/<id>/ directory
   b. Write HEAD file
   c. Write gitdir file (points to worktree's .git)
   d. Write commondir file (points to main .git)
   e. Create index (copy from main or fresh)
   f. Write .git file in worktree directory
6. Checkout files to worktree directory
7. Return worktree info to agent
```

### Proposed API

```typescript
// TypeScript interface for worktree operations
interface WorktreeCreateOptions {
  /** Base path for the worktree (e.g., .fspec/worktrees/) */
  basePath: string;
  /** Unique identifier for the worktree */
  worktreeId: string;
  /** Branch to checkout (create if doesn't exist) */
  branch?: string;
  /** Commit/ref to base new branch on */
  startPoint?: string;
  /** If true, create new branch even if it exists */
  force?: boolean;
}

interface WorktreeInfo {
  /** Worktree ID */
  id: string;
  /** Absolute path to worktree directory */
  path: string;
  /** Current branch name */
  branch: string;
  /** Current HEAD commit */
  head: string;
  /** Whether worktree is locked */
  locked: boolean;
  /** Lock reason if locked */
  lockReason?: string;
}

// NAPI functions
async function createWorktree(
  repoDir: string,
  options: WorktreeCreateOptions
): Promise<WorktreeInfo>;

async function listWorktrees(repoDir: string): Promise<WorktreeInfo[]>;

async function removeWorktree(
  repoDir: string,
  worktreeId: string,
  force?: boolean
): Promise<boolean>;

async function lockWorktree(
  repoDir: string,
  worktreeId: string,
  reason?: string
): Promise<void>;

async function unlockWorktree(
  repoDir: string,
  worktreeId: string
): Promise<void>;

async function pruneWorktrees(
  repoDir: string,
  dryRun?: boolean
): Promise<string[]>;
```

### Rust Implementation Approach

Since gitoxide lacks `worktree add`, we have two options:

#### Option A: Manual Implementation (Preferred)

```rust
use gix::Repository;
use std::path::Path;
use std::fs;

pub fn create_worktree(
    repo: &Repository,
    worktree_path: &Path,
    branch_name: &str,
    create_branch: bool,
) -> Result<(), Error> {
    let worktree_id = worktree_path.file_name()
        .ok_or_else(|| Error::InvalidPath)?;
    
    let git_dir = repo.git_dir();
    let worktrees_dir = git_dir.join("worktrees").join(worktree_id);
    
    // 1. Create worktrees metadata directory
    fs::create_dir_all(&worktrees_dir)?;
    
    // 2. Write HEAD file
    let head_content = if create_branch {
        format!("ref: refs/heads/{}", branch_name)
    } else {
        format!("ref: refs/heads/{}", branch_name)
    };
    fs::write(worktrees_dir.join("HEAD"), head_content)?;
    
    // 3. Write gitdir file (path from worktree metadata to worktree's .git file)
    let worktree_gitfile = worktree_path.join(".git");
    fs::write(worktrees_dir.join("gitdir"), worktree_gitfile.to_string_lossy().as_bytes())?;
    
    // 4. Write commondir file
    fs::write(worktrees_dir.join("commondir"), "../..")?;
    
    // 5. Create worktree directory and .git file
    fs::create_dir_all(worktree_path)?;
    fs::write(
        &worktree_gitfile,
        format!("gitdir: {}", worktrees_dir.display())
    )?;
    
    // 6. Checkout files using gitoxide
    // Use gix-worktree-state for checkout
    // ...
    
    Ok(())
}
```

#### Option B: CLI Fallback

```rust
use std::process::Command;

pub fn create_worktree_via_cli(
    repo_path: &Path,
    worktree_path: &Path,
    branch: &str,
) -> Result<(), Error> {
    let output = Command::new("git")
        .args(&["worktree", "add", "-b", branch])
        .arg(worktree_path)
        .current_dir(repo_path)
        .output()?;
    
    if !output.status.success() {
        return Err(Error::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string()
        ));
    }
    
    Ok(())
}
```

**Recommendation**: Start with CLI fallback for MVP, then implement native gitoxide version.

## Agent Session Integration

### Session-Worktree Association

```typescript
interface AgentSession {
  sessionId: string;
  parentSessionId?: string;
  worktreeInfo?: WorktreeInfo;
  workingDirectory: string; // Main repo or worktree path
}

// When spawning a new agent
async function spawnAgentWithWorktree(
  parentSession: AgentSession,
  workUnitId: string
): Promise<AgentSession> {
  const worktreeId = `agent-${workUnitId}-${Date.now()}`;
  const worktreePath = join(repoRoot, '.fspec', 'worktrees', worktreeId);
  
  const worktree = await createWorktree(repoRoot, {
    basePath: join(repoRoot, '.fspec', 'worktrees'),
    worktreeId,
    branch: `agent/${workUnitId}`,
    startPoint: 'HEAD',
  });
  
  return {
    sessionId: generateId(),
    parentSessionId: parentSession.sessionId,
    worktreeInfo: worktree,
    workingDirectory: worktree.path,
  };
}
```

### Cleanup Strategy

```typescript
// Track active worktrees in session metadata
interface SessionMetadata {
  activeWorktrees: Map<string, WorktreeInfo>;
}

// On session end
async function cleanupSessionWorktree(session: AgentSession): Promise<void> {
  if (session.worktreeInfo) {
    await removeWorktree(repoRoot, session.worktreeInfo.id, true);
  }
}

// Periodic cleanup of orphaned worktrees
async function pruneOrphanedWorktrees(): Promise<void> {
  const worktrees = await listWorktrees(repoRoot);
  const activeSessions = getActiveSessions();
  
  for (const wt of worktrees) {
    if (!activeSessions.some(s => s.worktreeInfo?.id === wt.id)) {
      if (wt.id.startsWith('agent-')) {
        await removeWorktree(repoRoot, wt.id, true);
      }
    }
  }
}
```

## Considerations

### Branch Naming Convention

- Format: `agent/<work-unit-id>` or `agent/<session-id>`
- Allows easy identification of agent branches
- Can be cleaned up by pattern matching

### Worktree Location

Recommended: `.fspec/worktrees/<worktree-id>/`
- Inside project root but gitignored
- Easy to find and manage
- Can be on same filesystem for hardlinks

### Lock Files

Use locking to prevent concurrent access:
```rust
async fn lock_worktree_for_session(
    repo: &Repository,
    worktree_id: &str,
    session_id: &str,
) -> Result<(), Error> {
    let lock_path = repo.git_dir()
        .join("worktrees")
        .join(worktree_id)
        .join("locked");
    
    fs::write(&lock_path, format!("Session: {}", session_id))?;
    Ok(())
}
```

### Index Isolation

Each worktree has its own index, ensuring:
- Independent staging areas
- No interference between agents
- Clean git status per worktree

## Testing Strategy

1. **Unit Tests**
   - Worktree creation/removal
   - Branch operations within worktrees
   - Lock/unlock operations

2. **Integration Tests**
   - Multiple agents with worktrees
   - Parallel operations
   - Cleanup after session end

3. **Edge Cases**
   - Worktree on missing directory
   - Stale worktree metadata
   - Concurrent worktree operations

## Dependencies

```toml
[dependencies]
gix = { version = "0.72", features = ["worktree-stream", "worktree-mutation"] }
```

## References

- [Git Worktree Documentation](https://git-scm.com/docs/git-worktree)
- [gitoxide worktree module](https://docs.rs/gix/latest/gix/worktree/)
- [Git Worktree Internals](https://github.com/git/git/blob/master/Documentation/gitrepository-layout.txt)
