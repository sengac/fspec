//! Git operations NAPI bindings
//!
//! Exposes codelet-git operations to TypeScript via NAPI-RS bindings.

use napi_derive::napi;
use std::path::Path;

// =============================================================================
// Status Operations
// =============================================================================

/// Get list of staged files (files added to the index via git add)
///
/// @param dir - Path to the repository root
/// @returns Array of file paths (relative to repository root) that are staged
#[napi]
pub fn get_staged_files(dir: String) -> napi::Result<Vec<String>> {
    codelet_git::get_staged_files(&dir)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Get list of unstaged files (modified files not yet staged)
///
/// @param dir - Path to the repository root
/// @returns Array of file paths that have unstaged modifications
#[napi]
pub fn get_unstaged_files(dir: String) -> napi::Result<Vec<String>> {
    codelet_git::get_unstaged_files(&dir)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Get list of untracked files (files not tracked by git)
///
/// @param dir - Path to the repository root
/// @returns Array of file paths that are not tracked by git
#[napi]
pub fn get_untracked_files(dir: String) -> napi::Result<Vec<String>> {
    codelet_git::get_untracked_files(&dir)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Get unified diff for a file comparing working directory to HEAD
///
/// @param dir - Path to the repository root
/// @param filepath - Path to the file (relative to repository root)
/// @returns Unified diff string, or null if no changes
#[napi]
pub fn get_file_diff(dir: String, filepath: String) -> napi::Result<Option<String>> {
    codelet_git::get_file_diff(&dir, &filepath)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Get unified diff for a single file between HEAD and a checkpoint commit
///
/// Shows what would change if the checkpoint were restored:
/// HEAD content is shown as "old" (lines removed), checkpoint as "new" (lines added).
///
/// @param dir - Path to the repository root
/// @param filepath - Path to the file (relative to repository root)
/// @param checkpointRef - Full ref or SHA of the checkpoint commit
/// @returns Unified diff string, or null if no changes
#[napi]
pub fn get_checkpoint_file_diff(
    dir: String,
    filepath: String,
    checkpoint_ref: String,
) -> napi::Result<Option<String>> {
    codelet_git::get_checkpoint_file_diff(&dir, &filepath, &checkpoint_ref)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Get current branch name
///
/// @param dir - Path to the repository root
/// @returns Branch name, or undefined if in detached HEAD state
#[napi]
pub fn get_current_branch(dir: String) -> napi::Result<Option<String>> {
    codelet_git::get_current_branch(&dir)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

// =============================================================================
// Worktree Operations
// =============================================================================

/// Result of creating a worktree
#[napi(object)]
pub struct WorktreeCreateResultJs {
    /// Session ID this worktree belongs to
    pub session_id: String,
    /// Absolute path to the worktree directory
    pub path: String,
    /// Commit SHA the worktree HEAD points to
    pub head_commit: String,
    /// Whether the worktree is in detached HEAD mode
    pub is_detached: bool,
    /// The base commit the worktree was created from
    pub base_commit: String,
    /// When the worktree was created (ISO 8601 format)
    pub created_at: String,
}

/// Information about a worktree
#[napi(object)]
pub struct WorktreeInfoJs {
    /// Session ID this worktree belongs to
    pub session_id: String,
    /// Absolute path to the worktree directory
    pub path: String,
    /// Commit SHA the worktree HEAD points to
    pub head_commit: String,
    /// Whether the worktree is in detached HEAD mode
    pub is_detached: bool,
}

/// Create a worktree for a session at HEAD
///
/// @param repoPath - Path to the git repository
/// @param sessionId - Unique session identifier
/// @returns WorktreeCreateResult with worktree info and metadata
#[napi]
pub fn create_worktree(repo_path: String, session_id: String) -> napi::Result<WorktreeCreateResultJs> {
    let result = codelet_git::create_worktree(&repo_path, &session_id)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(WorktreeCreateResultJs {
        session_id: result.info.session_id,
        path: result.info.path.to_string_lossy().to_string(),
        head_commit: result.info.head_commit,
        is_detached: result.info.is_detached,
        base_commit: result.base_commit,
        created_at: result.created_at.to_rfc3339(),
    })
}

/// Create a worktree for a session at a specific commit ref
///
/// @param repoPath - Path to the git repository
/// @param sessionId - Unique session identifier
/// @param commitRef - Optional commit reference (defaults to HEAD)
/// @returns WorktreeCreateResult with worktree info and metadata
#[napi]
pub fn create_worktree_at_ref(
    repo_path: String,
    session_id: String,
    commit_ref: Option<String>,
) -> napi::Result<WorktreeCreateResultJs> {
    let result = codelet_git::create_worktree_at_ref(
        &repo_path,
        &session_id,
        commit_ref.as_deref(),
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(WorktreeCreateResultJs {
        session_id: result.info.session_id,
        path: result.info.path.to_string_lossy().to_string(),
        head_commit: result.info.head_commit,
        is_detached: result.info.is_detached,
        base_commit: result.base_commit,
        created_at: result.created_at.to_rfc3339(),
    })
}

/// Remove a worktree for a session
///
/// @param repoPath - Path to the git repository
/// @param sessionId - Session identifier of the worktree to remove
#[napi]
pub fn remove_worktree(repo_path: String, session_id: String) -> napi::Result<()> {
    codelet_git::remove_worktree(&repo_path, &session_id)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// List all session worktrees in a repository
///
/// @param repoPath - Path to the git repository
/// @returns Array of WorktreeInfo for each worktree found
#[napi]
pub fn list_worktrees(repo_path: String) -> napi::Result<Vec<WorktreeInfoJs>> {
    let worktrees = codelet_git::list_worktrees(&repo_path)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(worktrees
        .into_iter()
        .map(|w| WorktreeInfoJs {
            session_id: w.session_id,
            path: w.path.to_string_lossy().to_string(),
            head_commit: w.head_commit,
            is_detached: w.is_detached,
        })
        .collect())
}

// =============================================================================
// Session Result Operations
// =============================================================================

/// Result of getting a session diff
#[napi(object)]
pub struct SessionResultJs {
    /// Session ID this result belongs to
    pub session_id: String,
    /// Unified diff of all changes
    pub diff: String,
    /// List of files that were modified
    pub files_changed: Vec<String>,
    /// List of files that were added
    pub files_added: Vec<String>,
    /// List of files that were deleted
    pub files_deleted: Vec<String>,
    /// The base commit the session was created from
    pub base_commit: String,
}

/// Get session diff comparing base commit to current worktree state
///
/// This compares the base_commit tree against the worktree's working directory,
/// capturing all changes including uncommitted modifications.
/// The worktree remains intact after this operation.
///
/// @param repoPath - Path to the main git repository
/// @param sessionId - Session identifier
/// @returns SessionResult with unified diff and file lists
#[napi]
pub fn get_session_diff(repo_path: String, session_id: String) -> napi::Result<SessionResultJs> {
    let result = codelet_git::get_session_diff(&repo_path, &session_id)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(SessionResultJs {
        session_id: result.session_id,
        diff: result.diff,
        files_changed: result.files_changed,
        files_added: result.files_added,
        files_deleted: result.files_deleted,
        base_commit: result.base_commit,
    })
}

/// Apply session changes by copying files from session worktree to main worktree
///
/// This copies modified/added files and removes deleted files from the main worktree.
/// After successful application, the session worktree is removed.
///
/// @param repoPath - Path to the main git repository
/// @param sessionId - Session identifier
/// @throws Error if conflicts detected or worktree not found
#[napi]
pub fn apply_session_changes(repo_path: String, session_id: String) -> napi::Result<()> {
    codelet_git::apply_session_changes(&repo_path, &session_id)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Abort a session by removing its worktree without applying changes
///
/// @param repoPath - Path to the main git repository
/// @param sessionId - Session identifier
#[napi]
pub fn abort_session(repo_path: String, session_id: String) -> napi::Result<()> {
    codelet_git::abort_session(&repo_path, &session_id)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

// =============================================================================
// Ghost Commit Checkpoint Operations
// =============================================================================

/// Result of creating a ghost commit checkpoint
#[napi(object)]
pub struct GhostCheckpointJs {
    /// SHA of the ghost commit
    pub sha: String,
    /// SHA of the parent commit (HEAD at creation time)
    pub parent_sha: String,
    /// List of files captured in the checkpoint
    pub files: Vec<String>,
}

/// Result of restoring a ghost commit checkpoint
#[napi(object)]
pub struct RestoreResultJs {
    /// Whether restore was successful
    pub success: bool,
    /// Files that were restored
    pub restored_files: Vec<String>,
    /// Files that were deleted (existed after checkpoint but not in it)
    pub deleted_files: Vec<String>,
}

/// Create a ghost commit checkpoint capturing current working tree state
///
/// Ghost commits are detached commits that capture complete working tree state
/// (staged, unstaged, untracked files) without disturbing the user's staging area.
/// They are invisible to git log but can be restored later.
///
/// @param dir - Path to the repository root
/// @param workUnitId - Work unit identifier for ref namespace
/// @param checkpointName - Name for the checkpoint
/// @returns GhostCheckpoint with SHA, parent SHA, and captured files
#[napi]
pub fn create_ghost_checkpoint(
    dir: String,
    work_unit_id: String,
    checkpoint_name: String,
) -> napi::Result<GhostCheckpointJs> {
    let result = codelet_git::ghost_commit::create_ghost_commit(
        Path::new(&dir),
        &work_unit_id,
        &checkpoint_name,
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(GhostCheckpointJs {
        sha: result.sha,
        parent_sha: result.parent_sha,
        files: result.files,
    })
}

/// Restore working tree from ghost commit checkpoint
///
/// Restores all files from the checkpoint and deletes files that were added
/// after the checkpoint was created, returning the working tree to the exact
/// state at checkpoint creation.
///
/// @param dir - Path to the repository root
/// @param workUnitId - Work unit identifier
/// @param checkpointName - Name of the checkpoint to restore
/// @param force - If true, overwrite without conflict detection
/// @returns RestoreResult with success status and affected files
#[napi]
pub fn restore_ghost_checkpoint(
    dir: String,
    work_unit_id: String,
    checkpoint_name: String,
    force: Option<bool>,
) -> napi::Result<RestoreResultJs> {
    let result = codelet_git::ghost_commit::restore_ghost_commit(
        Path::new(&dir),
        &work_unit_id,
        &checkpoint_name,
        force.unwrap_or(false),
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(RestoreResultJs {
        success: result.success,
        restored_files: result.restored_files,
        deleted_files: result.deleted_files,
    })
}

/// List all ghost commit checkpoints for a work unit
///
/// @param dir - Path to the repository root
/// @param workUnitId - Work unit identifier
/// @returns Array of checkpoint names
#[napi]
pub fn list_ghost_checkpoints(
    dir: String,
    work_unit_id: String,
) -> napi::Result<Vec<String>> {
    codelet_git::ghost_commit::list_ghost_checkpoints(
        Path::new(&dir),
        &work_unit_id,
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Delete a ghost commit checkpoint
///
/// @param dir - Path to the repository root
/// @param workUnitId - Work unit identifier
/// @param checkpointName - Name of the checkpoint to delete
#[napi]
pub fn delete_ghost_checkpoint(
    dir: String,
    work_unit_id: String,
    checkpoint_name: String,
) -> napi::Result<()> {
    codelet_git::ghost_commit::delete_ghost_checkpoint(
        Path::new(&dir),
        &work_unit_id,
        &checkpoint_name,
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Get files that changed between checkpoint and current working tree
///
/// @param dir - Path to the repository root
/// @param workUnitId - Work unit identifier
/// @param checkpointName - Name of the checkpoint
/// @returns Array of file paths that differ
#[napi]
pub fn get_checkpoint_diff_files(
    dir: String,
    work_unit_id: String,
    checkpoint_name: String,
) -> napi::Result<Vec<String>> {
    codelet_git::ghost_commit::get_checkpoint_diff_files(
        Path::new(&dir),
        &work_unit_id,
        &checkpoint_name,
    )
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// RPC-015: Count manual + auto checkpoints across all work units.
///
/// Mirrors the TS `countCheckpoints(cwd)` helper from
/// `src/utils/checkpoint-index.ts` but reads directly from
/// `refs/fspec-checkpoints/...` git refs (rather than the
/// `.git/fspec-checkpoints-index/{workUnitId}.json` sidecar files)
/// so both UIs converge on the SAME source of truth.
///
/// The existing TS pure-JS `countCheckpoints` helper is NOT changed by
/// this card — it can switch to this NAPI export at its own pace.
/// Both paths converge in `codelet_git::ghost_commit::count_checkpoints`.
///
/// @param cwd - Path to the workspace root (containing `.git/`)
/// @returns CheckpointCounts with `manual` + `auto` u32 fields
#[napi]
pub fn count_checkpoints(cwd: String) -> napi::Result<codelet_rpc_types::CheckpointCounts> {
    codelet_git::ghost_commit::count_checkpoints(Path::new(&cwd))
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

// =============================================================================
// Repository Operations (GIT-039: resolve_ref, init, add, commit, setConfig)
// =============================================================================

/// Resolve a git ref to its target commit SHA
///
/// @param dir - Path to the repository root
/// @param refName - Full ref path (e.g. "refs/fspec-checkpoints/GIT-039/baseline")
/// @returns Hex string of the resolved commit SHA
#[napi]
pub fn resolve_ref(dir: String, ref_name: String) -> napi::Result<String> {
    codelet_git::resolve_ref(&dir, &ref_name)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Initialize a new git repository
///
/// @param dir - Path to create the repository at
/// @param defaultBranch - Name of the default branch (e.g. "main")
#[napi]
pub fn git_init(dir: String, default_branch: String) -> napi::Result<()> {
    codelet_git::git_init(&dir, &default_branch)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Set a git config value
///
/// @param dir - Path to the repository root
/// @param key - Config key (e.g. "user.name")
/// @param value - Config value
#[napi]
pub fn git_set_config(dir: String, key: String, value: String) -> napi::Result<()> {
    codelet_git::git_set_config(&dir, &key, &value)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Stage a file (equivalent to `git add <filepath>`)
///
/// @param dir - Path to the repository root
/// @param filepath - Path to the file relative to repository root
#[napi]
pub fn git_add(dir: String, filepath: String) -> napi::Result<()> {
    codelet_git::git_add(&dir, &filepath)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Create a commit from the current index
///
/// @param dir - Path to the repository root
/// @param message - Commit message
/// @param authorName - Author name
/// @param authorEmail - Author email
/// @returns Hex string of the new commit SHA
#[napi]
pub fn git_commit(
    dir: String,
    message: String,
    author_name: String,
    author_email: String,
) -> napi::Result<String> {
    codelet_git::git_commit(&dir, &message, &author_name, &author_email)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

// =============================================================================
// Session Worktree Operations (GIT-027)
// =============================================================================

use std::collections::HashSet;

/// Session information with derived status for listing
#[napi(object)]
pub struct SessionInfoJs {
    /// Session ID
    pub session_id: String,
    /// Derived status: "active", "pending_merge", "clean", "orphaned"
    pub status: String,
    /// Base commit the worktree was created from
    pub base_commit: String,
    /// Number of files changed (modified + added + deleted)
    pub files_changed: u32,
    /// When the session was created (ISO 8601 format)
    pub created_at: String,
    /// Path to the worktree
    pub worktree_path: String,
}

/// Result of merging a session
#[napi(object)]
pub struct MergeResultJs {
    /// Session ID that was merged
    pub session_id: String,
    /// Files that were modified in main
    pub files_modified: Vec<String>,
    /// Files that were added to main
    pub files_added: Vec<String>,
    /// Files that were deleted from main
    pub files_deleted: Vec<String>,
}

/// Result of discarding a session
#[napi(object)]
pub struct DiscardResultJs {
    /// Session ID that was discarded
    pub session_id: String,
    /// Number of files that were in the session (not applied)
    pub files_discarded: u32,
}

/// Result of pruning orphaned sessions
#[napi(object)]
pub struct PruneResultJs {
    /// Number of orphaned worktrees that were pruned
    pub count: u32,
    /// List of session IDs that were pruned
    pub pruned: Vec<String>,
}

/// List all session worktrees with status information
///
/// Returns information about all session worktrees, optionally filtered by status.
///
/// @param repoPath - Path to the git repository
/// @param activeSessions - Array of currently active session IDs (from BackgroundSession)
/// @param filter - Optional filter: "all", "active", "pending_merge", "clean", "orphaned"
/// @returns Array of SessionInfo objects
#[napi]
#[allow(dead_code)]
pub fn list_sessions(
    repo_path: String,
    active_sessions: Vec<String>,
    filter: Option<String>,
) -> napi::Result<Vec<SessionInfoJs>> {
    let active_set: HashSet<String> = active_sessions.into_iter().collect();
    
    let session_filter = match filter.as_deref() {
        Some("active") => codelet_git::SessionFilter::Active,
        Some("pending_merge") => codelet_git::SessionFilter::PendingMerge,
        Some("clean") => codelet_git::SessionFilter::Clean,
        Some("orphaned") => codelet_git::SessionFilter::Orphaned,
        _ => codelet_git::SessionFilter::All,
    };
    
    let sessions = codelet_git::list_sessions(&repo_path, &active_set, session_filter)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    
    Ok(sessions
        .into_iter()
        .map(|s| SessionInfoJs {
            session_id: s.session_id,
            status: s.status.to_string().to_lowercase(),
            base_commit: s.base_commit,
            files_changed: s.files_changed as u32,
            created_at: s.created_at.to_rfc3339(),
            worktree_path: s.worktree_path.to_string_lossy().to_string(),
        })
        .collect())
}

/// Inspect session diff before merging
///
/// Returns diff information without modifying the worktree or any session state.
///
/// @param repoPath - Path to the git repository
/// @param sessionId - Session identifier
/// @returns SessionResult with unified diff and file lists
#[napi]
pub fn inspect_session(repo_path: String, session_id: String) -> napi::Result<SessionResultJs> {
    let result = codelet_git::inspect_session(&repo_path, &session_id)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    
    Ok(SessionResultJs {
        session_id: result.session_id,
        diff: result.diff,
        files_changed: result.files_changed,
        files_added: result.files_added,
        files_deleted: result.files_deleted,
        base_commit: result.base_commit,
    })
}

/// Merge session changes to main worktree
///
/// Applies all changes from session to main and removes worktree on success.
/// Returns conflict error if main has diverged since session base commit.
///
/// @param repoPath - Path to the git repository
/// @param sessionId - Session identifier
/// @returns MergeResult with file lists on success
/// @throws Error with "Conflict" and file list if main has conflicting changes
#[napi]
pub fn merge_session(repo_path: String, session_id: String) -> napi::Result<MergeResultJs> {
    let result = codelet_git::merge_session(&repo_path, &session_id)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    
    Ok(MergeResultJs {
        session_id: result.session_id,
        files_modified: result.files_modified,
        files_added: result.files_added,
        files_deleted: result.files_deleted,
    })
}

/// Discard session without applying any changes
///
/// Removes the worktree and cleans up git metadata without
/// applying any of the session's changes to the main worktree.
///
/// @param repoPath - Path to the git repository
/// @param sessionId - Session identifier
/// @returns DiscardResult with session ID and files discarded count
#[napi]
pub fn discard_session(repo_path: String, session_id: String) -> napi::Result<DiscardResultJs> {
    let result = codelet_git::discard_session(&repo_path, &session_id)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    
    Ok(DiscardResultJs {
        session_id: result.session_id,
        files_discarded: result.files_discarded as u32,
    })
}

/// Prune all orphaned session worktrees
///
/// Removes worktrees that have no valid session record.
/// Active sessions are never pruned.
///
/// @param repoPath - Path to the git repository
/// @param activeSessions - Array of currently active session IDs
/// @returns PruneResult with count and list of pruned session IDs
#[napi]
pub fn prune_orphaned(
    repo_path: String,
    active_sessions: Vec<String>,
) -> napi::Result<PruneResultJs> {
    let active_set: HashSet<String> = active_sessions.into_iter().collect();
    
    let result = codelet_git::prune_orphaned(&repo_path, &active_set)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    
    Ok(PruneResultJs {
        count: result.count as u32,
        pruned: result.pruned,
    })
}
