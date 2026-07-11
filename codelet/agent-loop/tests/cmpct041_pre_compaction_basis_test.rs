#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/pre-compaction-snapshot-basis-unification.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.
//!
//! CMPCT-041 basis unification: the AUTO CompactionStarted writers (agent-loop
//! twin `background_output.rs` and NAPI twin `napi/src/agent_loop.rs`) cannot
//! read `token_tracker` mid-stream — `BackgroundSession.inner` is a
//! `tokio::sync::Mutex` held by the streaming agent loop for the whole turn —
//! so they snapshot `cached_input_tokens` through the shared
//! `BackgroundSession::snapshot_pre_compaction_tokens()` accessor. After the
//! CMPCT-041 root seed fix the cached display basis equals the tracker basis
//! in the seed window, and the manual writers (`sessions/src/handle_impl.rs`,
//! `napi/src/session_bindings.rs`) route their tracker-based value through
//! the shared `store_pre_compaction_tokens()` accessor, so all four writers
//! agree on an equivalent basis. Behavioral tests are gated behind the
//! `test-support` feature (stub provider), mirroring
//! `rpc086_token_tracking.rs`; wiring tests run unconditionally.

use std::fs;
use std::path::PathBuf;

// ===========================================================================
// Helpers
// ===========================================================================

fn read_workspace_source(rel: &str) -> String {
    // CARGO_MANIFEST_DIR = codelet/agent-loop; walk up two parents to repo root.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(rel))
        .unwrap_or_else(|| PathBuf::from(rel));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
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
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());

    let manager = Arc::new(SessionManager::new());
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));

    codelet_providers::stub_provider::register_stub_provider();
    manager.set_default_model("stub/canned");

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
// Scenario: Overflow-recovery snapshot in the seed window records the true
//           context total
// ===========================================================================

#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn overflow_recovery_snapshot_records_true_total_in_seed_window() {
    use codelet_cli::interactive::output::TokenInfo;
    use codelet_core::StreamingTokenDisplay;
    use std::sync::atomic::Ordering;

    // @step Given a background session whose display pipeline emitted the turn-start seed for a 180000-token context with 150000 cache-read tokens
    let (_data_dir, _tmp, session) = create_background_session().await;
    let display = StreamingTokenDisplay::from_cache_inclusive_total(180_000, 500, 150_000, 0);
    // The seed emit lands in cached_input_tokens exactly as the
    // StreamEvent::Tokens arm does: update_tokens(info.input_tokens, ...).
    let info: TokenInfo = display.current().into();
    session.update_tokens(info.input_tokens as u32, info.output_tokens as u32);

    // @step When overflow recovery starts compaction and snapshots the pre-compaction token count
    let snapshot = session.snapshot_pre_compaction_tokens();

    // @step Then the pre-compaction snapshot records 180000 tokens
    assert_eq!(
        snapshot, 180_000,
        "seed-window CompactionStarted snapshot must record the true context total"
    );
    assert_eq!(
        session.pre_compaction_tokens.load(Ordering::Acquire),
        180_000,
        "pre_compaction_tokens atomic must hold the snapshot value"
    );

    // @step And the snapshot never records the double-counted 330000 tokens
    assert_ne!(
        snapshot, 330_000,
        "snapshot must never be the cache-double-counted value"
    );
}

// ===========================================================================
// Scenario: Auto and manual compaction snapshots agree on the same basis
//           across both twins
// ===========================================================================

#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_and_manual_snapshots_agree_on_the_same_basis() {
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
    let agent_loop_src = read_workspace_source("codelet/agent-loop/src/background_output.rs");
    let agent_loop_arm =
        extract_brace_block_after(&agent_loop_src, "StreamEvent::CompactionStarted =>");
    assert!(
        agent_loop_arm.contains("snapshot_pre_compaction_tokens()"),
        "agent-loop twin CompactionStarted arm must use the shared accessor; arm was:\n{agent_loop_arm}"
    );
    let napi_src = read_workspace_source("codelet/napi/src/agent_loop.rs");
    let napi_arm = extract_brace_block_after(&napi_src, "StreamEvent::CompactionStarted =>");
    assert!(
        napi_arm.contains("snapshot_pre_compaction_tokens()"),
        "NAPI twin CompactionStarted arm must use the shared accessor; arm was:\n{napi_arm}"
    );
}

/// SUPPLEMENTARY STRUCTURAL GUARD — NOT a Gherkin scenario test.
///
/// The behavioral parity test above
/// (`auto_and_manual_snapshots_agree_on_the_same_basis`, gated behind the
/// `test-support` feature) carries the @step coverage for the "Auto and
/// manual compaction snapshots agree on the same basis across both twins"
/// scenario. This guard has no @step comments on purpose: it is runnable
/// without the `test-support` feature and pins only the wiring shape — all
/// four `pre_compaction_tokens` writers must route through the shared
/// `BackgroundSession` accessors so basis drift is structurally impossible
/// across the NAPI/agent-loop twins.
#[test]
fn all_four_pre_compaction_writers_route_through_shared_accessors() {
    let sessions_src = read_workspace_source("codelet/sessions/src/background_session.rs");
    assert!(
        sessions_src.contains("pub fn snapshot_pre_compaction_tokens"),
        "BackgroundSession must expose snapshot_pre_compaction_tokens (AUTO accessor)"
    );
    assert!(
        sessions_src.contains("pub fn store_pre_compaction_tokens"),
        "BackgroundSession must expose store_pre_compaction_tokens (manual accessor)"
    );

    let agent_loop_src = read_workspace_source("codelet/agent-loop/src/background_output.rs");
    let agent_loop_arm =
        extract_brace_block_after(&agent_loop_src, "StreamEvent::CompactionStarted =>");
    let napi_src = read_workspace_source("codelet/napi/src/agent_loop.rs");
    let napi_arm = extract_brace_block_after(&napi_src, "StreamEvent::CompactionStarted =>");

    let handle_impl_src = read_workspace_source("codelet/sessions/src/handle_impl.rs");
    let session_bindings_src = read_workspace_source("codelet/napi/src/session_bindings.rs");

    for (twin, arm) in [("agent-loop", agent_loop_arm), ("napi", napi_arm)] {
        assert!(
            arm.contains("snapshot_pre_compaction_tokens()"),
            "[{twin}] AUTO CompactionStarted arm must call the shared \
             snapshot_pre_compaction_tokens() accessor; arm was:\n{arm}"
        );
        assert!(
            !arm.contains("pre_compaction_tokens") || !arm.contains(".store("),
            "[{twin}] AUTO CompactionStarted arm must not hand-roll the \
             load/store snapshot; arm was:\n{arm}"
        );
    }

    assert!(
        handle_impl_src.contains("store_pre_compaction_tokens("),
        "manual sessions RPC writer (handle_impl.rs) must route through \
         store_pre_compaction_tokens()"
    );
    assert!(
        session_bindings_src.contains("store_pre_compaction_tokens("),
        "manual NAPI writer (session_bindings.rs) must route through \
         store_pre_compaction_tokens()"
    );
}
