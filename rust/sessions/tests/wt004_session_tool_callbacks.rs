//! WT-004 — Isolation path-validation callback is never registered in the
//! fspec binary.
//!
//! Feature: spec/features/isolation-path-validation-callback-is-never-registered-in-the-fspec-binary-isolated-sessions-are-not-actually-pinned-to-their-worktree.feature
//!
//! These scenarios drive the real, NAPI-free tool-callbacks module
//! (`codelet_sessions::session_tool_callbacks`) exactly the way the fspec
//! binary's `build_service` registers it: a NON-singleton `SessionManager`
//! is registered via `register_manager`, then the shared callback functions
//! are used as the tool layer's effective-cwd / work-unit-stage /
//! block-notification callbacks. The tool-layer validation goes through the
//! real `codelet_tools::facade` entry points (`get_isolation_context` and
//! `validate_and_resolve_path_with_isolation`), so a green test proves the
//! boundary is live under the fspec binary rather than in the NAPI adapter
//! only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use uuid::Uuid;

use codelet_rpc_types::{NotificationSeverity, SessionId, StreamChunk};
use codelet_sessions::SessionManager;
use codelet_tools::facade::{
    get_isolation_context, set_block_notification_callback, set_get_effective_cwd_callback,
    set_get_work_unit_stage_callback, validate_and_resolve_path_with_isolation, IsolationContext,
};
use codelet_tools::ToolError;

/// Trimmed offline models.dev catalog (anthropic/openai/google) — shared
/// with the RPC-385 / WT-001 / WT-002 precedent so registry validation stays
/// fully offline.
const MODELS_FIXTURE: &str = include_str!("fixtures/prov101_models.json");

/// Serialises tests that swap process-global state (data dir, HOME, the
/// tool-callbacks manager slot, the footer-poller chunk-sender slot, and
/// codelet-tools' callback OnceLocks) so parallel tests cannot observe
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

/// Create a fresh git repository (temp dir) with the branch explicitly named
/// `main` and one committed file.
fn fresh_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir for git repo");
    let path = dir.path();
    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    std::fs::write(path.join("hello.txt"), "hello\n").expect("write hello.txt");
    std::fs::create_dir_all(path.join("src")).expect("create src dir");
    std::fs::write(path.join("src").join("main.rs"), "fn main() {}\n").expect("write src/main.rs");
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "initial commit"]);
    dir
}

/// Process-wide hermetic dirs, created ONCE per test binary:
/// `codelet_common::set_data_directory` is first-call-wins, so a per-test
/// data dir would leak into later tests whose tempdir is already dropped
/// (manifest writes then fail with ENOENT).
static SHARED_DIR: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();

fn shared_dir() -> &'static std::path::Path {
    SHARED_DIR
        .get_or_init(|| tempfile::TempDir::new().expect("tempdir for data dir"))
        .path()
}

/// Build a manager rooted in the shared hermetic data dir with the offline
/// models.dev fixture pre-seeded, with HOME redirected so the session
/// manifest writes stay hermetic.
fn manager_with_seeded_cache() -> Arc<SessionManager> {
    set_dummy_credentials();
    let cache_dir = shared_dir().join("cache");
    std::fs::create_dir_all(&cache_dir).expect("create cache dir");
    std::fs::write(cache_dir.join("models.json"), MODELS_FIXTURE).expect("seed models.json");
    let _ = codelet_common::set_data_directory(shared_dir().to_path_buf());
    let home_dir = shared_dir().join("home");
    std::fs::create_dir_all(&home_dir).expect("create home dir");
    std::env::set_var("HOME", &home_dir);
    Arc::new(SessionManager::new())
}

/// Wire the tool-callbacks manager slot + the codelet-tools OnceLocks the
/// way `build_service` does it (WT-004 architecture note). Idempotent per
/// process: the OnceLocks accept the first set only, `register_manager`
/// replaces the previous registration.
fn register_tool_callbacks(manager: &Arc<SessionManager>) {
    codelet_sessions::session_tool_callbacks::register_manager(Arc::clone(manager));
    set_get_effective_cwd_callback(codelet_sessions::session_tool_callbacks::isolation_context);
    set_get_work_unit_stage_callback(codelet_sessions::session_tool_callbacks::work_unit_stage);
    set_block_notification_callback(
        codelet_sessions::session_tool_callbacks::emit_block_notification,
    );
}

/// Register the manager's chunks_tx as the block-notification emission
/// target (mirrors build_service's WT-002 registration).
fn register_chunk_sender(manager: &Arc<SessionManager>) {
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
}

/// Pull the next chunk for `sid` off the broadcast, skipping chunks for
/// other sessions or that do not match. Times out after `timeout`.
async fn next_matching_chunk(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
    timeout: Duration,
    matches: impl Fn(&StreamChunk) -> bool,
) -> StreamChunk {
    let deadline = tokio::time::Instant::now() + timeout;
    while tokio::time::Instant::now() < deadline {
        match rx.recv().await {
            Ok((id, chunk)) => {
                if id == *sid && matches(&chunk) {
                    return chunk;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                panic!("chunks broadcast closed before the expected chunk arrived");
            }
        }
    }
    panic!("timed out waiting for a matching chunk for {sid}");
}

/// Create an isolated session through the registered manager's in-memory
/// API (the same call the fspec binary drives via RPC).
async fn create_isolated(manager: &Arc<SessionManager>, repo: &Path) -> String {
    let id = Uuid::new_v4().to_string();
    manager
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.to_str().expect("repo path is utf8"),
            "wt004",
        )
        .await
        .expect("isolated session creation must succeed");
    id
}

/// Create a plain (non-isolated) session through the registered manager.
async fn create_plain(manager: &Arc<SessionManager>, project: &str) -> String {
    let id = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(&id, "anthropic/claude-opus-4-5", project, "wt004-plain")
        .await
        .expect("plain session creation must succeed");
    id
}

// =============================================================================
// Scenario: Isolated session under the fspec binary exposes its isolation
// context to the tool layer
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolated_session_under_the_fspec_binary_exposes_its_isolation_context_to_the_tool_layer() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a fresh git repository with one committed file
    let repo = fresh_git_repo();

    // @step And a non-singleton session manager whose tool callbacks are registered the way build_service registers them
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step When an isolated session is created against that repository through the registered manager
    let session_id = create_isolated(&manager, repo.path()).await;

    // @step Then the tool layer resolves an isolation context for that session with the worktree path of the created worktree
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"))
        .expect("WT-004: the tool layer must resolve an isolation context for an isolated session under the fspec binary");
    let worktree = repo.path().join(".fspec").join("worktrees").join(&session_id);
    assert_eq!(
        ctx.worktree_path,
        worktree,
        "worktree_path must be the created worktree"
    );

    // @step And the isolation context's blocked project path is the repository root
    assert_eq!(
        ctx.blocked_project_path,
        repo.path().to_path_buf(),
        "blocked_project_path must be the session's project root"
    );
}

// =============================================================================
// Scenario: Isolation lookup through the registered manager ignores the
// global singleton
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_isolation_lookup_through_the_registered_manager_ignores_the_global_singleton() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And an isolated session created in that manager against a fresh git repository
    let repo = fresh_git_repo();
    let session_id = create_isolated(&manager, repo.path()).await;

    // @step When the tool layer resolves the isolation context for that session id
    // (the singleton manager has never seen this session — it was created in
    // the non-singleton manager, exactly like the fspec binary does)
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"))
        .expect("WT-004: the registered manager must be consulted, not the singleton");

    // @step Then the lookup succeeds even though the singleton manager has no such session
    assert!(
        codelet_sessions::SessionManager::instance()
            .get_session(&session_id)
            .is_err(),
        "sanity: the session must NOT exist in the singleton manager"
    );

    // @step And the returned context matches the session's worktree path and project root
    assert_eq!(
        ctx.blocked_project_path,
        repo.path().to_path_buf(),
        "the context must come from the registered manager's session record"
    );
}

// =============================================================================
// Scenario: Write tool of an isolated session blocks absolute paths into the
// original project
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_write_tool_of_an_isolated_session_blocks_absolute_paths_into_the_original_project() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And an isolated session created against a fresh git repository that contains a tracked source file
    let repo = fresh_git_repo();
    let session_id = create_isolated(&manager, repo.path()).await;
    let target = repo.path().join("src").join("main.rs");
    let before = std::fs::read_to_string(&target).expect("read tracked source file");

    // @step When a file operation for that session validates an absolute path inside the original project root
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"))
        .expect("isolation context must be resolved");
    let result =
        validate_and_resolve_path_with_isolation(&target.to_string_lossy(), Some(&ctx), "write");

    // @step Then the validation fails with an error naming the blocked project path
    match result {
        Err(ToolError::Validation { tool, message }) => {
            assert_eq!(tool, "write");
            assert!(
                message.contains("original project"),
                "error must name the blocked project path, got: {message}"
            );
        }
        other => panic!("expected a write validation error, got {other:?}"),
    }

    // @step And the original project file is not modified
    assert_eq!(
        std::fs::read_to_string(&target).expect("re-read tracked source file"),
        before,
        "the original project file must not be touched by the blocked write"
    );
}

// =============================================================================
// Scenario: Write tool of an isolated session resolves relative paths into
// the worktree
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_write_tool_of_an_isolated_session_resolves_relative_paths_into_the_worktree() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And an isolated session created against a fresh git repository
    let repo = fresh_git_repo();
    let session_id = create_isolated(&manager, repo.path()).await;

    // @step When a file operation for that session validates a relative path
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"))
        .expect("isolation context must be resolved");
    let resolved =
        validate_and_resolve_path_with_isolation("src/feature.rs", Some(&ctx), "write")
            .expect("a relative path inside the worktree must be allowed");

    // @step Then the resolved path lands inside the session's worktree directory
    let worktree = repo.path().join(".fspec").join("worktrees").join(&session_id);
    assert!(
        resolved.starts_with(&worktree),
        "relative path must resolve inside the worktree, got {resolved:?} (worktree {worktree:?})"
    );
}

// =============================================================================
// Scenario: Non-isolated session keeps unrestricted path access
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_non_isolated_session_keeps_unrestricted_path_access() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And a plain non-isolated session created in that manager
    let repo = fresh_git_repo();
    let session_id = create_plain(&manager, repo.path().to_str().expect("utf8")).await;

    // @step When the tool layer resolves the isolation context for that session
    let ctx = get_isolation_context(Uuid::parse_str(&session_id).expect("session id is a uuid"));

    // @step Then it gets no isolation context
    assert!(
        ctx.is_none(),
        "non-isolated sessions must NOT carry an isolation context (they keep full access)"
    );

    // @step And a path validation for any absolute path outside the project succeeds unchanged
    let outside = tempfile::tempdir().expect("tempdir outside the project");
    let path = outside.path().join("loose.txt");
    let resolved =
        validate_and_resolve_path_with_isolation(path.to_str().expect("utf8"), None, "read")
            .expect("no isolation => allow-all paths");
    assert_eq!(
        resolved,
        path,
        "without an isolation context the path must pass through unchanged"
    );
}

// =============================================================================
// Scenario: Unknown session ids degrade to no isolation
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_unknown_session_ids_degrade_to_no_isolation() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step When the tool layer resolves the isolation context for a session id that no manager knows
    let unknown = Uuid::new_v4();

    // @step Then it gets no isolation context
    assert!(
        get_isolation_context(unknown).is_none(),
        "unknown session ids must degrade to no isolation, never to an error"
    );
}

// =============================================================================
// Scenario: Work unit stage lookup under the fspec binary reads the
// session's work unit context
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_work_unit_stage_lookup_under_the_fspec_binary_reads_the_sessions_work_unit_context() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And a session with a work unit context whose status is testing
    let repo = fresh_git_repo();
    let session_id = create_plain(&manager, repo.path().to_str().expect("utf8")).await;
    let session = manager.get_session(&session_id).expect("session exists");
    session.set_work_unit_context(
        Some("WT-999".to_string()),
        Some("Test card".to_string()),
        Some("testing".to_string()),
    );

    // @step When the tool layer asks for that session's work unit stage
    let stage = codelet_sessions::session_tool_callbacks::work_unit_stage(session_id);

    // @step Then it returns testing
    assert_eq!(
        stage.as_deref(),
        Some("testing"),
        "the stage lookup must return the session's work unit context status"
    );
}

// =============================================================================
// Scenario: Work unit stage lookup returns None for sessions without a work
// unit
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_work_unit_stage_lookup_returns_none_for_sessions_without_a_work_unit() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And a session without a work unit context
    let repo = fresh_git_repo();
    let session_id = create_plain(&manager, repo.path().to_str().expect("utf8")).await;

    // @step When the tool layer asks for that session's work unit stage
    let stage = codelet_sessions::session_tool_callbacks::work_unit_stage(session_id);

    // @step Then it returns no stage
    assert_eq!(stage, None, "no work unit context => no stage");
}

// =============================================================================
// Scenario: A blocked tool action emits a warning notification on the
// manager's chunk stream
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_blocked_tool_action_emits_a_warning_notification_on_the_managers_chunk_stream() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given a non-singleton session manager registered as the tool-callbacks manager with its chunk sender registered
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);
    register_chunk_sender(&manager);
    let mut chunks_rx = manager.chunks_tx().subscribe();
    let session_id = Uuid::new_v4();

    // @step And a subscriber on that manager's chunks broadcast
    // (the subscriber is `chunks_rx` above)

    // @step When a block notification is emitted for a session with an action and a reason
    codelet_tools::facade::emit_block_notification(
        session_id,
        "writing /project/src/lib.rs",
        "path outside worktree",
    );

    // @step Then a UserNotification chunk with Warning severity is delivered on the broadcast
    let chunk = next_matching_chunk(
        &mut chunks_rx,
        &SessionId::new(session_id.to_string()),
        Duration::from_secs(5),
        |c| matches!(c, StreamChunk::UserNotification { .. }),
    )
    .await;
    match chunk {
        StreamChunk::UserNotification { message, severity } => {
            assert_eq!(
                severity,
                NotificationSeverity::Warning,
                "block notifications must carry Warning severity"
            );
            // @step And its message is "AI was blocked from {action} - {reason}"
            assert_eq!(
                message,
                "AI was blocked from writing /project/src/lib.rs - path outside worktree",
                "message format must stay stable (TUI renders it verbatim)"
            );
        }
        other => panic!("expected UserNotification chunk, got {other:?}"),
    }
}

// =============================================================================
// Scenario: The NAPI isolation lookup falls back to the singleton manager
// when no manager is registered
// =============================================================================

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_the_napi_isolation_lookup_falls_back_to_the_singleton_manager_when_no_manager_is_registered() {
    let _guard = ENV_GUARD.lock().await;

    // @step Given no tool-callbacks manager has been registered (fresh NAPI process)
    codelet_sessions::session_tool_callbacks::unregister_manager();

    // @step And an isolated session created in the singleton manager against a fresh git repository
    // (seed the hermetic data dir + HOME the same way the non-singleton tests
    // do: `set_data_directory` is first-call-wins and the provider manager
    // created for the session needs it at session-creation time)
    manager_with_seeded_cache();
    let singleton = SessionManager::instance();
    let repo = fresh_git_repo();
    let id = Uuid::new_v4().to_string();
    singleton
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "wt004-napi-fallback",
        )
        .await
        .expect("isolated session creation in the singleton must succeed");

    // @step When the tool layer resolves the isolation context for that session id
    let ctx = get_isolation_context(Uuid::parse_str(&id).expect("id is a uuid"))
        .expect("WT-004: with no manager registered, the singleton fallback must find the session");

    // @step Then the singleton manager's session is found and the context matches its worktree path and project root
    let worktree = repo.path().join(".fspec").join("worktrees").join(&id);
    assert_eq!(ctx.worktree_path, worktree, "worktree_path must match the singleton session's worktree");
    assert_eq!(
        ctx.blocked_project_path,
        repo.path().to_path_buf(),
        "blocked_project_path must match the singleton session's project root"
    );
}

// =============================================================================
// Compile-time shape: the shared callbacks are plain function pointers
// (business rule 2 — no new OnceLocks in codelet-tools, existing callback
// types unchanged).
// =============================================================================

#[test]
fn scenario_shared_callbacks_are_plain_function_pointers() {
    // @step Given the fspec binary crate source after this fix
    // (the callback signatures are compile-time checked here against the
    // codelet-tools callback type aliases)
    let _iso: codelet_tools::facade::GetEffectiveCwdCallback =
        codelet_sessions::session_tool_callbacks::isolation_context;
    let _stage: codelet_tools::facade::GetWorkUnitStageCallback =
        codelet_sessions::session_tool_callbacks::work_unit_stage;
    let _block: codelet_tools::facade::BlockNotificationCallback =
        codelet_sessions::session_tool_callbacks::emit_block_notification;

    // @step When build_service is inspected
    // (wiring is asserted by the source-shape scenarios below in this file)

    // @step Then the tool layer's callback types accept the shared functions unchanged
    assert!(
        std::any::type_name::<IsolationContext>().contains("IsolationContext"),
        "sanity: the IsolationContext type is the codelet-tools one"
    );
}

// =============================================================================
// Source-shape scenarios for the two-front-doors wiring (structural — the
// process-global callback OnceLocks are first-set and the NAPI module is a
// separate binary, so these cannot be observed at runtime).
// =============================================================================

/// codelet workspace root (the sessions crate lives at `rust/sessions`).
fn rust_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("sessions lives under rust/")
}

/// Strip `//`-style line comments so substring scans only see code.
fn strip_line_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        match line.find("//") {
            Some(pos) => {
                out.push_str(&line[..pos]);
                out.push('\n');
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

/// Extract the body of `pub fn build_service` from the fspec binary's
/// common.rs (up to the first `\n}` at top-level indentation after the
/// signature).
fn build_service_body(common_src: &str) -> &str {
    let start = common_src
        .find("pub fn build_service")
        .expect("build_service must exist in common.rs");
    let rest = &common_src[start..];
    let end = rest
        .find("\n}\n")
        .map(|i| i + 2)
        .unwrap_or_else(|| rest.len());
    &rest[..end]
}

// =============================================================================
// Scenario: build_service registers the tool-callbacks manager and all
// three tool callbacks
// =============================================================================

#[test]
fn scenario_build_service_registers_the_tool_callbacks_manager_and_all_three_tool_callbacks() {
    // @step Given the fspec binary crate source after this fix
    let common_src =
        std::fs::read_to_string(rust_root().join("fspec/src/common.rs"))
            .expect("fspec/src/common.rs must be readable");
    let stripped = strip_line_comments(&common_src);

    // @step When build_service is inspected
    let body = build_service_body(&stripped);

    // @step Then it calls session_tool_callbacks::register_manager with its non-singleton SessionManager
    assert!(
        body.contains("session_tool_callbacks::register_manager"),
        "build_service must call codelet_sessions::session_tool_callbacks::register_manager with its non-singleton SessionManager (WT-004)"
    );

    // @step And it registers the shared isolation_context function as the effective-cwd callback
    assert!(
        body.contains("set_get_effective_cwd_callback")
            && body.contains("session_tool_callbacks::isolation_context"),
        "build_service must register the shared isolation_context function via set_get_effective_cwd_callback (WT-004)"
    );

    // @step And it registers the shared work_unit_stage function as the work-unit-stage callback
    assert!(
        body.contains("set_get_work_unit_stage_callback")
            && body.contains("session_tool_callbacks::work_unit_stage"),
        "build_service must register the shared work_unit_stage function via set_get_work_unit_stage_callback (WT-004)"
    );

    // @step And it registers the shared emit_block_notification function as the block-notification callback
    assert!(
        body.contains("set_block_notification_callback")
            && body.contains("session_tool_callbacks::emit_block_notification"),
        "build_service must register the shared emit_block_notification function via set_block_notification_callback (WT-004)"
    );

    // @step And the registration happens after the SessionManager is constructed and before the service is returned
    let sm_idx = body
        .find("Arc::new(SessionManager::new())")
        .expect("build_service must construct the SessionManager (RPC-044)");
    let register_idx = body
        .find("session_tool_callbacks::register_manager")
        .expect("register_manager call must exist");
    assert!(
        register_idx > sm_idx,
        "register_manager must run AFTER the SessionManager is constructed"
    );
    let return_idx = body
        .rfind("Ok(Arc::new(")
        .expect("build_service must end by returning the service");
    assert!(
        register_idx < return_idx,
        "register_manager must run BEFORE the service is returned"
    );
}

// =============================================================================
// Scenario: The NAPI front door registers the shared codelet-sessions
// callback functions
// =============================================================================

#[test]
fn scenario_the_napi_front_door_registers_the_shared_codelet_sessions_callback_functions() {
    // @step Given the codelet-napi crate source after this fix
    let bridges_src =
        std::fs::read_to_string(rust_root().join("napi/src/bridges.rs"))
            .expect("napi/src/bridges.rs must be readable");
    let stripped = strip_line_comments(&bridges_src);

    // @step When init_block_notification_callbacks is inspected
    // (the function itself stays — the RPC-043 shape test pins it)
    assert!(
        stripped.contains("fn init_block_notification_callbacks"),
        "init_block_notification_callbacks must remain (RPC-043 shape)"
    );

    // @step Then it registers the shared codelet-sessions isolation_context function as the effective-cwd callback
    assert!(
        stripped.contains("codelet_sessions::session_tool_callbacks::isolation_context"),
        "NAPI must register the SHARED isolation_context function (two-front-doors rule, WT-004)"
    );

    // @step And it registers the shared codelet-sessions work_unit_stage function as the work-unit-stage callback
    assert!(
        stripped.contains("codelet_sessions::session_tool_callbacks::work_unit_stage"),
        "NAPI must register the SHARED work_unit_stage function (two-front-doors rule, WT-004)"
    );

    // @step And it registers the shared codelet-sessions emit_block_notification function as the block-notification callback
    assert!(
        stripped.contains("codelet_sessions::session_tool_callbacks::emit_block_notification"),
        "NAPI must register the SHARED emit_block_notification function (two-front-doors rule, WT-004)"
    );

    // @step And the local napi-side stage and isolation lookups are removed and the RPC-043-pinned emitter name now delegates to the shared module
    // The stage + isolation lookups are gone entirely; the block-notification
    // emitter keeps its RPC-043-pinned name (`emit_block_notification_to_tui`)
    // but is now a thin shim delegating to the shared function — no local
    // chunk-building / chunks_tx-sending logic remains in bridges.rs.
    for duplicate in ["fn get_session_work_unit_stage", "fn get_session_effective_cwd"] {
        assert!(
            !stripped.contains(duplicate),
            "the local napi-side implementation `{duplicate}` must be removed — the shared codelet-sessions module is the single source of truth (WT-004)"
        );
    }
    assert!(
        stripped.contains("fn emit_block_notification_to_tui"),
        "emit_block_notification_to_tui keeps its name (RPC-043 shape test pins it)"
    );
    assert!(
        stripped.contains(
            "codelet_sessions::session_tool_callbacks::emit_block_notification(session_id_str, action, reason)"
        ),
        "emit_block_notification_to_tui must be a thin shim delegating to the shared emitter (WT-004)"
    );
    // The old local emitter built the chunk and sent it directly on the
    // singleton's chunks_tx — that logic must be gone from bridges.rs.
    assert!(
        !stripped.contains("StreamChunk::user_notification(message, NotificationSeverity::Warning)"),
        "the local chunk-building logic must be removed from bridges.rs (moved to codelet-sessions, WT-004)"
    );
}
