//! WT-004 — tool-callbacks wiring: work-unit stage lookup without a work
//! unit, block notifications on the manager's chunk stream, and the
//! build_service registration source shape.
//!
//! Feature: spec/features/wt004-tool-callbacks-wiring.feature
//!
//! Split out of wt004_session_tool_callbacks.rs by REFAC-053; shared helpers
//! live in `wt004_session_tool_callbacks_support`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod wt004_session_tool_callbacks_support;

use std::path::Path;
use std::sync::Arc;

use uuid::Uuid;

use codelet_rpc_types::{NotificationSeverity, SessionId, StreamChunk};
use codelet_sessions::SessionManager;
use codelet_tools::facade::get_isolation_context;
use wt004_session_tool_callbacks_support::{
    create_plain, fresh_git_repo, manager_with_seeded_cache, register_tool_callbacks, ENV_GUARD,
};

/// Register the manager's chunks_tx as the block-notification emission
/// target (mirrors build_service's WT-002 registration).
fn register_chunk_sender(manager: &Arc<SessionManager>) {
    codelet_sessions::footer_poller::register_chunk_sender(manager.chunks_tx().clone());
}

/// Pull the next chunk for `sid` off the broadcast, skipping chunks for
/// other sessions or that do not match (TEST-042: under the deadlock
/// guard, not a timeout).
async fn next_matching_chunk(
    rx: &mut tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
    sid: &SessionId,
    matches: impl Fn(&StreamChunk) -> bool,
) -> StreamChunk {
    // TEST-042: wait for the chunk under the deadlock guard, not a deadline.
    codelet_test_helpers::deadlock_guard("the expected chunk", async {
        loop {
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
    })
    .await
    .unwrap_or_else(|failure| panic!("{failure}"))
}

/// codelet workspace root (the sessions crate lives at `rust/sessions`).
fn rust_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sessions lives under rust/")
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
        .expect("build_service must end with a top-level closing brace");
    &rest[..end]
}

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
    assert_eq!(stage, Ok(None), "no work unit context => no stage");
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_a_blocked_tool_action_emits_a_warning_notification_on_the_managers_chunk_stream()
{
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
                message, "AI was blocked from writing /project/src/lib.rs - path outside worktree",
                "message format must stay stable (TUI renders it verbatim)"
            );
        }
        other => panic!("expected UserNotification chunk, got {other:?}"),
    }
}

#[serial_test::serial]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_with_no_manager_registered_the_isolation_lookup_does_not_consult_the_singleton_manager(
) {
    let _guard = ENV_GUARD.lock().await;

    // @step Given the tool callbacks are installed the way build_service installs them
    // (installed here, not by a sibling test: BUG-398)
    let manager = manager_with_seeded_cache();
    register_tool_callbacks(&manager);

    // @step And no tool-callbacks manager is registered
    codelet_sessions::session_tool_callbacks::unregister_manager();

    // @step And an isolated session created in a second, unregistered manager against a fresh git repository
    let unregistered = SessionManager::new();
    let repo = fresh_git_repo();
    let id = Uuid::new_v4().to_string();
    unregistered
        .create_isolated_session_with_id(
            &id,
            "anthropic/claude-opus-4-5",
            repo.path().to_str().expect("repo path is utf8"),
            "bug398-no-singleton-fallback",
        )
        .await
        .expect("isolated session creation in the unregistered manager must succeed");

    // @step When the tool layer resolves the isolation context for that session id
    let ctx = get_isolation_context(Uuid::parse_str(&id).expect("id is a uuid"));

    // @step Then the lookup fails instead of finding the unregistered manager's session
    assert!(
        ctx.is_err(),
        "BUG-398/BUG-405: with no manager registered the lookup must be refused, got {ctx:?}"
    );
}

#[test]
fn scenario_shared_callbacks_are_plain_function_pointers() {
    // @step Given the shared isolation, stage and block-notification functions
    // @step When they are bound to the codelet-tools callback types and called with a malformed session id
    // (binding them to the type aliases is the compile-time plain-function-pointer check)
    let isolation: codelet_tools::facade::GetEffectiveCwdCallback =
        codelet_sessions::session_tool_callbacks::isolation_context;
    let stage: codelet_tools::facade::GetWorkUnitStageCallback =
        codelet_sessions::session_tool_callbacks::work_unit_stage;
    let _block: codelet_tools::facade::BlockNotificationCallback =
        codelet_sessions::session_tool_callbacks::emit_block_notification;
    let isolation_result = isolation("not-a-session-id".to_string());
    let stage_result = stage("not-a-session-id".to_string());

    // @step Then the isolation and stage callbacks both refuse the lookup
    let error = isolation_result.expect_err("a malformed session id must be refused");
    assert!(error.contains("not-a-session-id"), "{error}");
    // BUG-449: a failed stage lookup is an error (the write is refused).
    let error = stage_result.expect_err("a malformed session id must be refused");
    assert!(error.contains("not-a-session-id"), "{error}");
}

#[test]
fn scenario_build_service_registers_the_tool_callbacks_manager_and_all_three_tool_callbacks() {
    // @step Given the fspec binary crate source after this fix
    let common_src = codelet_test_helpers::source_composition::read_tree_or_file(
        rust_root().join("fspec/src/common.rs"),
    )
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
        .find("SessionManager::new()")
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
