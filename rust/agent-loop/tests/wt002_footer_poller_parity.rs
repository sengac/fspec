//! WT-002 — Footer poller never runs in the `fspec` binary.
//!
//! Feature: spec/features/fspec-binary-footer-poller.feature
//!
//! Scenarios 1-4 are behavioural: a `SessionManager` configured with
//! [`FspecAgentHooks`] (the fspec binary's hook impl, installed by
//! `build_service`) must spawn the NAPI-free footer poller, which emits
//! `StreamChunk::FooterStateUpdate` on THAT manager's `chunks_tx`
//! broadcast. Scenario 5 is source-shape: the NAPI hook path delegates
//! to the same shared poller (no two-front-door drift).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use serial_test::serial;
use tokio::sync::Mutex;
use uuid::Uuid;

use codelet_agent_loop::FspecAgentHooks;
use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::SessionManager;

/// Trimmed offline models.dev catalog (shared with the RPC-385 / PROV-101
/// / WT-001 precedent) so registry validation stays fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises tests that swap process-global state (data dir, HOME, the
/// footer-poller manager-sender slot) so parallel tests cannot observe
/// each other's state.
static ENV_GUARD: Mutex<()> = Mutex::const_new(());

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

/// Create a fresh git repository (temp dir) with the branch explicitly
/// named `main` (deterministic branch name regardless of git config) and
/// one committed file.
fn fresh_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir for git repo");
    let path = dir.path();
    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    std::fs::write(path.join("hello.txt"), "hello\n").expect("write hello.txt");
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "initial commit"]);
    dir
}

/// Process-wide hermetic dirs, created ONCE per test binary:
/// `codelet_common::set_data_directory` is a first-registration-wins
/// OnceLock, so a per-test data dir would leak into later tests whose
/// tempdir is already dropped (manifest writes then fail with ENOENT).
static SHARED_DIR: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();

fn shared_dir() -> &'static std::path::Path {
    SHARED_DIR
        .get_or_init(|| tempfile::TempDir::new().expect("tempdir for data dir"))
        .path()
}

/// Build a manager rooted in the shared hermetic data dir with the
/// offline models.dev fixture pre-seeded. HOME is redirected to a
/// shared temp dir so the session manifest writes stay hermetic and
/// survive all tests in this binary.
fn manager_with_seeded_cache() -> Arc<SessionManager> {
    set_dummy_credentials();
    let cache_dir = shared_dir().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("seed models.json");
    // First call wins (OnceLock) — subsequent tests reuse the same dir.
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(shared_dir().to_path_buf())
        .expect("set the shared data directory");
    let home_dir = shared_dir().join("home");
    std::fs::create_dir_all(&home_dir).expect("create home dir");
    std::env::set_var("HOME", &home_dir);
    SessionManager::new()
}

/// Pull the next `FooterStateUpdate` chunk for `sid` matching `matches`
/// off the broadcast, skipping chunks for other sessions or that do not
/// match (TEST-042: under the deadlock guard).
async fn next_footer_chunk(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
    matches: impl Fn(&StreamChunk) -> bool,
) -> StreamChunk {
    codelet_test_helpers::deadlock_guard("a matching FooterStateUpdate chunk", async {
        loop {
            match rx.recv().await {
                Ok((id, chunk)) => {
                    if id == *sid {
                        if let StreamChunk::FooterStateUpdate { .. } = &chunk {
                            if matches(&chunk) {
                                return chunk;
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    panic!("chunks broadcast closed before a FooterStateUpdate arrived");
                }
            }
        }
    })
    .await
    .unwrap_or_else(|failure| panic!("{failure} for {sid}"))
}

// =============================================================================
// Scenario: A session created under the fspec binary's hooks receives a
// footer state update on the first poll tick
// =============================================================================

// =============================================================================
// Scenario: A session in a git repository shows its branch in the footer
// without any Bash command first
// =============================================================================

// =============================================================================
// Scenario: An isolated worktree session reports a detached git state, not
// a blank non-repo state
// =============================================================================

// =============================================================================
// Scenario: The footer follows the session's effective CWD when the Bash
// tool moves the session to another directory
// =============================================================================

#[path = "wt002_footer_poller_parity/footer_follows_effective_cwd.rs"]
mod footer_follows_effective_cwd;
#[path = "wt002_footer_poller_parity/fspec_binary_session_receives.rs"]
mod fspec_binary_session_receives;
