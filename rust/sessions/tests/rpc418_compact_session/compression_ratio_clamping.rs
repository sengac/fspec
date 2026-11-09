//! Feature: spec/features/compression-ratio-clamping.feature
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compact_tiny_session_never_ships_a_negative_ratio() {
    let _guard = DATA_DIR_GUARD.lock().await;
    let data_dir =
        codelet_test_helpers::shared_temp_subdirectory().expect("shared temp subdirectory");
    codelet_test_helpers::isolated_user_directory().expect("BUG-640: isolate the user directory");
    codelet_common::set_data_directory(data_dir.clone()).expect("set the data directory");
    codelet_core::persistence::reset_stores_for_tests();
    let manager = SessionManager::new();
    // BUG-437: sessions need a start workspace.
    assert_eq!(
        codelet_test_helpers::shared_temp_subdirectory()
            .and_then(|workspace| manager.set_default_project(workspace)),
        Ok(())
    );

    // @step Given a tiny session with a 3-token original context
    let sid = fresh_session(&manager).await;
    let session = manager.get_session(&sid.value).expect("session must exist");
    {
        let mut inner = session.inner.lock().await;
        inner.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text("hi")),
        });
        inner.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::text("hello")),
        });
        inner.token_tracker.input_tokens = 3;
        inner.token_tracker.output_tokens = 1;
    }

    // @step When the user compacts the session through the session manager handle
    let handle: &dyn SessionManagerHandle = &*manager;
    let compaction = handle
        .compact_session(&sid)
        .expect("compact_session must succeed");
    assert_eq!(compaction.original_tokens, 3);

    // @step Then the returned CompactionResult compression_ratio is exactly 0.0
    assert_eq!(compaction.compression_ratio, 0.0);

    // @step And the returned CompactionResult compression_ratio is not negative
    assert!(
        !compaction.compression_ratio.is_sign_negative(),
        "compression_ratio must not be negative (not even -0.0), got {}",
        compaction.compression_ratio
    );
}
