#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/completion-snapshot-delegation-guards.feature
//!
//! CONT-011 guard: the shared snapshot delegation keeps its semantic basis
//! check and all four logical writer checks. Its own test file per the product
//! owner (2026-11-05, COV-075); the behaviour and helpers are the same as
//! cmpct041_pre_compaction_basis_test.rs, which keeps CMPCT-041's scenarios.

#[path = "support/cont011_delegation.rs"]
mod delegation;

use std::path::PathBuf;

// ===========================================================================
// Helpers (copied from cmpct041_pre_compaction_basis_test.rs)
// ===========================================================================

fn read_workspace_source(rel: &str) -> String {
    // CARGO_MANIFEST_DIR = rust/agent-loop; walk up two parents to repo root.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(rel))
        .unwrap_or_else(|| PathBuf::from(rel));
    codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

/// Extract a balanced-brace block beginning at `start_marker` inside `src`.
fn extract_brace_block_after<'a>(src: &'a str, start_marker: &str) -> &'a str {
    let arm_start = src
        .find(start_marker)
        .unwrap_or_else(|| panic!("source must contain `{start_marker}`"));
    let bytes = src.as_bytes();
    let mut i = arm_start + start_marker.len();
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    assert!(
        i < bytes.len() && bytes[i] == b'{',
        "expected `{{` after `{start_marker}`"
    );
    let body_start = i;
    let mut depth: i32 = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[body_start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("unterminated brace block starting at `{start_marker}`");
}

/// Hermetic BackgroundSession via the SessionManager + stub provider,
/// mirroring `rpc086_token_tracking.rs`.
#[cfg(feature = "test-support")]
async fn create_background_session() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    std::sync::Arc<codelet_sessions::background_session::BackgroundSession>,
) {
    use std::sync::Arc;

    use codelet_agent_loop::FspecAgentHooks;
    use codelet_sessions::session_manager::SessionManager;
    use uuid::Uuid;

    let data_dir = tempfile::tempdir().expect("data dir tempdir");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.path().to_path_buf())
        .expect("isolated snapshot data directory must be installed");
    codelet_core::persistence::reset_stores_for_tests();

    let manager = SessionManager::new();
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));

    codelet_providers::stub_provider::register_stub_provider();
    assert_eq!(manager.set_default_model("stub/canned"), Ok(()));

    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp
        .path()
        .to_str()
        .expect("tempdir path is utf8")
        .to_string();
    let session_id_str = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &session_id_str,
            "stub/canned",
            &project,
            "cmpct041-test-session",
        )
        .await
        .expect("create_session_with_id");

    let session = manager
        .get_session(&session_id_str)
        .expect("session must exist after create_session_with_id");
    (data_dir, tmp, session)
}

// ===========================================================================
// Scenario: Shared snapshot delegation retains semantic and writer guards
// ===========================================================================

#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn shared_snapshot_delegation_retains_semantic_and_writer_guards() {
    use codelet_cli::interactive::output::TokenInfo;
    use codelet_core::{ApiTokenUsage, StreamingTokenDisplay};

    // @step Given a background session whose token tracker reads 180000 tokens and whose cached display tokens were fed by the seed emit
    let (_data_dir, _tmp, session) = create_background_session().await;
    {
        let mut inner = session.inner.lock().await;
        let usage = ApiTokenUsage::new(30_000, 150_000, 0, 500);
        inner.token_tracker.update_from_usage(&usage, 500);
        assert_eq!(inner.token_tracker.input_tokens, 180_000, "sanity");
    }
    // Seed emit feeds cached_input_tokens exactly as the Tokens arm does.
    let (tracker_total, tracker_out, tracker_cr, tracker_cc) = {
        let inner = session.inner.lock().await;
        (
            inner.token_tracker.input_tokens,
            inner.token_tracker.output_tokens,
            inner.token_tracker.cache_read_input_tokens.unwrap_or(0),
            inner.token_tracker.cache_creation_input_tokens.unwrap_or(0),
        )
    };
    let display = StreamingTokenDisplay::from_cache_inclusive_total(
        tracker_total,
        tracker_out,
        tracker_cr,
        tracker_cc,
    );
    let info: TokenInfo = display.current().into();
    session.update_tokens(info.input_tokens as u32, info.output_tokens as u32);

    // @step When the auto-compaction path snapshots the pre-compaction token count
    let auto_basis = session.snapshot_pre_compaction_tokens();

    // @step And the manual compaction path reads its tracker-based original token count
    let manual_basis = {
        let inner = session.inner.lock().await;
        inner.token_tracker.input_tokens as u32
    };

    // @step Then both paths report the same 180000-token basis
    assert_eq!(
        auto_basis, manual_basis,
        "auto (cached display) and manual (tracker) bases must be equal for identical state"
    );
    assert_eq!(auto_basis, 180_000, "both bases must be the true total");

    // @step And both the agent-loop twin and the NAPI twin route the snapshot through the same shared session accessor
    let agent_loop_src = read_workspace_source("rust/agent-loop/src/background_output.rs");
    let agent_loop_arm =
        extract_brace_block_after(&agent_loop_src, "StreamEvent::CompactionStarted =>");
    assert!(
        agent_loop_arm.contains("snapshot_pre_compaction_tokens()"),
        "agent-loop twin CompactionStarted arm must use the shared accessor; arm was:\n{agent_loop_arm}"
    );
    let engine_arm = delegation::engine_compaction_arm();
    assert!(
        engine_arm.contains("snapshot_pre_compaction_tokens()"),
        "engine CompactionStarted arm must use the shared accessor; arm was:\n{engine_arm}"
    );
    // @step And all four logical writers retain the shared snapshot and storage accessors
    all_four_pre_compaction_writers_route_through_shared_accessors();
}

/// The four logical pre-compaction writers route through the shared
/// `BackgroundSession` accessors (same checks as the CMPCT-041 structural
/// guard; runnable without the `test-support` feature).
#[test]
fn all_four_pre_compaction_writers_route_through_shared_accessors() {
    let sessions_src = read_workspace_source("rust/sessions/src/background_session/metrics.rs");
    assert!(
        sessions_src.contains("pub fn snapshot_pre_compaction_tokens"),
        "BackgroundSession must expose snapshot_pre_compaction_tokens (AUTO accessor)"
    );
    assert!(
        sessions_src.contains("pub fn store_pre_compaction_tokens"),
        "BackgroundSession must expose store_pre_compaction_tokens (manual accessor)"
    );

    let arm = delegation::engine_compaction_arm();

    // BUG-516: the manual /compact writer lives in
    // BackgroundSession::start_manual_compaction.
    let handle_impl_src =
        read_workspace_source("rust/sessions/src/background_session/compaction_handoff.rs");

    assert!(
        arm.contains("snapshot_pre_compaction_tokens()"),
        "[agent-loop] AUTO CompactionStarted arm must call the shared \
         snapshot_pre_compaction_tokens() accessor; arm was:\n{arm}"
    );
    assert!(
        !arm.contains("pre_compaction_tokens") || !arm.contains(".store("),
        "[agent-loop] AUTO CompactionStarted arm must not hand-roll the \
         load/store snapshot; arm was:\n{arm}"
    );

    assert!(
        handle_impl_src.contains("store_pre_compaction_tokens("),
        "manual sessions RPC writer (start_manual_compaction) must route through \
         store_pre_compaction_tokens()"
    );
}
