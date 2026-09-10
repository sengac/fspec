//! WT-005 — Closing an isolated session must terminate its git-session
//! manifest so the worktree becomes prunable (and the close stays a
//! best-effort, non-failing step).
//!
//! Feature: spec/features/closing-an-isolated-session-terminates-its-git-session-manifest-so-its-worktree-becomes-prunable.feature
//!
//! Scenarios drive a real `SessionManager` against a fresh git
//! repository (temp dir, HOME redirected to a temp dir so the
//! `~/.fspec/git-sessions` manifest writes stay hermetic).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, MutexGuard};

use tracing_subscriber::prelude::*;
use serial_test::serial;
use uuid::Uuid;

use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (anthropic/openai/google) — shared
/// with the WT-001 / RPC-385 precedent so registry validation stays
/// fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Set dummy credentials so `ProviderCredentials::detect()` passes offline.
fn set_dummy_credentials() {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    std::env::set_var("GOOGLE_GENERATIVE_AI_API_KEY", "AIza-test-dummy-key");
}

/// Run a `git` CLI command in `cwd` and fail the test if it exits non-zero.
fn run_git(cwd: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Create a fresh git repository (temp dir) with one committed file.
fn fresh_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir for git repo");
    let path = dir.path();
    run_git(path, &["init"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    std::fs::write(path.join("hello.txt"), "hello\n").expect("write hello.txt");
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "initial commit"]);
    dir
}

/// Build a manager rooted in a fresh data dir with the offline models.dev
/// fixture pre-seeded, with HOME redirected to a temp dir so the
/// `~/.fspec/git-sessions` manifest writes from
/// `create_isolated_session_with_id` / `destroy_session` stay hermetic.
fn manager_with_seeded_cache() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    Arc<SessionManager>,
) {
    set_dummy_credentials();
    let data_dir = tempfile::tempdir().expect("tempdir for data dir");
    let home_dir = tempfile::tempdir().expect("tempdir for HOME");
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("seed models.json");
    // RPC-423 precedent: reset the persistence singletons BEFORE pointing
    // the data directory at the fresh temp dir, so SessionStore re-initialises
    // against THIS test's dir (it caches `sessions_dir` at first use).
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data_dir.path().to_path_buf())
        .expect("set data directory");
    std::env::set_var("HOME", home_dir.path());
    let manager = Arc::new(SessionManager::new());
    (data_dir, home_dir, manager)
}

/// Read the `~/.fspec/git-sessions/<id>.json` manifest written by
/// `codelet_git::create_session_manifest`.
fn read_git_manifest(home: &Path, id: &str) -> Option<codelet_git::SessionManifest> {
    let path = home.join(".fspec").join("git-sessions").join(format!("{id}.json"));
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Path of the `~/.fspec/git-sessions/<id>.json` manifest file.
fn git_manifest_path(home: &Path, id: &str) -> PathBuf {
    home.join(".fspec")
        .join("git-sessions")
        .join(format!("{id}.json"))
}

/// Path of the `{data_dir}/sessions/{uuid}.json` persistence manifest
/// (codelet-core SessionStore layout, data dir redirected per-test).
fn persistence_manifest_path(data_dir: &Path, id: &str) -> PathBuf {
    data_dir.join("sessions").join(format!("{id}.json"))
}

/// Wait up to 5s for a filesystem predicate to hold (the persistence
/// manifest write may lag the destroy call).
async fn wait_until_fs<F: Fn() -> bool>(predicate: F, label: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if predicate() {
            return;
        }
        tokio::task::yield_now().await;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("timed out waiting for: {label}");
}

// =============================================================================
// Scenario: Closing an isolated session marks its git-session manifest
// terminated and keeps the worktree recoverable
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closing_an_isolated_session_marks_its_git_session_manifest_terminated_and_keeps_the_worktree_recoverable() {
    // @step Given a git repository with one committed file
    let repo = fresh_git_repo();

    // @step And an isolated session created for that repository
    let (data_dir, home_dir, manager) = manager_with_seeded_cache();
    let id = Uuid::new_v4().to_string();
    let info = manager
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "Isolated test",
        )
        .await
        .expect("isolated session creation must succeed");

    let manifest_before = read_git_manifest(home_dir.path(), &id)
        .expect("git-session manifest must exist after isolation creation");
    assert!(!manifest_before.terminated, "manifest starts un-terminated");

    // @step When I close the session via destroy_session
    // @step Then the close succeeds
    manager
        .destroy_session(&id)
        .expect("destroy_session must succeed");

    // @step And the git-session manifest for the session has terminated true
    let manifest_after = read_git_manifest(home_dir.path(), &id)
        .expect("git-session manifest must survive the close (only the flag flips)");
    assert!(
        manifest_after.terminated,
        "WT-005: closing the session must set terminated=true on the git-session manifest"
    );

    // @step And the worktree directory still exists on disk
    assert!(
        Path::new(&info.worktree_path).exists(),
        "worktree dir must survive the close (recovery case)"
    );

    // @step And the session persistence manifest still exists on disk
    wait_until_fs(
        || persistence_manifest_path(data_dir.path(), &id).exists(),
        "persistence manifest after close",
    )
    .await;
}

// =============================================================================
// Scenario: Closing an isolated session whose worktree was already merged
// is a silent no-op for termination
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closing_an_isolated_session_whose_worktree_was_already_merged_is_a_silent_no_op_for_termination() {
    // @step Given a git repository with one committed file
    let repo = fresh_git_repo();

    // @step And an isolated session created for that repository
    let (_data_dir, home_dir, manager) = manager_with_seeded_cache();
    let id = Uuid::new_v4().to_string();
    let info = manager
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "Isolated test",
        )
        .await
        .expect("isolated session creation must succeed");

    // Make the worktree dirty so the merge has something to apply.
    let wt = Path::new(&info.worktree_path);
    std::fs::write(wt.join("hello.txt"), "hello (session edit)\n")
        .expect("write session edit");
    run_git(wt, &["add", "."]);
    run_git(wt, &["commit", "-m", "session change"]);

    // @step And the session worktree was merged and its git-session manifest deleted
    codelet_git::merge_session(repo.path(), &id)
        .expect("merge_session must succeed");
    assert!(
        !git_manifest_path(home_dir.path(), &id).exists(),
        "merge_session must delete the git-session manifest"
    );

    // @step When I close the session via destroy_session
    // @step Then the close succeeds
    manager
        .destroy_session(&id)
        .expect("destroy_session must succeed");

    // @step And no git-session manifest is created or left behind for the session
    assert!(
        !git_manifest_path(home_dir.path(), &id).exists(),
        "terminate-on-close must not recreate the git-session manifest"
    );
}

// =============================================================================
// Scenario: Closing a non-isolated session never touches git-session
// manifests
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closing_a_non_isolated_session_never_touches_git_session_manifests() {
    // @step Given a SessionManager with a non-isolated session
    let (_data_dir, home_dir, manager) = manager_with_seeded_cache();
    let id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            std::env::temp_dir().to_str().expect("temp dir is utf8"),
            "Plain test",
        )
        .await
        .expect("non-isolated session creation must succeed");
    assert!(
        !git_manifest_path(home_dir.path(), &id).exists(),
        "precondition: non-isolated sessions have no git-session manifest"
    );

    // @step When I close the session via destroy_session
    // @step Then the close succeeds
    manager
        .destroy_session(&id)
        .expect("destroy_session must succeed");

    // @step And no git-session manifest exists for the session
    assert!(
        !git_manifest_path(home_dir.path(), &id).exists(),
        "non-isolated close must not create a git-session manifest"
    );
}

// =============================================================================
// Scenario: A terminate failure does not fail the close
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_terminate_failure_does_not_fail_the_close() {
    // @step Given an isolated session whose git-session manifest cannot be updated
    // Deterministic way to force `terminate_session` to fail: replace
    // `~/.fspec/git-sessions/<id>.json` (a file) with a DIRECTORY of the
    // same name — `read_manifest` then hits EISDIR on read and the
    // terminate path returns Err instead of the silent no-op.
    let repo = fresh_git_repo();
    let (_data_dir, home_dir, manager) = manager_with_seeded_cache();
    let id = Uuid::new_v4().to_string();
    let _info = manager
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "Isolated test",
        )
        .await
        .expect("isolated session creation must succeed");
    assert!(
        git_manifest_path(home_dir.path(), &id).exists(),
        "precondition: git-session manifest exists"
    );

    let manifest_path = git_manifest_path(home_dir.path(), &id);
    std::fs::remove_file(&manifest_path).expect("remove manifest file");
    std::fs::create_dir_all(&manifest_path).expect("create directory at manifest path");
    assert!(
        manifest_path.is_dir(),
        "git-session manifest path must now be a directory (reads through it fail)"
    );

    // @step When I close the session via destroy_session
    // @step Then the close succeeds
    // @step And the failure is logged as a warning
    LOG_BUFFER.lock().expect("log buffer").clear();
    let layer = tracing_subscriber::fmt::Layer::new()
        .with_writer(LogCapture)
        .with_ansi(false)
        .with_target(true)
        .with_level(true);
    let subscriber = tracing_subscriber::registry().with(layer);
    let close_result = tracing::subscriber::with_default(subscriber, || manager.destroy_session(&id));
    assert!(close_result.is_ok(), "the close must succeed: {:?}", close_result.err());

    let logs = String::from_utf8_lossy(&LOG_BUFFER.lock().expect("log buffer")).to_string();
    let warn_logged = logs.contains("WARN")
        && logs.contains("terminate")
        && logs.contains("codelet_sessions");
    assert!(
        warn_logged,
        "a WARN-level log naming the terminate failure must be emitted; got:\n{logs}"
    );
}

// =============================================================================
// Tracing capture for the warn-assertion above
// =============================================================================

static LOG_BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[derive(Clone, Debug, Default)]
struct LogCapture;

/// `io::Write` adapter over the `LOG_BUFFER` guard — tracing-subscriber's
/// `MakeWriter` bound requires an actual `Write` impl, which
/// `MutexGuard<Vec<u8>>` itself does not provide.
struct LogWriter(MutexGuard<'static, Vec<u8>>);

impl io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogCapture {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter(LOG_BUFFER.lock().expect("log buffer"))
    }
}


