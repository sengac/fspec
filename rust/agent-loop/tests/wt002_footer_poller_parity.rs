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
use std::time::Duration;

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
    let _ = codelet_common::set_data_directory(shared_dir().to_path_buf());
    let home_dir = shared_dir().join("home");
    std::fs::create_dir_all(&home_dir).expect("create home dir");
    std::env::set_var("HOME", &home_dir);
    Arc::new(SessionManager::new())
}

/// Pull the next `FooterStateUpdate` chunk for `sid` matching `matches`
/// off the broadcast, skipping chunks for other sessions or that do not
/// match. Times out after `timeout`.
async fn next_footer_chunk(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
    timeout: Duration,
    matches: impl Fn(&StreamChunk) -> bool,
) -> StreamChunk {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
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
    panic!("timed out waiting for a matching FooterStateUpdate chunk for {sid}");
}

// =============================================================================
// Scenario: A session created under the fspec binary's hooks receives a
// footer state update on the first poll tick
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_fspec_binary_session_receives_footer_state_update_on_first_poll_tick() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    // (build_service wiring: hooks + the manager's chunks_tx registered as
    // the footer-poller emission target — the non-singleton manager the
    // binary owns.)
    let manager = manager_with_seeded_cache();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step And a new session has been created in a git repository that has a branch checked out
    let repo = fresh_git_repo();
    let project = repo.path().to_str().expect("repo path is utf8");
    let sid = Uuid::new_v4();
    manager
        .create_session_with_id(&sid.to_string(), "anthropic/claude-opus-4-5", project, "wt002")
        .await
        .expect("session creation under FspecAgentHooks must succeed");

    // @step When the footer poller's first tick runs
    // @step Then a FooterStateUpdate chunk is emitted on that manager's chunks broadcast for the session
    // (the first tick fires immediately — the first_run gate)
    let chunk = next_footer_chunk(
        &mut chunks_rx,
        &SessionId::new(sid.to_string()),
        Duration::from_secs(5),
        |c| matches!(c, StreamChunk::FooterStateUpdate { .. }),
    )
    .await;

    // @step And the chunk carries the session's working directory and the checked-out branch name
    match chunk {
        StreamChunk::FooterStateUpdate {
            cwd,
            display_path: _,
            is_git_repo,
            branch,
        } => {
            assert_eq!(cwd, project, "chunk must carry the session's working directory");
            assert!(is_git_repo, "the fresh repo must be reported as a git repo");
            assert_eq!(branch.as_deref(), Some("main"), "chunk must carry the checked-out branch");
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }
}

// =============================================================================
// Scenario: A session in a git repository shows its branch in the footer
// without any Bash command first
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_session_in_git_repo_shows_branch_in_footer_without_any_bash_command_first() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    let manager = manager_with_seeded_cache();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step And a new session has been created in a git repository that has a branch checked out
    let repo = fresh_git_repo();
    let project = repo.path().to_str().expect("repo path is utf8");
    let sid = Uuid::new_v4();
    manager
        .create_session_with_id(&sid.to_string(), "anthropic/claude-opus-4-5", project, "wt002")
        .await
        .expect("session creation under FspecAgentHooks must succeed");

    // @step When I observe the footer state for that session
    let chunk = next_footer_chunk(
        &mut chunks_rx,
        &SessionId::new(sid.to_string()),
        Duration::from_secs(5),
        |c| matches!(c, StreamChunk::FooterStateUpdate { .. }),
    )
    .await;

    // @step Then the footer state reports the working directory and the branch name
    match chunk {
        StreamChunk::FooterStateUpdate {
            cwd,
            display_path: _,
            is_git_repo,
            branch,
        } => {
            assert_eq!(cwd, project, "footer state must report the working directory");
            assert!(is_git_repo && branch.as_deref() == Some("main"));
            // The poller must have SEEDED the footer-cwd registry at creation
            // time — that is the value the first emission carries without
            // any tool ever running.
            assert_eq!(
                codelet_tools::footer_cwd::get_footer_cwd(sid).as_deref(),
                Some(project),
                "the footer CWD registry must be seeded with the project root at session creation"
            );
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }

    // @step And no Bash tool invocation is required for the branch to appear
    // (this test never invokes any tool — the branch arrived via the
    // creation-time poll alone)
}

// =============================================================================
// Scenario: An isolated worktree session reports a detached git state, not
// a blank non-repo state
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_worktree_session_reports_detached_git_state_not_a_blank_non_repo_state() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    let manager = manager_with_seeded_cache();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step And a new isolated session whose worktree is in a detached HEAD state
    let repo = fresh_git_repo();
    let sid = Uuid::new_v4();
    let info = manager
        .create_isolated_session_with_id(
            &sid.to_string(),
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "wt002-isolated",
        )
        .await
        .expect("isolated session creation under FspecAgentHooks must succeed");
    let worktree_path = info.worktree_path.clone();

    // @step When the footer poller ticks for that session
    let chunk = next_footer_chunk(
        &mut chunks_rx,
        &SessionId::new(sid.to_string()),
        Duration::from_secs(5),
        |c| matches!(c, StreamChunk::FooterStateUpdate { .. }),
    )
    .await;

    // @step Then the emitted footer state has is_git_repo true with an empty (None) branch
    match chunk {
        StreamChunk::FooterStateUpdate {
            cwd,
            display_path: _,
            is_git_repo,
            branch,
        } => {
            assert!(
                is_git_repo,
                "WT-002: a detached-HEAD worktree is still a git repo — is_git_repo must be true (a false value collapses the branch to blank)"
            );
            assert!(
                branch.is_none(),
                "a detached-HEAD worktree has no branch — branch must be None, got {branch:?}"
            );

            // @step And the chunk carries the worktree path as the CWD
            assert_eq!(
                cwd, worktree_path,
                "the footer state must carry the worktree path so the TUI can render its detached indicator"
            );
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }
}

// =============================================================================
// Scenario: The footer follows the session's effective CWD when the Bash
// tool moves the session to another directory
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_footer_follows_effective_cwd_when_bash_moves_the_session_to_another_directory() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    let manager = manager_with_seeded_cache();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
    let mut chunks_rx = manager.chunks_tx().subscribe();

    // @step And a new session has been created whose footer CWD has been seeded to the project root
    let repo = fresh_git_repo();
    let project = repo.path().to_str().expect("repo path is utf8");
    let sub = repo.path().join("src");
    std::fs::create_dir_all(&sub).expect("create subdirectory");
    let sub_str = sub.to_str().expect("subdir path is utf8").to_string();
    let sid = Uuid::new_v4();
    manager
        .create_session_with_id(&sid.to_string(), "anthropic/claude-opus-4-5", project, "wt002")
        .await
        .expect("session creation under FspecAgentHooks must succeed");
    let initial = next_footer_chunk(
        &mut chunks_rx,
        &SessionId::new(sid.to_string()),
        Duration::from_secs(5),
        |c| matches!(c, StreamChunk::FooterStateUpdate { .. }),
    )
    .await;
    match initial {
        StreamChunk::FooterStateUpdate { cwd, .. } => {
            assert_eq!(cwd, project, "first tick must seed the CWD to the project root");
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }

    // @step When the session's footer CWD registry is updated to a subdirectory
    codelet_tools::footer_cwd::update_footer_cwd(sid, sub_str.clone());

    // @step And the footer poller ticks again
    // @step Then the emitted footer state carries the subdirectory as the CWD
    let updated = next_footer_chunk(
        &mut chunks_rx,
        &SessionId::new(sid.to_string()),
        Duration::from_secs(15),
        |c| match c {
            StreamChunk::FooterStateUpdate { cwd, .. } => cwd == &sub_str,
            _ => false,
        },
    )
    .await;
    match updated {
        StreamChunk::FooterStateUpdate { cwd, .. } => {
            assert_eq!(cwd, sub_str, "the second emission must carry the subdirectory");
        }
        other => panic!("expected FooterStateUpdate chunk, got {other:?}"),
    }
}

// =============================================================================
// Scenario: The NAPI hook path delegates to the same shared footer poller
// implementation
// =============================================================================

#[test]
fn scenario_napi_hook_path_delegates_to_the_same_shared_footer_poller_implementation() {
    // Pure source-shape check (file reads only) — no process-global state is
    // touched, so the behavioural ENV_GUARD is not needed here.
    let napi_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../napi/src");
    let footer_poller_src =
        std::fs::read_to_string(napi_src.join("footer_poller.rs")).expect("read napi footer_poller.rs");
    let session_hooks_src =
        std::fs::read_to_string(napi_src.join("session_hooks.rs")).expect("read napi session_hooks.rs");

    // @step Given the NAPI session hooks are installed on a session manager
    assert!(
        session_hooks_src.contains("install_napi_session_manager_hooks"),
        "the NAPI install entry point must still exist"
    );

    // @step When a session is created under the NAPI hook implementation
    // (the hook delegates to the crate shim, which must now run the shared
    // NAPI-free poller)
    assert!(
        session_hooks_src.contains("crate::footer_poller::spawn_footer_poller"),
        "WT-002: NapiSessionManagerHooks must still delegate spawn to crate::footer_poller (shape pinned by RPC-043 tests)"
    );
    assert!(
        session_hooks_src.contains("crate::footer_poller::stop_footer_poller"),
        "WT-002: NapiSessionManagerHooks must still delegate stop to crate::footer_poller (shape pinned by RPC-043 tests)"
    );

    // @step Then the footer poller for that session runs the shared NAPI-free poller (codelet-sessions)
    assert!(
        footer_poller_src.contains("codelet_sessions::footer_poller::spawn_footer_poller"),
        "WT-002: the napi footer_poller.rs shim must delegate to the shared NAPI-free poller in codelet-sessions"
    );

    // @step And the NAPI hook still delegates spawn/stop to its crate::footer_poller shim so existing shape tests stay green
    assert!(
        footer_poller_src.contains("codelet_sessions::footer_poller::stop_footer_poller"),
        "WT-002: the napi footer_poller.rs shim must delegate stop to the shared NAPI-free poller"
    );
}
