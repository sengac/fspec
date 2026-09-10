//! WT-012 — `create_isolated_session_with_id` must route through the shared
//! session-creation helper (`create_background_session_inner`) instead of
//! re-implementing the pre-RPC-425 bootstrap inline.
//!
//! Feature: spec/features/create-isolated-session-with-id-duplicates-the-pre-rpc-425-session-creation-code-path-instead-of-using-the-shared-helper.feature
//!
//! Scenarios drive a real `SessionManager` against a fresh git repository
//! (temp dir, HOME redirected to a temp dir so the `~/.fspec/git-sessions`
//! manifest writes stay hermetic), plus source-shape pins for the
//! delegation and a shared end-to-end pre-tool-hook check.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use codelet_tools::pre_tool_hook::pre_tool_hook_check;
use codelet_tools::McpInjection;
use serde_json::Value;
use serial_test::serial;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use codelet_sessions::background_session::{BackgroundSession, PromptInput};
use codelet_sessions::session_manager::SessionManager;
use codelet_sessions::SessionManagerHooks;

/// Trimmed offline models.dev catalog (anthropic/openai/google) — shared
/// with the WT-005 precedent so registry validation stays fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serializes the tests that swap process-global state: the data directory
/// (`codelet_common::set_data_directory`), the credential env vars, the
/// HOME redirect (git-session manifest + `~/.fspec/fspec-hooks.json`), and
/// the process-global pre-tool hook registry. An async mutex (not a std
/// one) so the guard is never held across an await point.
static ENV_AND_DATA_DIR_GUARD: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

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
fn manager_with_seeded_cache() -> (tempfile::TempDir, tempfile::TempDir, Arc<SessionManager>) {
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
    codelet_common::set_data_directory(data_dir.path().to_path_buf()).expect("set data directory");
    std::env::set_var("HOME", home_dir.path());
    let manager = Arc::new(SessionManager::new());
    (data_dir, home_dir, manager)
}

/// Build a manager rooted in a fresh data dir with the offline models.dev
/// fixture pre-seeded, HOME redirected to a temp dir, and the user profile
/// config isolated to `user_dir_config` (written to
/// `<FSPEC_USER_DIR>/fspec-config.json`).
fn manager_with_profile_config(
    user_dir_config: &str,
) -> (Vec<tempfile::TempDir>, Arc<SessionManager>) {
    set_dummy_credentials();
    for var in [
        "ANTHROPIC_API_KEY",
        "CLAUDE_CODE_OAUTH_TOKEN",
        "OPENAI_API_KEY",
        "GOOGLE_GENERATIVE_AI_API_KEY",
        "ZAI_API_KEY",
        "ZAI_PLAN_API_KEY",
    ] {
        std::env::remove_var(var);
    }

    let codex_home = tempfile::tempdir().expect("create empty CODEX_HOME");
    std::env::set_var("CODEX_HOME", codex_home.path());
    let fspec_home = tempfile::tempdir().expect("create empty FSPEC_HOME");
    std::env::set_var("FSPEC_HOME", fspec_home.path());
    let user_dir = tempfile::tempdir().expect("create temp FSPEC_USER_DIR");
    std::fs::write(user_dir.path().join("fspec-config.json"), user_dir_config)
        .expect("write fspec-config.json");
    std::env::set_var("FSPEC_USER_DIR", user_dir.path());

    let data_dir = tempfile::tempdir().expect("tempdir for data dir");
    let home_dir = tempfile::tempdir().expect("tempdir for HOME");
    let cache_dir = data_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("seed models.json");
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(data_dir.path().to_path_buf()).expect("set data directory");
    std::env::set_var("HOME", home_dir.path());
    let manager = Arc::new(SessionManager::new());
    (
        vec![codex_home, fspec_home, user_dir, data_dir, home_dir],
        manager,
    )
}

/// Record the footer-poller spawns requested from the session manager, so
/// the isolation-surface scenarios can assert the worktree cwd was used.
struct RecordingFooterHooks {
    spawned: Mutex<Vec<(String, String, Option<String>)>>,
}

impl RecordingFooterHooks {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            spawned: Mutex::new(Vec::new()),
        })
    }
}

impl SessionManagerHooks for RecordingFooterHooks {
    fn spawn_agent_loop(
        &self,
        _session: Arc<BackgroundSession>,
        _input_rx: mpsc::Receiver<PromptInput>,
        _mcp_injection_rx: mpsc::Receiver<McpInjection>,
    ) {
    }

    fn spawn_scheduler(&self, _project: String, _rt: tokio::runtime::Handle) {}

    fn ensure_scheduler_running_for_loop(&self, _project: String, _rt: tokio::runtime::Handle) {}

    fn spawn_footer_poller(&self, session_id: String, cwd: String, worktree_path: Option<String>) {
        let mut guard = self.spawned.lock().expect("spawned lock poisoned");
        guard.push((session_id, cwd, worktree_path));
    }

    fn stop_footer_poller(&self, _session_id: &str) {}

    fn cleanup_session_loops(&self, _session_id: Uuid) {}
}

// =============================================================================
// Scenario: An isolated session against a registry model preserves the full
// isolation surface
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_an_isolated_session_against_a_registry_model_preserves_the_full_isolation_surface(
) {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a git repository with one committed file
    let repo = fresh_git_repo();
    let head_sha = run_git_capture(repo.path(), &["rev-parse", "HEAD"]);

    // @step And a fresh session manager with an offline models registry seeded
    let (data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let footer_hooks = RecordingFooterHooks::new();
    manager.set_hooks(footer_hooks.clone());
    let mut chunks_rx = manager.chunks_tx().subscribe();
    let session_id = Uuid::new_v4().to_string();

    // @step When I create an isolated session with id "abc123" and registry model "anthropic/claude-opus-4-5"
    let info = manager
        .create_isolated_session_with_id(
            &session_id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "Isolated test",
        )
        .await
        .expect("isolated session creation must succeed");

    // @step Then the returned IsolatedSessionInfo has worktree_path "<repo>/.fspec/worktrees/abc123" and base_commit equal to the repo HEAD sha
    let expected_worktree = repo
        .path()
        .join(".fspec")
        .join("worktrees")
        .join(&session_id);
    assert_eq!(
        Path::new(&info.worktree_path),
        expected_worktree.as_path(),
        "IsolatedSessionInfo.worktree_path must point at .fspec/worktrees/<id>"
    );
    assert_eq!(
        info.base_commit, head_sha,
        "IsolatedSessionInfo.base_commit must be the repo HEAD sha"
    );

    // @step And the in-memory BackgroundSession carries worktree_path and base_commit
    let session = manager
        .get_session(&session_id)
        .expect("session must exist");
    assert_eq!(
        session.worktree_path().as_deref(),
        Some(expected_worktree.as_path()),
        "BackgroundSession must carry the worktree path"
    );
    assert_eq!(
        session.base_commit().as_deref(),
        Some(head_sha.as_str()),
        "BackgroundSession must carry the base commit"
    );

    // @step And the isolation context reminder was injected for the session
    assert_isolation_reminder_injected(&session, &head_sha);

    // @step And the session persistence manifest was written to disk best-effort
    let manifest_path = data_dir
        .path()
        .join("sessions")
        .join(format!("{session_id}.json"));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !manifest_path.exists() && std::time::Instant::now() < deadline {
        tokio::task::yield_now().await;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(
        manifest_path.exists(),
        "WT-005: the session persistence manifest must be written to disk (best-effort)"
    );

    // @step And an IsolationStateChange(true, worktree, base_commit) chunk was broadcast
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut saw_isolation_chunk = false;
    while std::time::Instant::now() < deadline {
        match chunks_rx.recv().await {
            Ok((sid, chunk)) if sid == codelet_rpc_types::SessionId::new(session_id.clone()) => {
                if let codelet_rpc_types::StreamChunk::IsolationStateChange {
                    is_isolated,
                    worktree_path,
                    base_commit,
                } = chunk
                {
                    assert!(is_isolated, "chunk must be an isolation ON event");
                    assert_eq!(
                        worktree_path.as_deref(),
                        Some(info.worktree_path.as_str()),
                        "chunk must carry the worktree path (WT-001)"
                    );
                    assert_eq!(
                        base_commit.as_deref(),
                        Some(head_sha.as_str()),
                        "chunk must carry the base commit (WT-001)"
                    );
                    saw_isolation_chunk = true;
                    break;
                }
            }
            Ok(_) => {}
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    assert!(
        saw_isolation_chunk,
        "an IsolationStateChange(true, worktree, base_commit) chunk must be broadcast on chunks_tx"
    );

    // @step And the footer poller was spawned with the worktree cwd
    let spawned = footer_hooks
        .spawned
        .lock()
        .expect("spawned lock poisoned")
        .clone();
    let entry = spawned
        .iter()
        .find(|(sid, _, _)| sid == &session_id)
        .expect("footer poller must be spawned for the isolated session");
    assert_eq!(
        entry.1, info.worktree_path,
        "footer poller cwd must be the worktree path"
    );
    assert_eq!(
        entry.2.as_deref(),
        Some(info.worktree_path.as_str()),
        "footer poller worktree_path must be the worktree path"
    );
}

// =============================================================================
// Scenario: An isolated session against a profile model seeds preserve-thinking
// and auto-continue from the profile
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_an_isolated_session_against_a_profile_model_seeds_preserve_thinking_and_auto_continue_from_the_profile(
) {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a stored profile "spark" with autoContinue 300 and preserveThinking false
    let (_dirs, manager) = manager_with_profile_config(
        r#"{
          "providers": {
            "openai": {
              "profiles": {
                "spark": {
                  "baseUrl": "http://spark:8001",
                  "apiKey": "test",
                  "contextWindow": 262144,
                  "autoContinue": 300,
                  "preserveThinking": false
                }
              }
            }
          }
        }"#,
    );

    // @step And a fresh session manager with an offline models registry seeded
    let repo = fresh_git_repo();

    // @step When I create an isolated session against model "openai:spark/o3"
    let session_id = Uuid::new_v4().to_string();
    let info = manager
        .create_isolated_session_with_id(
            &session_id,
            "openai:spark/o3",
            repo.path().to_str().expect("repo path is utf8"),
            "Isolated profile test",
        )
        .await
        .expect("isolated profile-model session creation must succeed");

    // @step Then the session's auto-continue is enabled with budget 300
    // @step And the session's preserve_thinking_enabled flag is false
    // @step And these seeds match what a non-isolated session against the same profile receives
    let session = manager
        .get_session(&session_id)
        .expect("session must exist");
    let (enabled, budget) = session.get_continue_state();
    assert!(
        enabled,
        "WT-012 (PROV-142): an isolated session against a profile with autoContinue=300 must seed auto-continue ON"
    );
    assert_eq!(
        budget, 300,
        "WT-012 (PROV-142): the seeded budget must be the profile's autoContinue value (300)"
    );
    let flag = {
        let guard = session.inner.try_lock().expect("idle session lock");
        guard.preserve_thinking_enabled
    };
    assert!(
        !flag,
        "WT-012 (PROV-143): preserveThinking=false must seed the flag OFF"
    );

    let _ = info;
}

// =============================================================================
// Scenario: create_isolated_session_with_id delegates shared bootstrap to the
// shared helper
// =============================================================================

#[test]
fn scenario_create_isolated_session_with_id_delegates_shared_bootstrap_to_the_shared_helper() {
    // @step Given the session manager source in rust/sessions/src/session_manager.rs
    let sm = read_source("session_manager.rs");
    let helper = read_source("session_creation_helper.rs");

    // @step When I inspect the create_isolated_session_with_id method body
    let body = find_function_body(&sm, "pub async fn create_isolated_session_with_id");
    assert!(
        !body.is_empty(),
        "create_isolated_session_with_id must exist in session_manager.rs"
    );

    // @step Then the body calls create_background_session_inner
    assert!(
        body.contains("create_background_session_inner"),
        "WT-012: create_isolated_session_with_id must delegate to create_background_session_inner"
    );

    // @step And the body no longer inlines BackgroundSession::new, load_lifecycle_hooks, register_pre_tool_hook, init_mcp_session, set_base_thinking_level, set_model_limits or set_session_model_vision
    for symbol in [
        "BackgroundSession::new",
        "load_lifecycle_hooks",
        "register_pre_tool_hook",
        "init_mcp_session",
        "set_base_thinking_level",
        "set_model_limits",
        "set_session_model_vision",
    ] {
        assert!(
            !body.contains(symbol),
            "WT-012: create_isolated_session_with_id must NOT inline `{symbol}` — it must come from the shared helper"
        );
    }

    // @step And those shared steps live only in session_creation_helper.rs
    for symbol in [
        "BackgroundSession::new",
        "load_lifecycle_hooks",
        "register_pre_tool_hook",
        "init_mcp_session",
        "set_base_thinking_level",
        "set_model_limits",
        "set_session_model_vision",
    ] {
        assert!(
            helper.contains(symbol),
            "session_creation_helper.rs must still own `{symbol}`"
        );
    }

    // @step And the method still contains create_worktree, create_session_manifest, isolation_state_change_with_base and the worktree-cwd footer poller
    for symbol in [
        "create_worktree",
        "create_session_manifest",
        "isolation_state_change_with_base",
        "spawn_footer_poller",
    ] {
        assert!(
            body.contains(symbol),
            "create_isolated_session_with_id must still contain `{symbol}` (isolation-specific)"
        );
    }
}

// =============================================================================
// Scenario: A pre_tool_use lifecycle hook is live for an isolated session
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_pre_tool_use_lifecycle_hook_is_live_for_an_isolated_session() {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a project whose spec/fspec-hooks.json defines a pre_tool_use command matching "Bash" that prints {"hookSpecificOutput":{"permissionDecision":"deny","reason":"no bash"}}
    let repo = fresh_git_repo();
    std::fs::create_dir_all(repo.path().join("spec")).expect("create spec dir");
    std::fs::write(
        repo.path().join("spec").join("fspec-hooks.json"),
        r#"{
          "global": { "timeout": 10, "shell": "bash -c" },
          "hooks": {
            "pre_tool_use": [
              {
                "matcher": "Bash",
                "hooks": [
                  { "command": "printf '{\"reason\":\"no bash\",\"hookSpecificOutput\":{\"permissionDecision\":\"deny\"}}'" }
                ]
              }
            ]
          }
        }"#,
    )
    .expect("write fspec-hooks.json");

    // @step And an isolated session created for that project
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();
    let session_id = Uuid::new_v4().to_string();
    manager
        .create_isolated_session_with_id(
            &session_id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "Isolated hooks test",
        )
        .await
        .expect("isolated session creation must succeed");

    // @step When a Bash tool call is checked for the isolated session
    let uuid = Uuid::parse_str(&session_id).expect("session id is a uuid");
    let result = pre_tool_hook_check(uuid, "Bash", &Value::Object(Default::default()));

    // @step Then the tool call is denied with the hook's reason "no bash"
    match result {
        Err(reason) => {
            assert_eq!(
                reason, "no bash",
                "the pre-tool hook registered by the shared helper must deny with the hook's reason"
            );
        }
        Ok(_) => panic!(
            "WT-012: the pre_tool_use hook must be live for isolated sessions — Bash was not denied"
        ),
    }
}

// =============================================================================
// Scenario: Creating an isolated session fails cleanly on precheck errors
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_creating_an_isolated_session_fails_cleanly_on_precheck_errors() {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a directory that is not a git repository
    let not_git = tempfile::tempdir().expect("tempdir for non-git dir");
    let (_data_dir, _home_dir, manager) = manager_with_seeded_cache();

    // @step When I call create_isolated_session_with_id for that directory
    let result = manager
        .create_isolated_session_with_id(
            &Uuid::new_v4().to_string(),
            "anthropic/claude-opus-4-5",
            not_git.path().to_str().expect("path is utf8"),
            "Not a git repo",
        )
        .await;

    // @step Then the call fails with a "Failed to create worktree" error before any session bootstrap runs
    let err = result.expect_err("worktree creation must fail for a non-git directory");
    assert!(
        err.contains("Failed to create worktree"),
        "the error must be 'Failed to create worktree ...', got: {err}"
    );

    // @step Given a git repository and a session id already present in the manager
    let repo = fresh_git_repo();
    let session_id = Uuid::new_v4().to_string();
    manager
        .create_isolated_session_with_id(
            &session_id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "Isolated test",
        )
        .await
        .expect("first isolated session creation must succeed");

    // @step When I call create_isolated_session_with_id again with that id
    let dup = manager
        .create_isolated_session_with_id(
            &session_id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "Isolated test",
        )
        .await
        .expect_err("duplicate id must fail");

    // @step Then the call fails with "Session <id> already exists"
    assert!(
        dup.contains("already exists"),
        "the duplicate-id error must say 'Session {session_id} already exists', got: {dup}"
    );
}

// =============================================================================
// Scenario: The shared provider-manager helper is behavior-preserving for
// non-isolated callers
// =============================================================================

#[serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_shared_provider_manager_helper_is_behavior_preserving_for_non_isolated_callers(
) {
    let _guard = ENV_AND_DATA_DIR_GUARD.lock().await;

    // @step Given a stored profile "spark" with autoContinue 300 and preserveThinking true
    let (_dirs, manager) = manager_with_profile_config(
        r#"{
          "providers": {
            "openai": {
              "profiles": {
                "spark": {
                  "baseUrl": "http://spark:8001",
                  "apiKey": "test",
                  "contextWindow": 262144,
                  "autoContinue": 300,
                  "preserveThinking": true
                }
              }
            }
          }
        }"#,
    );

    // @step And a fresh session manager with an offline models registry seeded
    let project = tempfile::tempdir().expect("tempdir for project");

    // @step When I create a non-isolated session via create_session_with_id against model "openai:spark/o3"
    let profile_id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &profile_id,
            "openai:spark/o3",
            project.path().to_str().expect("path is utf8"),
            "Profile parity test",
        )
        .await
        .expect("non-isolated profile-model session creation must succeed");

    // @step Then the session's auto-continue is enabled with budget 300
    // @step And the session's preserve_thinking_enabled flag is true
    let session = manager
        .get_session(&profile_id)
        .expect("session must exist");
    let (enabled, budget) = session.get_continue_state();
    assert!(
        enabled,
        "parity: a profile with autoContinue=300 must seed auto-continue ON"
    );
    assert_eq!(
        budget, 300,
        "parity: the seeded budget must be the profile's autoContinue value (300)"
    );
    let flag = {
        let guard = session.inner.try_lock().expect("idle session lock");
        guard.preserve_thinking_enabled
    };
    assert!(flag, "parity: preserveThinking=true must seed the flag ON");

    // @step Given a registry model "anthropic/claude-opus-4-5"
    // @step When I create a non-isolated session via create_session_with_id against that registry model
    let registry_id = Uuid::new_v4().to_string();
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-test-dummy-key");
    manager
        .create_session_with_id(
            &registry_id,
            "anthropic/claude-opus-4-5",
            project.path().to_str().expect("path is utf8"),
            "Registry parity test",
        )
        .await
        .expect("non-isolated registry-model session creation must succeed");

    // @step Then the session is created via the with_model_support plus select_model funnel and succeeds
    let registry_session = manager
        .get_session(&registry_id)
        .expect("registry session must exist");
    assert!(
        registry_session.model_id.read().expect("lock").is_some(),
        "the registry session must carry its model id"
    );
}

// =============================================================================
// Helpers
// =============================================================================

/// Read a crate source file (relative to `rust/sessions/src/`).
fn read_source(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join(name),
    )
    .expect("source file must exist")
}

/// Capture stdout of a git command (e.g. `rev-parse HEAD`).
fn run_git_capture(cwd: &Path, args: &[&str]) -> String {
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
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Assert the session's injected environment system-reminder carries the
/// isolation block (Isolation: ACTIVE / Worktree: / Base commit:).
fn assert_isolation_reminder_injected(session: &Arc<BackgroundSession>, head_sha: &str) {
    let guard = session.inner.try_lock().expect("idle session lock");
    let found = guard.messages.iter().any(|msg| {
        matches!(
            msg,
            rig::message::Message::User { content }
                if content
                    .iter()
                    .any(|item| {
                        matches!(
                            item,
                            rig::message::UserContent::Text(t)
                                if t.text.contains("Isolation: ACTIVE")
                                    && t.text.contains("Worktree:")
                                    && t.text.contains("Base commit:")
                        )
                    })
        )
    });
    assert!(
        found,
        "the injected context reminder must carry the isolation block (worktree + base commit)"
    );
    let short = &head_sha[..8.min(head_sha.len())];
    let has_base = guard.messages.iter().any(|msg| {
        matches!(
            msg,
            rig::message::Message::User { content }
                if content
                    .iter()
                    .any(|item| {
                        matches!(
                            item,
                            rig::message::UserContent::Text(t)
                                if t.text.contains(&format!("Base commit: {short}"))
                        )
                    })
        )
    });
    assert!(
        has_base,
        "the isolation reminder must carry the short base commit sha {short}"
    );
}

/// Find the body of a function in source code (brace-matched from the
/// `pub async fn` declaration).
fn find_function_body(content: &str, func_pattern: &str) -> String {
    if let Some(start) = content.find(func_pattern) {
        let brace_start = content[start..]
            .find('{')
            .map(|i| start + i)
            .unwrap_or(start);
        let rest = &content[brace_start..];
        let mut depth = 0;
        let mut end = brace_start;
        for (i, c) in rest.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = brace_start + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        content[brace_start..end].to_string()
    } else {
        String::new()
    }
}
