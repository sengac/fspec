//! Feature: spec/features/rust-tui-compact-real-compaction.feature
use super::*;

// BUG-516: the scenario "Compacting a populated session runs the compactor
// agent and replaces the conversation with its summary" needs a live agent
// loop and compactor agent; it is tested in
// rust/agent-loop/tests/bug516_manual_compaction_uses_compactor_agent.rs.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_empty_returns_error_and_leaves_untouched() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");
    // The persistence stores cache their directory; drop them so this test
    // writes under its own temp dir, not a previous test's deleted one.
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );

    // @step Given a brand-new session with no messages
    let sid = fresh_session(&manager).await;
    // Force an empty conversation (a fresh session may carry context reminders).
    let baseline_len = {
        let session = manager.get_session(&sid.value).expect("session must exist");
        let mut inner = session.inner.lock().await;
        inner.messages.clear();
        inner.token_tracker.input_tokens = 0;
        inner.messages.len()
    };
    assert_eq!(baseline_len, 0, "precondition: empty conversation");

    // @step When I compact the session through the handle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.compact_session(&sid);

    // @step Then the handle returns an error containing "Nothing to compact"
    let err = result.expect_err("expected Err on empty session");
    assert!(
        err.contains("Nothing to compact"),
        "expected 'Nothing to compact' in err message, got: {err}"
    );

    // @step And the session message count stays at zero
    let session = manager.get_session(&sid.value).expect("session must exist");
    let inner = session.inner.lock().await;
    assert_eq!(
        inner.messages.len(),
        0,
        "empty session must be left untouched (message count stays at zero)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_populated_reports_real_token_counts() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");
    // The persistence stores cache their directory; drop them so this test
    // writes under its own temp dir, not a previous test's deleted one.
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );

    // @step Given a session with several user and assistant messages
    // @step Given a populated session with several user and assistant messages
    let sid = fresh_session(&manager).await;
    let seeded = seed_populated(&manager, &sid).await;
    assert!(
        seeded >= 5,
        "expected several seeded messages, got {seeded}"
    );

    // @step When I compact the session through the handle
    // @step When the session is compacted through the session manager handle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.compact_session(&sid);
    let compaction = result.expect("compact_session must succeed");

    // @step Then the returned CompactionResult original_tokens is greater than zero
    assert!(
        compaction.original_tokens > 0,
        "expected original_tokens > 0, got {}",
        compaction.original_tokens
    );

    // RPC-421: the old assertion `compacted_tokens < original_tokens` locked
    // the premature trough measurement in as passing behaviour. The result is
    // now an acknowledgement — final numbers ship on the CompactionComplete
    // chunk (CMPCT-038 apply-site emission), not on the RPC return.

    // @step And the returned CompactionResult compacted_tokens is the acknowledgement sentinel 0
    // @step And the returned CompactionResult compacted_tokens is exactly 0
    assert_eq!(
        compaction.compacted_tokens, 0,
        "compacted_tokens must be the acknowledgement sentinel 0 (the DAG \
         summary does not exist yet at RPC-return time), got {}",
        compaction.compacted_tokens
    );

    // @step And the returned CompactionResult compression_ratio is the acknowledgement sentinel 0.0
    // @step And the returned CompactionResult compression_ratio is exactly 0.0
    assert_eq!(
        compaction.compression_ratio, 0.0,
        "compression_ratio must be the acknowledgement sentinel 0.0, never a \
         fabricated trough-measured reduction, got {}",
        compaction.compression_ratio
    );

    // RPC-421: the acknowledgement also reports no summarized/kept turns
    // (relocated from the retired codelet-napi rpc421_honest_ack_test).
    // @step And the returned CompactionResult turns_summarized and turns_kept are 0
    assert_eq!(compaction.turns_summarized, 0);
    assert_eq!(compaction.turns_kept, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_unknown_session_id_returns_not_found() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir).expect("set the data directory");
    // The persistence stores cache their directory; drop them so this test
    // writes under its own temp dir, not a previous test's deleted one.
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );

    // @step Given a session id that does not exist
    let unknown = SessionId::from("00000000-0000-0000-0000-000000000000".to_string());

    // @step When I compact the session through the handle
    let handle: &dyn SessionManagerHandle = &*manager;
    let result = handle.compact_session(&unknown);

    // @step Then the handle returns an error beginning with "Session not found"
    let err = result.expect_err("expected Err on unknown session id");
    assert!(
        err.starts_with("Session not found"),
        "expected err to begin with 'Session not found', got: {err}"
    );

    // Sanity: status of an unknown session is Idle (no side effects).
    let _ = SessionStatus::Idle;
}
