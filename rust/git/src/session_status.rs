//! Session status derivation and manifest management
//!
//! GIT-022: Provides status derivation at query time and session manifest storage.
//! Session status is derived dynamically based on:
//! 1. BackgroundSession active map (Active)
//! 2. Session manifest state (Orphaned if terminated/missing)
//! 3. Worktree change detection (PendingMerge vs Clean)

use crate::error::{GitError, Result};
use crate::session_result::get_session_diff;
use crate::worktree::{list_worktrees, FSPEC_WORKTREES_DIR};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Session status derived at query time
///
/// Status is NOT stored - it's computed based on:
/// - Whether session is in BackgroundSession's active map
/// - Whether session manifest exists and is not terminated
/// - Whether worktree has uncommitted changes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedSessionStatus {
    /// Session is currently active (in BackgroundSession's active map)
    Active,
    /// Worktree exists, not active, HAS uncommitted changes - ready for merge
    PendingMerge,
    /// Worktree exists, not active, NO uncommitted changes
    Clean,
    /// Worktree exists but no valid session record (manifest missing or terminated)
    Orphaned,
}

impl std::fmt::Display for DerivedSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DerivedSessionStatus::Active => write!(f, "Active"),
            DerivedSessionStatus::PendingMerge => write!(f, "PendingMerge"),
            DerivedSessionStatus::Clean => write!(f, "Clean"),
            DerivedSessionStatus::Orphaned => write!(f, "Orphaned"),
        }
    }
}

/// Session manifest stored at ~/.fspec/git-sessions/<session-id>.json
///
/// Tracks session metadata for:
/// - Orphan detection (terminated sessions)
/// - Session history
/// - Completion timestamps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManifest {
    /// Unique session identifier
    pub session_id: String,
    /// Project root where session was created
    pub project_root: PathBuf,
    /// Path to the session worktree (if isolated)
    pub worktree_path: Option<PathBuf>,
    /// Base commit SHA the session was created from
    pub base_commit: Option<String>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was completed (None if still active or terminated)
    pub completed_at: Option<DateTime<Utc>>,
    /// Whether the session was terminated abnormally (orphaned)
    #[serde(default)]
    pub terminated: bool,
}

impl SessionManifest {
    /// Create a new session manifest
    pub fn new(
        session_id: String,
        project_root: PathBuf,
        worktree_path: Option<PathBuf>,
        base_commit: Option<String>,
    ) -> Self {
        Self {
            session_id,
            project_root,
            worktree_path,
            base_commit,
            created_at: Utc::now(),
            completed_at: None,
            terminated: false,
        }
    }

    /// Mark the session as completed
    pub fn mark_completed(&mut self) {
        self.completed_at = Some(Utc::now());
    }

    /// Mark the session as terminated (orphaned)
    pub fn mark_terminated(&mut self) {
        self.terminated = true;
    }
}

/// Directory path for git session manifests: ~/.fspec/git-sessions/
///
/// NOTE: This uses a separate directory from the persistence module's
/// ~/.fspec/sessions/ to avoid schema conflicts. The git module's
/// SessionManifest has a different schema (session_id: String) than
/// the persistence module's SessionManifest (id: Uuid).
pub fn get_sessions_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".fspec").join("git-sessions"))
}

/// Get the manifest file path for a session
pub fn get_manifest_path(session_id: &str) -> Option<PathBuf> {
    get_sessions_dir().map(|dir| dir.join(format!("{session_id}.json")))
}

/// Read a session manifest from disk
pub fn read_manifest(session_id: &str) -> Result<Option<SessionManifest>> {
    let Some(path) = get_manifest_path(session_id) else {
        return Ok(None);
    };

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)?;
    let manifest: SessionManifest = serde_json::from_str(&content)
        .map_err(|e| GitError::Other(format!("Invalid manifest JSON: {e}")))?;

    Ok(Some(manifest))
}

/// Write a session manifest to disk
pub fn write_manifest(manifest: &SessionManifest) -> Result<()> {
    let Some(path) = get_manifest_path(&manifest.session_id) else {
        return Err(GitError::Other(
            "Could not determine manifest path (home dir not found)".to_string(),
        ));
    };

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(manifest)
        .map_err(|e| GitError::Other(format!("Failed to serialize manifest: {e}")))?;

    fs::write(&path, content)?;
    Ok(())
}

/// Delete a session manifest from disk
pub fn delete_manifest(session_id: &str) -> Result<()> {
    let Some(path) = get_manifest_path(session_id) else {
        return Ok(()); // No home dir, nothing to delete
    };

    if path.exists() {
        fs::remove_file(&path)?;
    }

    Ok(())
}

/// Derive the session status at query time
///
/// Status derivation priority:
/// 1. If session is in active_sessions set → Active
/// 2. If worktree doesn't exist → Error (WorktreeNotFound)
/// 3. If manifest missing or terminated → Orphaned
/// 4. If worktree has changes → PendingMerge
/// 5. Otherwise → Clean
///
/// # Arguments
/// * `repo_path` - Path to the git repository
/// * `session_id` - Session identifier
/// * `active_sessions` - Set of currently active session IDs (from BackgroundSession)
///
/// # Returns
/// DerivedSessionStatus based on current state
pub fn derive_session_status(
    repo_path: impl AsRef<Path>,
    session_id: &str,
    active_sessions: &HashSet<String>,
) -> Result<DerivedSessionStatus> {
    let repo_path = repo_path.as_ref();

    // 1. Check if session is active first (highest priority)
    if active_sessions.contains(session_id) {
        return Ok(DerivedSessionStatus::Active);
    }

    // 2. Check if worktree exists
    let worktrees = list_worktrees(repo_path)?;
    let worktree_exists = worktrees.iter().any(|w| w.session_id == session_id);

    if !worktree_exists {
        // Also check if the worktree directory physically exists
        let worktree_path = repo_path.join(FSPEC_WORKTREES_DIR).join(session_id);
        if !worktree_path.exists() {
            return Err(GitError::WorktreeNotFound {
                session_id: session_id.to_string(),
            });
        }
    }

    // 3. Check session manifest
    let manifest = read_manifest(session_id)?;

    match manifest {
        None => {
            // No manifest = orphaned
            Ok(DerivedSessionStatus::Orphaned)
        }
        Some(m) if m.terminated => {
            // Terminated session = orphaned
            Ok(DerivedSessionStatus::Orphaned)
        }
        Some(_) => {
            // Manifest exists and not terminated - check for changes
            // 4. Check worktree for changes
            let diff = get_session_diff(repo_path, session_id)?;

            if diff.files_changed.is_empty()
                && diff.files_added.is_empty()
                && diff.files_deleted.is_empty()
            {
                Ok(DerivedSessionStatus::Clean)
            } else {
                Ok(DerivedSessionStatus::PendingMerge)
            }
        }
    }
}

/// Complete a session by updating its manifest
///
/// This does NOT cleanup the worktree - it leaves it for user review.
/// The worktree is only cleaned up when the user explicitly:
/// - Merges the session (apply_session_changes)
/// - Discards the session (abort_session)
///
/// # Arguments
/// * `session_id` - Session identifier
///
/// # Returns
/// Ok(()) if manifest was updated successfully
pub fn complete_session(session_id: &str) -> Result<()> {
    let manifest = read_manifest(session_id)?;

    match manifest {
        Some(mut m) => {
            m.mark_completed();
            write_manifest(&m)?;
            Ok(())
        }
        None => Err(GitError::Other(format!(
            "Cannot complete session '{session_id}': manifest not found"
        ))),
    }
}

/// Create a session manifest when starting a new session
///
/// Should be called when creating an isolated session.
///
/// # Arguments
/// * `session_id` - Session identifier
/// * `project_root` - Project root directory
/// * `worktree_path` - Path to the worktree (None for non-isolated)
/// * `base_commit` - Base commit SHA
pub fn create_session_manifest(
    session_id: &str,
    project_root: impl AsRef<Path>,
    worktree_path: Option<PathBuf>,
    base_commit: Option<String>,
) -> Result<SessionManifest> {
    let manifest = SessionManifest::new(
        session_id.to_string(),
        project_root.as_ref().to_path_buf(),
        worktree_path,
        base_commit,
    );

    write_manifest(&manifest)?;
    Ok(manifest)
}

/// Terminate a session (mark as orphaned)
///
/// Used when a session is abnormally terminated (e.g., process crash).
///
/// # Arguments
/// * `session_id` - Session identifier
pub fn terminate_session(session_id: &str) -> Result<()> {
    let manifest = read_manifest(session_id)?;

    match manifest {
        Some(mut m) => {
            m.mark_terminated();
            write_manifest(&m)?;
            Ok(())
        }
        None => {
            // No manifest exists - nothing to terminate
            Ok(())
        }
    }
}

// =============================================================================
// GIT-023: Session List and Inspect Operations
// =============================================================================

/// Filter for listing sessions
///
/// Used to filter the list of sessions returned by `list_sessions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionFilter {
    /// Return all sessions
    All,
    /// Only active sessions
    Active,
    /// Only pending merge sessions (have uncommitted changes)
    PendingMerge,
    /// Only clean sessions (no uncommitted changes)
    Clean,
    /// Only orphaned sessions (no valid manifest)
    Orphaned,
}

/// Information about a session for listing
///
/// Contains all information needed to display session status in a list.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Session ID
    pub session_id: String,
    /// Derived status (Active, PendingMerge, Clean, Orphaned)
    pub status: DerivedSessionStatus,
    /// Base commit the worktree was created from
    pub base_commit: String,
    /// Number of files changed (modified + added + deleted)
    pub files_changed: usize,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// Path to the worktree
    pub worktree_path: PathBuf,
}

/// List all sessions with derived status
///
/// Returns information about all session worktrees, optionally filtered by status.
///
/// # Arguments
/// * `repo_path` - Path to the git repository
/// * `active_sessions` - Set of currently active session IDs (from BackgroundSession)
/// * `filter` - Filter to apply to the results
///
/// # Returns
/// Vector of SessionInfo for sessions matching the filter
pub fn list_sessions(
    repo_path: impl AsRef<Path>,
    active_sessions: &HashSet<String>,
    filter: SessionFilter,
) -> Result<Vec<SessionInfo>> {
    let repo_path = repo_path.as_ref();
    let worktrees = list_worktrees(repo_path)?;

    let mut sessions = Vec::new();
    for worktree in worktrees {
        // Derive status for this session
        let status = match derive_session_status(repo_path, &worktree.session_id, active_sessions) {
            Ok(s) => s,
            Err(_) => continue, // Skip sessions that can't be status-derived
        };

        // Apply filter
        if !matches_filter(&status, &filter) {
            continue;
        }

        // Get change count
        let files_changed = match get_session_diff(repo_path, &worktree.session_id) {
            Ok(diff) => {
                diff.files_changed.len() + diff.files_added.len() + diff.files_deleted.len()
            }
            Err(_) => 0,
        };

        // Get created_at from manifest, or use current time as fallback
        let created_at = read_manifest(&worktree.session_id)
            .ok()
            .flatten()
            .map(|m| m.created_at)
            .unwrap_or_else(Utc::now);

        sessions.push(SessionInfo {
            session_id: worktree.session_id,
            status,
            base_commit: worktree.head_commit,
            files_changed,
            created_at,
            worktree_path: worktree.path,
        });
    }

    Ok(sessions)
}

/// Check if a session status matches a filter
fn matches_filter(status: &DerivedSessionStatus, filter: &SessionFilter) -> bool {
    match filter {
        SessionFilter::All => true,
        SessionFilter::Active => *status == DerivedSessionStatus::Active,
        SessionFilter::PendingMerge => *status == DerivedSessionStatus::PendingMerge,
        SessionFilter::Clean => *status == DerivedSessionStatus::Clean,
        SessionFilter::Orphaned => *status == DerivedSessionStatus::Orphaned,
    }
}

/// Inspect session diff without any side effects
///
/// This returns the diff information for a session without modifying
/// the worktree or any session state. Use this to preview changes
/// before deciding to merge or discard.
///
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `session_id` - Session identifier
///
/// # Returns
/// SessionResult with diff information, or WorktreeNotFound error
pub fn inspect_session(
    repo_path: impl AsRef<Path>,
    session_id: &str,
) -> Result<crate::session_result::SessionResult> {
    // Simply delegate to get_session_diff - it's already read-only
    get_session_diff(repo_path, session_id)
}

// =============================================================================
// GIT-024: Session Merge Operations
// =============================================================================

/// Result of merging a session to main worktree
///
/// Contains information about what files were changed during the merge.
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Session ID that was merged
    pub session_id: String,
    /// Files that were modified in main
    pub files_modified: Vec<String>,
    /// Files that were added to main
    pub files_added: Vec<String>,
    /// Files that were deleted from main
    pub files_deleted: Vec<String>,
}

/// Merge session changes to main worktree
///
/// Applies all changes from session to main and removes worktree on success.
/// Returns conflict error if main has diverged since session base commit.
///
/// # Algorithm
/// 1. Get session diff to know what changed (for return value)
/// 2. Call apply_session_changes() which:
///    - Detects conflicts with main worktree
///    - Copies modified/added files to main
///    - Deletes removed files from main
///    - Removes worktree on success
/// 3. Delete session manifest
/// 4. Return MergeResult with file lists
///
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `session_id` - Session identifier
///
/// # Returns
/// MergeResult on success, or error if:
/// - WorktreeNotFound: Session doesn't exist
/// - ConflictError: Main worktree has conflicting changes
///
/// # Example
/// ```ignore
/// match merge_session(repo_path, session_id) {
///     Ok(result) => {
///         println!("Merged {} files",
///             result.files_modified.len() +
///             result.files_added.len() +
///             result.files_deleted.len()
///         );
///     }
///     Err(GitError::ConflictError { files }) => {
///         eprintln!("Conflict: {:?}", files);
///         // Worktree is still intact - user can resolve and retry
///     }
///     Err(e) => return Err(e),
/// }
/// ```
pub fn merge_session(repo_path: impl AsRef<Path>, session_id: &str) -> Result<MergeResult> {
    use crate::session_result::apply_session_changes;

    let repo_path = repo_path.as_ref();

    // 1. Get diff first (to capture what will change)
    let diff = get_session_diff(repo_path, session_id)?;

    // 2. Apply changes (this handles conflicts and cleanup of worktree)
    apply_session_changes(repo_path, session_id)?;

    // 3. Delete manifest (cleanup session state)
    delete_manifest(session_id)?;

    // 4. Return what was merged
    Ok(MergeResult {
        session_id: session_id.to_string(),
        files_modified: diff.files_changed,
        files_added: diff.files_added,
        files_deleted: diff.files_deleted,
    })
}

// =============================================================================
// GIT-025: Session Discard Operations
// =============================================================================

/// Result of discarding a session
///
/// Contains information about what was discarded (not applied).
#[derive(Debug, Clone)]
pub struct DiscardResult {
    /// Session ID that was discarded
    pub session_id: String,
    /// Number of files that were in the session (not applied)
    pub files_discarded: usize,
    /// Status the session had before discard
    pub previous_status: DerivedSessionStatus,
}

/// Discard session without applying any changes
///
/// Removes the worktree and cleans up git metadata without
/// applying any of the session's changes to the main worktree.
///
/// # Algorithm
/// 1. Get session diff to count files (for return value)
/// 2. Derive session status (for return value)
/// 3. Call abort_session() to remove the worktree
/// 4. Delete session manifest
/// 5. Return DiscardResult
///
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `session_id` - Session identifier
///
/// # Returns
/// DiscardResult on success, or error if:
/// - WorktreeNotFound: Session doesn't exist
///
/// # Example
/// ```ignore
/// match discard_session(repo_path, session_id) {
///     Ok(result) => {
///         println!("Discarded {} files", result.files_discarded);
///         println!("Previous status was: {:?}", result.previous_status);
///     }
///     Err(GitError::WorktreeNotFound { .. }) => {
///         eprintln!("Session not found");
///     }
///     Err(e) => return Err(e),
/// }
/// ```
pub fn discard_session(repo_path: impl AsRef<Path>, session_id: &str) -> Result<DiscardResult> {
    use crate::session_result::abort_session;

    let repo_path = repo_path.as_ref();

    // 1. Get diff first (to count files that won't be applied)
    let diff = get_session_diff(repo_path, session_id)?;
    let files_discarded =
        diff.files_changed.len() + diff.files_added.len() + diff.files_deleted.len();

    // 2. Get status before discard (for informational purposes)
    // Note: Not active if we're discarding, so pass empty set
    let previous_status = derive_session_status(repo_path, session_id, &HashSet::new())?;

    // 3. Remove worktree (abort_session is alias for remove_worktree)
    abort_session(repo_path, session_id)?;

    // 4. Delete manifest (cleanup session state - ignore errors if manifest doesn't exist)
    let _ = delete_manifest(session_id);

    // 5. Return what was discarded
    Ok(DiscardResult {
        session_id: session_id.to_string(),
        files_discarded,
        previous_status,
    })
}

// =============================================================================
// GIT-026: Orphan Detection and Pruning Operations
// =============================================================================

/// Result of pruning orphaned worktrees
///
/// Contains information about how many worktrees were pruned and their IDs.
#[derive(Debug, Clone)]
pub struct PruneResult {
    /// Number of orphaned worktrees that were pruned
    pub count: usize,
    /// List of session IDs that were pruned
    pub pruned: Vec<String>,
}

/// Check if a session is orphaned
///
/// A session is orphaned if:
/// 1. NOT in the active sessions set
/// 2. AND (manifest doesn't exist OR manifest.terminated == true)
///
/// Active sessions are NEVER orphaned, regardless of manifest state.
/// This is a simplified check compared to `derive_session_status()` -
/// it only checks for orphan state without needing repo_path for diff detection.
///
/// # Arguments
/// * `session_id` - Session identifier
/// * `active_sessions` - Set of currently active session IDs (from BackgroundSession)
///
/// # Returns
/// Ok(true) if session is orphaned, Ok(false) otherwise
///
/// # Example
/// ```ignore
/// let active: HashSet<String> = HashSet::new();
/// if is_orphaned("session-123", &active)? {
///     println!("Session is orphaned and can be pruned");
/// }
/// ```
pub fn is_orphaned(session_id: &str, active_sessions: &HashSet<String>) -> Result<bool> {
    // Active sessions are NEVER orphaned
    if active_sessions.contains(session_id) {
        return Ok(false);
    }

    // Check manifest
    let manifest = read_manifest(session_id)?;

    match manifest {
        None => {
            // No manifest = orphaned
            Ok(true)
        }
        Some(m) if m.terminated => {
            // Terminated session = orphaned
            Ok(true)
        }
        Some(_) => {
            // Has valid manifest, not terminated = not orphaned
            Ok(false)
        }
    }
}

/// Prune all orphaned worktrees
///
/// Removes worktrees that have no valid session record.
/// Active sessions are never pruned.
///
/// For each orphaned session:
/// 1. Removes the worktree directory (.fspec/worktrees/<session-id>/)
/// 2. Removes the session manifest (~/.fspec/git-sessions/<session-id>.json)
///
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `active_sessions` - Set of currently active session IDs
///
/// # Returns
/// PruneResult with count and list of pruned session IDs
///
/// # Example
/// ```ignore
/// let active: HashSet<String> = get_active_sessions();
/// let result = prune_orphaned(repo_path, &active)?;
/// println!("Pruned {} orphaned worktrees:", result.count);
/// for id in &result.pruned {
///     println!("  - {}", id);
/// }
/// ```
pub fn prune_orphaned(
    repo_path: impl AsRef<Path>,
    active_sessions: &HashSet<String>,
) -> Result<PruneResult> {
    use crate::worktree::{list_worktrees, remove_worktree};

    let repo_path = repo_path.as_ref();
    let worktrees = list_worktrees(repo_path)?;
    let mut pruned = Vec::new();

    for worktree in worktrees {
        if is_orphaned(&worktree.session_id, active_sessions)? {
            // Remove worktree directory and git metadata
            remove_worktree(repo_path, &worktree.session_id)?;

            // Remove manifest if it exists (ignore errors if it doesn't)
            let _ = delete_manifest(&worktree.session_id);

            pruned.push(worktree.session_id);
        }
    }

    Ok(PruneResult {
        count: pruned.len(),
        pruned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derived_session_status_display() {
        assert_eq!(DerivedSessionStatus::Active.to_string(), "Active");
        assert_eq!(
            DerivedSessionStatus::PendingMerge.to_string(),
            "PendingMerge"
        );
        assert_eq!(DerivedSessionStatus::Clean.to_string(), "Clean");
        assert_eq!(DerivedSessionStatus::Orphaned.to_string(), "Orphaned");
    }

    #[test]
    fn test_session_manifest_new() {
        let manifest = SessionManifest::new(
            "test-session".to_string(),
            PathBuf::from("/project"),
            Some(PathBuf::from("/project/.fspec/worktrees/test-session")),
            Some("abc123".to_string()),
        );

        assert_eq!(manifest.session_id, "test-session");
        assert_eq!(manifest.project_root, PathBuf::from("/project"));
        assert!(manifest.worktree_path.is_some());
        assert!(manifest.base_commit.is_some());
        assert!(manifest.completed_at.is_none());
        assert!(!manifest.terminated);
    }

    #[test]
    fn test_session_manifest_mark_completed() {
        let mut manifest =
            SessionManifest::new("test".to_string(), PathBuf::from("/project"), None, None);

        assert!(manifest.completed_at.is_none());
        manifest.mark_completed();
        assert!(manifest.completed_at.is_some());
    }

    #[test]
    fn test_session_manifest_mark_terminated() {
        let mut manifest =
            SessionManifest::new("test".to_string(), PathBuf::from("/project"), None, None);

        assert!(!manifest.terminated);
        manifest.mark_terminated();
        assert!(manifest.terminated);
    }

    #[test]
    fn test_get_sessions_dir() {
        // This should return Some on most systems with a home directory
        let dir = get_sessions_dir();
        // We can't assert it's Some because CI might not have HOME set
        if let Some(d) = dir {
            assert!(
                d.ends_with(".fspec/git-sessions") || d.to_string_lossy().contains("git-sessions")
            );
        }
    }

    #[test]
    fn test_get_manifest_path() {
        let path = get_manifest_path("test-session");
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("test-session.json"));
        }
    }
}
