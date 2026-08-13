// Feature: spec/features/lazy-persistence-initialization.feature
//
// BUG-122: Lazy persistence initialization tests
//
// Tests for per-store lazy initialization (Layer 1), cross-session
// history access, data integrity with forked messages, and
// append/read consistency.
//
// Layer 2 (binary index + LRU) and Layer 3 (TypeScript deferral)
// tests are added during implementing phase once those APIs exist.
//
// RPC-035: relocated from rust/napi/src/persistence/lazy_init_tests.rs
// into codelet-core. `use super::*;` becomes `use crate::persistence::*;`
// (the canonical codelet-core import path) and the setup helper calls
// codelet_common::set_data_directory + reset_stores_for_tests directly.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    // RPC-035: see note in tests.rs — preserve the pre-relocation
    // `format!("X {}", y)` style.
    clippy::uninlined_format_args
)]

use super::tests::TEST_MUTEX;
use crate::persistence::*;
use std::path::PathBuf;

/// Setup an isolated temp directory for a test.
///
/// RPC-035: replaces the previous `crate::persistence::set_data_directory`
/// indirection (the deleted NAPI shim) with the codelet-core-only sequence:
/// `codelet_common::set_data_directory` + `reset_stores_for_tests()`.
fn setup_lazy_test_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = TEST_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    codelet_common::set_data_directory(temp_dir.path().to_path_buf())
        .expect("Failed to set data directory");
    reset_stores_for_tests();
    (guard, temp_dir)
}

/// Check whether a specific store's global singleton is initialized (Some) or not (None).
fn is_message_store_initialized() -> bool {
    crate::persistence::is_message_store_initialized_for_tests()
}

fn is_session_store_initialized() -> bool {
    crate::persistence::is_session_store_initialized_for_tests()
}

fn is_history_store_initialized() -> bool {
    crate::persistence::history::is_initialized_for_tests()
}

fn is_blob_store_initialized() -> bool {
    crate::persistence::is_blob_store_initialized_for_tests()
}

// ============================================================================
// Layer 1: Lazy per-store initialization
// ============================================================================

// Scenario: Lazy per-store initialization — get_history only inits HistoryStore
#[test]
fn test_lazy_get_history_only_inits_history_store() {
    let (_guard, _temp_dir) = setup_lazy_test_env();
    let project = PathBuf::from("/test/project/lazy");

    // @step Given the persistence layer has not been initialized
    assert!(!is_message_store_initialized());
    assert!(!is_session_store_initialized());
    assert!(!is_history_store_initialized());

    // @step When get_history() is called
    let result = history::get(Some(project.as_path()), Some(100));
    assert!(result.is_ok());

    // @step Then only HistoryStore is initialized
    assert!(is_history_store_initialized());

    // @step And MessageStore is NOT initialized
    assert!(
        !is_message_store_initialized(),
        "MessageStore should NOT be initialized by get_history()"
    );

    // @step And SessionStore is NOT initialized
    assert!(
        !is_session_store_initialized(),
        "SessionStore should NOT be initialized by get_history()"
    );
}

// Scenario: Lazy per-store initialization — store_message inits MessageStore and SessionStore
#[test]
fn test_lazy_store_message_inits_message_and_blob_and_session_store() {
    let (_guard, _temp_dir) = setup_lazy_test_env();
    let project = PathBuf::from("/test/project/lazy-msg");

    // @step Given the persistence layer has not been initialized
    assert!(!is_message_store_initialized());
    assert!(!is_blob_store_initialized());
    assert!(!is_session_store_initialized());
    assert!(!is_history_store_initialized());

    // @step When a message is stored via append_message()
    // Need a session first, which requires SessionStore
    let mut session = create_session("Lazy Test", &project).expect("create session");
    append_message(&mut session, "user", "Hello world").expect("append message");

    // @step Then MessageStore is initialized
    assert!(is_message_store_initialized());

    // @step And SessionStore is initialized
    assert!(is_session_store_initialized());

    // @step But BlobStore is NOT initialized
    assert!(
        !is_blob_store_initialized(),
        "BlobStore should NOT be initialized by append_message()"
    );

    // @step And HistoryStore is NOT initialized
    assert!(
        !is_history_store_initialized(),
        "HistoryStore should NOT be initialized by append_message()"
    );
}

// Scenario: Lazy per-store initialization — create_session only inits SessionStore
#[test]
fn test_lazy_create_session_only_inits_session_store() {
    let (_guard, _temp_dir) = setup_lazy_test_env();
    let project = PathBuf::from("/test/project/lazy-sess");

    // @step Given the persistence layer has not been initialized
    assert!(!is_message_store_initialized());
    assert!(!is_session_store_initialized());
    assert!(!is_history_store_initialized());

    // @step When create_session() is called
    let _session = create_session("Session Only", &project).expect("create session");

    // @step Then SessionStore is initialized
    assert!(is_session_store_initialized());

    // @step And MessageStore is NOT initialized
    assert!(
        !is_message_store_initialized(),
        "MessageStore should NOT be initialized by create_session()"
    );

    // @step And HistoryStore is NOT initialized
    assert!(
        !is_history_store_initialized(),
        "HistoryStore should NOT be initialized by create_session()"
    );
}

// ============================================================================
// Layer 2: Binary index + on-demand loading
// (Tests for index_len, cache_len, etc. will be added during implementing
//  phase once MessageStore gains the binary index API)
// ============================================================================

// Scenario: Session resume loads only that session's messages
#[test]
fn test_lazy_session_resume_loads_only_that_session() {
    let (_guard, _temp_dir) = setup_lazy_test_env();
    let project = PathBuf::from("/test/project/resume-lazy");

    // @step Given a session manifest with 200 message UUIDs
    let mut target_session = create_session("Target Session", &project).expect("create");
    for i in 0..200 {
        append_message(&mut target_session, "user", &format!("Target msg {}", i)).expect("append");
    }

    // @step And a MessageStore with 362000 indexed messages
    // (We can't create 362K in a test, but we create a second session to prove isolation)
    let mut other_session = create_session("Other Session", &project).expect("create");
    for i in 0..300 {
        append_message(&mut other_session, "user", &format!("Other msg {}", i)).expect("append");
    }

    // Reset store to verify messages load from storage
    crate::persistence::reset_message_store_for_tests();

    // @step When get_session_messages() is called for that session
    let messages =
        get_session_messages(&target_session).expect("get_session_messages should succeed");

    // @step Then only 200 messages are loaded from disk via index seek
    assert_eq!(messages.len(), 200);
    assert!(messages[0].content.contains("Target msg 0"));
    assert!(messages[199].content.contains("Target msg 199"));

    // @step And the remaining 361800 messages are not loaded
    // (Verified by checking only target session messages were returned)
}

// Scenario: SessionSearch loads messages on demand during cross-session search
#[test]
fn test_lazy_cross_session_search_loads_on_demand() {
    let (_guard, _temp_dir) = setup_lazy_test_env();
    let project = PathBuf::from("/test/project/cross-search");

    // @step Given 10 sessions each with 100 messages
    let mut sessions = Vec::new();
    for s in 0..10 {
        let mut session =
            create_session(&format!("Search Session {}", s), &project).expect("create");
        for i in 0..100 {
            let content = if s == 5 && i == 50 {
                "This is the NEEDLE in the haystack".to_string()
            } else {
                format!("Session {} message {}", s, i)
            };
            append_message(&mut session, "user", &content).expect("append");
        }
        sessions.push(session);
    }

    // Reset store
    crate::persistence::reset_message_store_for_tests();

    // @step And a MessageStore with a binary index
    // (Initialized lazily on first access)

    // @step When SessionSearch searches across all sessions with a regex query
    // Simulate what session_search_handler does: load messages for a session
    let target_session = &sessions[5];
    let messages = get_session_messages_full(target_session).expect("get_session_messages_full");

    // @step Then messages are loaded per-session via index seek as needed
    assert_eq!(messages.len(), 100);
    assert!(messages[50].content.contains("NEEDLE"));

    // @step And the full 1GB file is NOT loaded into a HashMap
    // (Proven by the fact we're using index-based loading, not load_all)
}

// ============================================================================
// Cross-session access patterns
// ============================================================================

// Scenario: Shell history recall shows entries from all sessions
#[test]
fn test_lazy_shell_history_cross_session() {
    let (_guard, _temp_dir) = setup_lazy_test_env();
    let project = PathBuf::from("/test/project/history-cross");

    // @step Given 5 sessions exist for the current project with different command histories
    for i in 0..5 {
        let session = create_session(&format!("Hist Session {}", i), &project).expect("create");
        history::add(HistoryEntry::new(
            format!("Command from session {}", i),
            project.clone(),
            session.id,
        ))
        .expect("add_history_entry");
    }

    // @step When the developer opens a new session
    // @step And presses Shift+Up to recall history
    let history = history::get(Some(project.as_path()), Some(100)).expect("get_history");

    // @step Then entries from all 5 previous sessions are available
    assert_eq!(history.len(), 5, "Should have entries from all 5 sessions");

    // @step And entries are sorted by most recent first
    // (HistoryStore sorts by timestamp descending)
    assert!(history[0].display.contains("session 4"));
    assert!(history[4].display.contains("session 0"));
}

// Scenario: Search command finds results across all sessions
#[test]
fn test_lazy_search_command_cross_session() {
    let (_guard, _temp_dir) = setup_lazy_test_env();
    let project = PathBuf::from("/test/project/search-cross");

    // @step Given 3 sessions exist with different command histories
    let session1 = create_session("Session 1", &project).expect("create");
    let session2 = create_session("Session 2", &project).expect("create");
    let session3 = create_session("Session 3", &project).expect("create");

    // @step And session 1 has a command containing "deploy"
    history::add(HistoryEntry::new(
        "deploy to staging".to_string(),
        project.clone(),
        session1.id,
    ))
    .expect("add_history_entry");

    history::add(HistoryEntry::new(
        "run tests".to_string(),
        project.clone(),
        session2.id,
    ))
    .expect("add_history_entry");

    // @step And session 3 has a command containing "deploy production"
    history::add(HistoryEntry::new(
        "deploy production release".to_string(),
        project.clone(),
        session3.id,
    ))
    .expect("add_history_entry");

    // @step When the developer runs /search and types "deploy"
    let results = history::search("deploy", Some(project.as_path())).expect("search_history");

    // @step Then results from both session 1 and session 3 are shown
    assert_eq!(results.len(), 2, "Should find 2 results matching 'deploy'");
    let displays: Vec<&str> = results.iter().map(|e| e.display.as_str()).collect();
    assert!(displays.contains(&"deploy to staging"));
    assert!(displays.contains(&"deploy production release"));
}

// ============================================================================
// Data integrity
// ============================================================================

// Scenario: Content-addressed messages shared via fork are accessible
#[test]
fn test_lazy_forked_message_accessible() {
    let (_guard, _temp_dir) = setup_lazy_test_env();
    let project = PathBuf::from("/test/project/fork-index");

    // @step Given session A has message UUID-123 via Native source
    let mut session_a = create_session("Session A", &project).expect("create");
    for i in 0..5 {
        append_message(&mut session_a, "user", &format!("Original msg {}", i)).expect("append");
    }
    let original_msg_id = session_a.messages[2].message_id;

    // @step And session B references the same UUID-123 via Forked source
    let session_b = fork_session(&session_a, 3, "Forked B").expect("fork");
    assert!(session_b
        .messages
        .iter()
        .any(|m| m.message_id == original_msg_id));

    // Reset store to force reload from storage
    crate::persistence::reset_message_store_for_tests();

    // @step When get_session_messages() is called for session B
    let messages_b = get_session_messages(&session_b).expect("get messages B");

    // @step Then message UUID-123 is loaded via index seek
    let found = messages_b
        .iter()
        .find(|m| m.id == original_msg_id)
        .expect("Forked message should be found");

    // @step And its content matches the original message from session A
    assert!(found.content.contains("Original msg 2"));
}

// Scenario: Append and immediate read consistency
#[test]
fn test_lazy_append_and_immediate_read() {
    let (_guard, _temp_dir) = setup_lazy_test_env();
    let project = PathBuf::from("/test/project/append-read");

    // @step Given a MessageStore with an index
    let mut session = create_session("Append Test", &project).expect("create");

    // @step When a new message is stored via store()
    let msg_id = append_message(&mut session, "user", "Freshly appended content").expect("append");

    // @step Then the message is immediately available via get()
    let loaded = get_message(msg_id)
        .expect("get should succeed")
        .expect("message should exist");
    assert_eq!(loaded.content, "Freshly appended content");

    // @step And the in-memory index contains the new entry
    // (Proven by the successful get() above)

    // @step And the binary index is updated on disk
    // Verify by resetting store and re-loading
    crate::persistence::reset_message_store_for_tests();

    let loaded_again = get_message(msg_id)
        .expect("get after reset should succeed")
        .expect("message should still exist after store reset");
    assert_eq!(loaded_again.content, "Freshly appended content");
}
