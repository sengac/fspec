# OpenAI Codex Git Methodology Research

## Executive Summary

**Key Finding: OpenAI Codex does NOT use git worktrees for agent isolation.**

Instead, they use a technique called "Ghost Commits" - detached commit objects that capture the working tree state without modifying branches or refs. This is combined with platform-specific sandboxing (Seatbelt on macOS, Landlock/bubblewrap on Linux) for process isolation.

## Ghost Commits System

### What is a Ghost Commit?

A ghost commit is a detached git commit object that:
- Captures the complete working tree state (tracked, staged, and untracked files)
- Has no branch reference (never visible in `git log` on any branch)
- Preserves the relationship to HEAD via parent commit
- Can be restored to return working directory to exact state

### Core Data Structure

```rust
pub struct GhostCommit {
    id: CommitID,                           // SHA of the ghost commit
    parent: Option<CommitID>,               // Original HEAD when snapshot taken
    preexisting_untracked_files: Vec<PathBuf>, // Files to preserve on restore
    preexisting_untracked_dirs: Vec<PathBuf>,  // Dirs to preserve on restore
}
```

### Creation Flow

1. **Capture status snapshot** using `git status --porcelain=2 -z`
2. **Create temporary index** (`GIT_INDEX_FILE=/tmp/index`) to avoid disturbing user's staging area
3. **Pre-populate temp index** with HEAD using `git read-tree HEAD`
4. **Stage working tree changes** using `git add --all` on temp index
5. **Write tree** using `git write-tree`
6. **Create detached commit** using `git commit-tree <tree> -p <parent> -m "codex snapshot"`
7. **Return ghost commit object** without updating any refs

### Key Innovation: Temporary Index

```rust
let index_tempdir = Builder::new().prefix("codex-git-index-").tempdir()?;
let index_path = index_tempdir.path().join("index");
let base_env = vec![(
    OsString::from("GIT_INDEX_FILE"),
    OsString::from(index_path.as_os_str()),
)];
```

This ensures:
- User's staged changes are never disturbed
- Snapshot is atomic and isolated
- No side effects on `git status` after snapshot

### Restoration Flow

1. **Resolve ghost commit** from stored commit ID
2. **Get current untracked files** for comparison
3. **Restore working tree** using `git restore --source <commit> --worktree -- .`
4. **Clean up new files** that didn't exist in the snapshot
5. **Preserve pre-existing untracked files** (files that existed before snapshot)

```rust
fn restore_to_commit_inner(repo_root: &Path, repo_prefix: Option<&Path>, commit_id: &str) {
    let restore_args = vec![
        "restore",
        "--source", commit_id,
        "--worktree",  // Intentionally NOT --staged to preserve user's index
        "--",
        ".",
    ];
    run_git_for_status(repo_root, restore_args, None)?;
}
```

## Large File Handling

Codex excludes large files from ghost snapshots to prevent bloat:

### Configurable Thresholds

```rust
const DEFAULT_IGNORE_LARGE_UNTRACKED_FILES: i64 = 10 * 1024 * 1024;  // 10 MiB
const DEFAULT_IGNORE_LARGE_UNTRACKED_DIRS: i64 = 200;  // 200+ files = "large"
```

### Always-Ignored Directories

```rust
const DEFAULT_IGNORED_DIR_NAMES: &[&str] = &[
    "node_modules", ".venv", "venv", "env", ".env",
    "dist", "build", ".pytest_cache", ".mypy_cache",
    ".cache", ".tox", "__pycache__",
];
```

### Preservation During Restore

Large files are:
- Tracked in `preexisting_untracked_files`
- Never deleted during restore
- Never captured in snapshot commit (saves space)

## Process Sandboxing (Separate from Git)

Codex uses OS-level sandboxing for command execution, NOT git-based isolation:

### macOS: Seatbelt (`/usr/bin/sandbox-exec`)
- Restricts filesystem access to writable roots
- Can disable network access

### Linux: Landlock + Bubblewrap
- Uses `codex-linux-sandbox` helper binary
- Seccomp for network restrictions
- Filesystem access control via bubblewrap

### Windows: Elevated Sandbox
- Uses `codex-windows-sandbox` crate
- Registry-based permissions

## Why NOT Git Worktrees?

The research documents note:

> "Note that this does **not** detect *work‑trees* created with `git worktree add` where the checkout lives outside the main repository directory."

Codex chose ghost commits over worktrees because:

1. **Simplicity**: Ghost commits work with existing git CLI commands
2. **No branch pollution**: Worktrees require branches; ghost commits are branchless
3. **Atomic undo**: Ghost commit restore is a single operation
4. **No cleanup needed**: Ghost commits don't leave filesystem artifacts
5. **Works in subdirectories**: Can snapshot from any subdirectory of a repo

## Relevance to fspec Work Units

### GIT-013 (isomorphic-git → gitoxide)
- Codex uses git CLI, not a git library
- They wrap commands with timeouts (5 second default)
- Status parsing uses `--porcelain=2 -z` format
- Consider: Should we use CLI fallback for complex operations?

### GIT-014 (Worktree Creation)
- **Alternative approach**: Ghost commits may be simpler than worktrees
- If proceeding with worktrees, note gitoxide lacks `worktree add` API
- Codex's sandbox approach (process isolation) is orthogonal to git isolation

### GIT-015 (Worktree Merge)
- Ghost commits don't need merge - they're restore-only
- If using worktrees, this complexity is still needed
- Consider hybrid: Ghost commits for undo, worktrees only for parallel agents

## API Surface Reference

### Creating a Snapshot
```typescript
interface CreateGhostCommitOptions {
  repo_path: string;
  message?: string;
  force_include?: string[];  // Files to include even if gitignored
  ignore_large_untracked_files?: number;  // Bytes threshold
  ignore_large_untracked_dirs?: number;   // File count threshold
}
```

### Snapshot Report
```typescript
interface GhostSnapshotReport {
  large_untracked_dirs: { path: string; file_count: number }[];
  ignored_untracked_files: { path: string; byte_size: number }[];
}
```

### Restoring
```typescript
interface RestoreGhostCommitOptions {
  repo_path: string;
  ignore_large_untracked_files?: number;
  ignore_large_untracked_dirs?: number;
}
```

## Recommendations for fspec

### Option A: Adopt Ghost Commits (Simpler)
- Replace checkpoint system with ghost commit approach
- Use gitoxide for blob reading, CLI for complex operations
- No worktree complexity needed

### Option B: Hybrid Approach
- Ghost commits for undo/restore (single agent)
- Worktrees for parallel agent isolation (multi-agent)
- Keep both systems separate

### Option C: Full Worktree Implementation
- Proceed with GIT-014/GIT-015 as planned
- Accept complexity of merge management
- Use CLI fallback for worktree creation

## Source Files Reviewed

| File | Purpose |
|------|---------|
| `codex-rs/utils/git/src/ghost_commits.rs` | Core ghost commit implementation |
| `codex-rs/utils/git/src/lib.rs` | Public API exports |
| `codex-rs/core/src/git_info.rs` | Git repository detection |
| `codex-rs/core/src/seatbelt.rs` | macOS sandboxing |
| `codex-rs/core/src/landlock.rs` | Linux sandboxing |

## References

- [OpenAI Codex Repository](https://github.com/openai/codex)
- [Git Internals - Plumbing Commands](https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain)
- [git commit-tree](https://git-scm.com/docs/git-commit-tree)
- [git restore](https://git-scm.com/docs/git-restore)
