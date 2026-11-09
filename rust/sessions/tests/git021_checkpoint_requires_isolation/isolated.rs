//! COV-057: isolated-session checkpoint scenarios — real
//! `BackgroundSession::checkpoint` / `restore` / `list_checkpoints` on a
//! worktree-backed session created by `create_isolated_session_with_id`.

use super::support::{checkpoint_ref, git, worktree, IsolatedEnv};
use super::*;

// ============================================================================
// Scenario: Checkpoint creates ghost commit with session ID namespace
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_creates_ghost_commit_with_session_id_namespace() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    let env = IsolatedEnv::new()?;
    // @step Given an isolated session with worktree path
    let session = env.isolated_session().await?;
    // @step When I call checkpoint with label "before-refactor"
    let checkpoint = session.checkpoint("before-refactor")?;
    // @step Then a ghost commit should be created
    assert_eq!(
        checkpoint.sha.len(),
        40,
        "ghost commit sha: {}",
        checkpoint.sha
    );
    let kind = git(&worktree(&session)?, &["cat-file", "-t", &checkpoint.sha])?;
    assert_eq!(kind, "commit");
    // @step And the ref should be stored at refs/fspec-checkpoints/<session-id>/before-refactor
    assert_eq!(checkpoint_ref(&session, "before-refactor")?, checkpoint.sha);
    Ok(())
}

// ============================================================================
// Scenario: Checkpoint captures all worktree changes
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn checkpoint_captures_all_worktree_changes() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    let env = IsolatedEnv::new()?;
    // @step Given an isolated session with worktree path
    let session = env.isolated_session().await?;
    let tree = worktree(&session)?;
    // @step And there are staged files in the worktree
    std::fs::write(tree.join("staged.txt"), "staged content\n")?;
    git(&tree, &["add", "staged.txt"])?;
    // @step And there are unstaged modifications in the worktree
    std::fs::write(tree.join("README.md"), "modified readme\n")?;
    // @step And there are untracked files in the worktree
    std::fs::write(tree.join("untracked.txt"), "untracked content\n")?;
    // @step When I call checkpoint with label "full-state"
    let checkpoint = session.checkpoint("full-state")?;
    // @step Then all file states should be captured in the ghost commit
    for (file, content) in [
        ("staged.txt", "staged content"),
        ("README.md", "modified readme"),
        ("untracked.txt", "untracked content"),
    ] {
        assert!(
            checkpoint.files.iter().any(|captured| captured == file),
            "{file} missing from checkpoint files {:?}",
            checkpoint.files
        );
        let object = format!("{}:{file}", checkpoint.sha);
        assert_eq!(git(&tree, &["show", &object])?, content);
    }
    Ok(())
}

// ============================================================================
// Scenario: Restore checkpoint returns worktree to checkpoint state
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restore_checkpoint_returns_worktree_to_checkpoint_state() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    let env = IsolatedEnv::new()?;
    // @step Given an isolated session with worktree path
    let session = env.isolated_session().await?;
    let tree = worktree(&session)?;
    // @step And I have created a checkpoint named "before-refactor"
    std::fs::write(tree.join("kept.txt"), "kept at checkpoint\n")?;
    session.checkpoint("before-refactor")?;
    // @step And I have modified files after the checkpoint
    std::fs::write(tree.join("README.md"), "refactored readme\n")?;
    std::fs::write(tree.join("kept.txt"), "changed after checkpoint\n")?;
    std::fs::write(tree.join("added.txt"), "added after checkpoint\n")?;
    // @step When I call restore with label "before-refactor"
    let restored = session.restore("before-refactor")?;
    assert!(restored.success);
    // @step Then the worktree files should match the checkpoint state
    assert_eq!(
        std::fs::read_to_string(tree.join("README.md"))?,
        "original readme\n"
    );
    assert_eq!(
        std::fs::read_to_string(tree.join("kept.txt"))?,
        "kept at checkpoint\n"
    );
    // @step And files added after checkpoint should be deleted
    assert!(!tree.join("added.txt").exists());
    assert!(
        restored
            .deleted_files
            .iter()
            .any(|file| file == "added.txt"),
        "deleted files: {:?}",
        restored.deleted_files
    );
    Ok(())
}

// ============================================================================
// Scenario: Parallel sessions have independent checkpoint namespaces
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parallel_sessions_have_independent_checkpoint_namespaces() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    let env = IsolatedEnv::new()?;
    // @step Given two isolated sessions with different IDs
    let session_a = env.isolated_session().await?;
    let session_b = env.isolated_session().await?;
    assert_ne!(session_a.id, session_b.id);
    // @step And both sessions create checkpoint named "baseline"
    std::fs::write(worktree(&session_a)?.join("a.txt"), "session A work\n")?;
    let checkpoint_a = session_a.checkpoint("baseline")?;
    std::fs::write(worktree(&session_b)?.join("b.txt"), "session B work\n")?;
    let checkpoint_b = session_b.checkpoint("baseline")?;
    // @step Then each checkpoint should be stored under its own session ID
    assert_eq!(checkpoint_ref(&session_a, "baseline")?, checkpoint_a.sha);
    assert_eq!(checkpoint_ref(&session_b, "baseline")?, checkpoint_b.sha);
    assert_ne!(checkpoint_a.sha, checkpoint_b.sha);
    // @step And session A should not see session B checkpoints
    assert_eq!(session_a.list_checkpoints()?, vec!["baseline".to_string()]);
    assert_eq!(session_b.list_checkpoints()?, vec!["baseline".to_string()]);
    session_a.restore("baseline")?;
    let tree_a = worktree(&session_a)?;
    assert!(tree_a.join("a.txt").exists());
    assert!(!tree_a.join("b.txt").exists());
    Ok(())
}

// ============================================================================
// Scenario: List checkpoints returns all checkpoint labels
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_checkpoints_returns_all_checkpoint_labels() -> TestResult {
    let _guard = DATA_DIR_GUARD.lock().await;
    let env = IsolatedEnv::new()?;
    // @step Given an isolated session with worktree path
    let session = env.isolated_session().await?;
    // @step And I have created checkpoints named "first", "second", "third"
    for label in ["first", "second", "third"] {
        session.checkpoint(label)?;
    }
    // @step When I call list_checkpoints
    let mut labels = session.list_checkpoints()?;
    // @step Then all three checkpoint labels should be returned
    labels.sort();
    assert_eq!(labels, vec!["first", "second", "third"]);
    Ok(())
}
