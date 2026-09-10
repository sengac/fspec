//! WT-007 — re-isolation resolution for the `/resume` path
//! (`SessionManager::create_session_from_manifest`).
//!
//! The codelet-git session manifest (`~/.fspec/git-sessions/<id>.json`)
//! survives a restart but used to be ignored on resume — a
//! formerly-isolated session would silently reattach to the main
//! project root. This module decides whether a resumed session should
//! be re-isolated and carries the isolation surface the live
//! `create_isolated_session_with_id` path gets:
//!
//! - `worktree_path` + `base_commit` for the BackgroundSession,
//! - the `IsolationContext` injected into environment reminders,
//! - (at the call site) the `IsolationStateChange(true, …)` chunk and
//!   the worktree-cwd footer poller.
//!
//! When the manifest claims isolation but the worktree (or its git
//! admin dir `<repo>/.git/worktrees/<id>`) is gone, the manifest is
//! marked `terminated` — WT-005's `/worktrees` prune hook — and the
//! session resumes non-isolated with a warning.

use std::path::Path;

use uuid::Uuid;

use codelet_cli::session::context_gathering::IsolationContext;

/// The isolation surface recovered for a resumed formerly-isolated
/// session. All fields `None` = resume non-isolated (the historical
/// behavior for never-isolated sessions).
#[derive(Debug, Clone, Default)]
pub struct ResumeIsolation {
    /// Absolute worktree path to attach to the BackgroundSession.
    pub worktree_path: Option<std::path::PathBuf>,
    /// Base commit SHA the worktree was forked from.
    pub base_commit: Option<String>,
    /// Isolation context for environment-reminder injection (relative
    /// worktree path, exactly as the live create path builds it).
    pub context: Option<IsolationContext>,
}

/// Decide whether a resumed session should be re-isolated.
///
/// Reads the codelet-git session manifest for `uuid`. Returns the
/// re-isolation surface when the manifest claims a `worktree_path` AND
/// both the worktree dir and the git admin dir
/// (`<project>/.git/worktrees/<id>`) still exist on disk.
pub fn resolve(uuid: &Uuid, project: &Path) -> ResumeIsolation {
    let id = uuid.to_string();
    let Ok(Some(git_manifest)) = codelet_git::read_manifest(&id) else {
        // No git-session manifest: the session was never isolated.
        return ResumeIsolation::default();
    };
    let Some(worktree_path) = git_manifest.worktree_path.clone() else {
        // Manifest without a worktree path: never isolated.
        return ResumeIsolation::default();
    };
    let admin_dir = project.join(".git").join("worktrees").join(&id);
    if worktree_path.exists() && admin_dir.exists() {
        // Prefer the manifest's recorded base commit; fall back to the
        // worktree's own HEAD file (the same source
        // `get_session_diff` uses) if the manifest lacks one.
        let base_commit = git_manifest
            .base_commit
            .or_else(|| read_worktree_head(&admin_dir));
        let relative = worktree_path
            .strip_prefix(project)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| worktree_path.to_string_lossy().to_string());
        let context = IsolationContext {
            is_isolated: true,
            worktree_path: Some(relative),
            base_commit: base_commit.clone(),
        };
        tracing::info!(
            session_id = %id,
            worktree = %worktree_path.to_string_lossy(),
            "WT-007: re-isolating resumed session in its surviving worktree"
        );
        return ResumeIsolation {
            worktree_path: Some(worktree_path),
            base_commit,
            context: Some(context),
        };
    }

    // The manifest claims isolation but the worktree is gone — resume
    // non-isolated and mark the strayed manifest terminated so
    // /worktrees prune (WT-005) can reclaim it.
    tracing::warn!(
        session_id = %id,
        worktree = %worktree_path.to_string_lossy(),
        "WT-007: git manifest claims isolation but the worktree is gone — resuming non-isolated and marking the manifest terminated"
    );
    mark_terminated(&id);
    ResumeIsolation::default()
}

/// Read the worktree's HEAD file (`<admin_dir>/HEAD`) — the same source
/// `get_session_diff` reads for the base commit.
fn read_worktree_head(admin_dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(admin_dir.join("HEAD")).ok()?;
    let sha = content.trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

/// Mark the session's git-session manifest terminated (best-effort —
/// the resume itself must never fail because of this).
fn mark_terminated(id: &str) {
    let Ok(Some(mut manifest)) = codelet_git::read_manifest(id) else {
        return;
    };
    if manifest.terminated {
        return;
    }
    manifest.mark_terminated();
    if let Err(e) = codelet_git::write_manifest(&manifest) {
        tracing::warn!(
            session_id = %id,
            "WT-007: failed to mark git manifest terminated: {e}"
        );
    }
}
