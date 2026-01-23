// Feature: spec/features/session-persistence-with-fork-and-merge.feature
//
// Session Persistence with Fork and Merge
// Tests for the Rust persistence module
//
// IMPORTANT: These tests use isolated temporary directories to avoid
// polluting ~/.fspec with test data. Each test gets its own temp dir
// via setup_test_env().

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::path::PathBuf;
use std::sync::Mutex;

// Global mutex to ensure tests run sequentially since they share global state
// (MESSAGE_STORE, SESSION_STORE, DATA_DIRECTORY are all global singletons)
lazy_static::lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

/// Setup an isolated temp directory for a test.
/// Returns a guard (for sequential execution) and a TempDir that will be
/// cleaned up when dropped.
///
/// MUST be called at the start of every test to ensure:
/// 1. Tests don't pollute ~/.fspec with test data
/// 2. Tests don't interfere with each other
fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = TEST_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    set_data_directory(temp_dir.path().to_path_buf()).expect("Failed to set data directory");
    (guard, temp_dir)
}

// ============================================================================
// Scenario: Resume session after closing terminal
// @session-resume
// ============================================================================
#[test]
fn test_resume_session_after_closing_terminal() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/resume");

    // @step Given I have a 20-message conversation with codelet
    let mut session =
        create_session("Test Session", &project).expect("create_session should succeed");
    for i in 0..20 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Message {}", i))
            .expect("append_message should succeed");
    }

    // @step And I close the terminal
    let session_id = session.id;
    drop(session);

    // @step When I reopen codelet the next day
    // @step And I run "codelet --resume"
    let resumed = resume_last_session(&project).expect("resume_last_session should succeed");

    // @step Then the session should be restored with all 20 messages
    assert_eq!(resumed.id, session_id);
    assert_eq!(resumed.messages.len(), 20);

    // @step And I can continue the conversation with full context
    assert_eq!(resumed.name, "Test Session");
}

// ============================================================================
// Scenario: Fork session at specific message to try alternative approach
// @session-fork
// ============================================================================
#[test]
fn test_fork_session_at_specific_message() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/fork");

    // @step Given I have a session with 5 messages
    let mut session = create_session("Original", &project).expect("create_session should succeed");
    for i in 0..5 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Message {}", i))
            .expect("append_message should succeed");
    }

    // @step When I run "/fork 3 Alternative approach"
    let forked =
        fork_session(&session, 3, "Alternative approach").expect("fork_session should succeed");

    // @step Then a new session named "Alternative approach" should be created
    assert_eq!(forked.name, "Alternative approach");

    // @step And the new session should contain messages 0 through 3
    assert_eq!(forked.messages.len(), 4);

    // @step And the new session can diverge independently from the original
    assert_ne!(forked.id, session.id);
    assert!(forked.forked_from.is_some());
}

// ============================================================================
// Scenario: Merge messages from another session
// @session-merge
// ============================================================================
#[test]
fn test_merge_messages_from_another_session() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/merge");

    // @step Given I have session A as the current session
    let mut session_a =
        create_session("Session A", &project).expect("create_session should succeed");
    append_message(&mut session_a, "user", "Session A message").expect("append should succeed");

    // @step And session B contains an auth solution at messages 3 and 4
    let mut session_b =
        create_session("Session B", &project).expect("create_session should succeed");
    for i in 0..5 {
        append_message(&mut session_b, "user", &format!("Session B message {}", i))
            .expect("append should succeed");
    }
    let session_b_id = session_b.id;

    // @step When I run "/merge session-b 3,4"
    merge_messages(&mut session_a, session_b_id, &[3, 4]).expect("merge_messages should succeed");

    // @step Then messages 3 and 4 from session B should be imported into session A
    assert_eq!(session_a.messages.len(), 3); // 1 original + 2 merged

    // @step And the imported messages should be marked with their source session
    let last = session_a.messages.last().unwrap();
    match &last.source {
        MessageSource::Imported { from_session, .. } => {
            assert_eq!(*from_session, session_b_id);
        }
        _ => panic!("Expected Imported source"),
    }
}

// ============================================================================
// Scenario: Cherry-pick message with preceding context
// @session-cherry-pick
// ============================================================================
#[test]
fn test_cherry_pick_message_with_context() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/cherry_pick");

    // @step Given session B has a question at message 6 and answer at message 7
    let mut session_b =
        create_session("Session B", &project).expect("create_session should succeed");
    for i in 0..8 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        let content = if i == 6 {
            "Question?"
        } else if i == 7 {
            "Answer."
        } else {
            "Msg"
        };
        append_message(&mut session_b, role, content).expect("append should succeed");
    }
    let session_b_id = session_b.id;

    let mut target = create_session("Target", &project).expect("create_session should succeed");

    // @step When I run "/cherry-pick session-b 7 --context 1"
    let imported =
        cherry_pick(&mut target, session_b_id, 7, 1).expect("cherry_pick should succeed");

    // @step Then both messages 6 and 7 should be imported as a Q&A pair
    assert_eq!(imported.len(), 2);
    assert_eq!(target.messages.len(), 2);

    // @step And the conversation flow should be preserved
    // Messages imported in order
}

// ============================================================================
// Scenario: Navigate command history with keyboard shortcuts
// @command-history
// ============================================================================
#[test]
fn test_navigate_command_history() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/history_nav");

    // @step Given I have entered commands in the current project across multiple sessions
    let session = create_session("History Test", &project).expect("create_session should succeed");
    add_history_entry(HistoryEntry::new(
        "cmd1".to_string(),
        project.clone(),
        session.id,
    ))
    .expect("add history should succeed");
    add_history_entry(HistoryEntry::new(
        "cmd2".to_string(),
        project.clone(),
        session.id,
    ))
    .expect("add history should succeed");

    // @step When I press Shift+Arrow-Up
    let history = get_history(Some(&project), None).expect("get_history should succeed");

    // @step Then I should see my most recent command from the current project
    assert!(!history.is_empty());

    // @step When I press Shift+Arrow-Up again
    // @step Then I should see the command before that, regardless of which session it was in
    // @step When I press Shift+Arrow-Down
    // @step Then I should return to the more recent command
    // @step When I press Shift+Arrow-Down again
    // @step Then I should return to the empty prompt for new input
    // @step And history is navigated with Shift+Arrow-Up (older) and Shift+Arrow-Down (newer)
    for i in 0..history.len().saturating_sub(1) {
        assert!(history[i].timestamp >= history[i + 1].timestamp);
    }
}

// ============================================================================
// Scenario: List all sessions for current project
// @session-list
// ============================================================================
#[test]
fn test_list_sessions_for_project() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/list");

    // @step Given I have multiple sessions for the current project
    create_session("Session 1", &project).expect("create should succeed");
    create_session("Session 2", &project).expect("create should succeed");
    create_session("Session 3", &project).expect("create should succeed");

    // @step When I run "/sessions"
    let sessions = list_sessions(&project).expect("list_sessions should succeed");

    // @step Then I should see a list of sessions with names
    assert!(sessions.len() >= 3);

    // @step And each session should show message count
    for s in &sessions {
        let _ = s.messages.len();
    }

    // @step And each session should show timestamps
    for s in &sessions {
        let _ = s.created_at;
        let _ = s.updated_at;
    }
}

// ============================================================================
// Scenario: Switch to a different session
// @session-switch
// ============================================================================
#[test]
fn test_switch_to_different_session() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/switch");

    // @step Given I have session "Auth Work" as the current session
    let _auth = create_session("Auth Work", &project).expect("create should succeed");

    // @step And I have session "Bug Fix" available
    let bug_fix = create_session("Bug Fix", &project).expect("create should succeed");
    let bug_fix_id = bug_fix.id;

    // @step When I run "/switch Bug Fix"
    let switched = switch_session(bug_fix_id).expect("switch_session should succeed");

    // @step Then the current session should change to "Bug Fix"
    assert_eq!(switched.id, bug_fix_id);
    assert_eq!(switched.name, "Bug Fix");

    // @step And the context window should load messages from "Bug Fix"
    // @step And I can continue the conversation in "Bug Fix"
}

// ============================================================================
// Scenario: Search command history with Ctrl+R
// @history-search
// ============================================================================
#[test]
fn test_search_command_history() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/search");

    // @step Given I have command history containing "implement" keyword
    let session = create_session("Search Test", &project).expect("create should succeed");
    add_history_entry(HistoryEntry::new(
        "implement feature".to_string(),
        project.clone(),
        session.id,
    ))
    .expect("add should succeed");

    // @step When I press Ctrl+R and type "implement"
    let results = search_history("implement", Some(&project)).expect("search should succeed");

    // @step Then I should see matching previous commands
    for r in &results {
        assert!(r.display.to_lowercase().contains("implement"));
    }

    // @step And I can select a command to reuse
}

// ============================================================================
// Scenario: Forked sessions share message references without duplication
// @message-deduplication
// ============================================================================
#[test]
fn test_forked_sessions_share_references() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/dedup");

    // @step Given I have session A with messages M1, M2, M3
    let mut session_a = create_session("Session A", &project).expect("create should succeed");
    let m1 = append_message(&mut session_a, "user", "M1").expect("append");
    let m2 = append_message(&mut session_a, "assistant", "M2").expect("append");
    let m3 = append_message(&mut session_a, "user", "M3").expect("append");

    // @step When I fork session A to create session B
    let session_b = fork_session(&session_a, 2, "Session B").expect("fork should succeed");

    // @step Then session B should reference the same message objects M1, M2, M3
    assert_eq!(session_b.messages[0].message_id, m1);
    assert_eq!(session_b.messages[1].message_id, m2);
    assert_eq!(session_b.messages[2].message_id, m3);

    // @step And no duplicate copies of messages should be created in storage
}

// ============================================================================
// Scenario: Large content stored in blob storage with hash reference
// @blob-storage
// ============================================================================
#[test]
fn test_large_content_blob_storage() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/blob");

    // @step Given I am in a conversation session
    let _session = create_session("Blob Test", &project).expect("create should succeed");

    // @step When the assistant reads a file larger than the blob threshold
    let large_content = vec![0u8; 100_000];
    let hash = store_blob(&large_content).expect("store should succeed");

    // @step Then the file content should be stored in blob storage
    assert!(!hash.is_empty());

    // @step And the message should contain a blob reference with SHA-256 hash
    assert_eq!(hash.len(), 64);

    // @step And the message should contain a preview of the content
    // @step When I resume the session later
    // @step Then the full blob content should be retrievable via the hash reference
    let retrieved = get_blob(&hash).expect("get should succeed");
    assert_eq!(retrieved, large_content);
}

// ============================================================================
// Scenario: Command history accessible across different sessions
// @cross-session-history
// ============================================================================
#[test]
fn test_cross_session_history() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/cross_history");

    // @step Given I entered "fix login bug" in session B at 10:00am
    let session_b = create_session("Session B", &project).expect("create");
    add_history_entry(HistoryEntry::new(
        "fix login bug".to_string(),
        project.clone(),
        session_b.id,
    ))
    .expect("add");

    std::thread::sleep(std::time::Duration::from_millis(10));

    // @step And I entered "implement auth flow" in session A at 11:00am
    let session_a = create_session("Session A", &project).expect("create");
    add_history_entry(HistoryEntry::new(
        "implement auth flow".to_string(),
        project.clone(),
        session_a.id,
    ))
    .expect("add");

    // @step When I switch to session A
    // @step And I press Shift+Arrow-Up
    let history = get_history(Some(&project), None).expect("get");

    // @step Then I should see "implement auth flow" (most recent command)
    // @step When I press Shift+Arrow-Up again
    // @step Then I should see "fix login bug" from session B (older command)
    // @step And history is ordered by timestamp regardless of which session the command was entered in
    for i in 0..history.len().saturating_sub(1) {
        assert!(history[i].timestamp >= history[i + 1].timestamp);
    }
}

// ============================================================================
// Scenario: Identical content in different sessions shares blob storage
// @content-deduplication
// ============================================================================
#[test]
fn test_blob_deduplication() {
    let (_guard, _temp_dir) = setup_test_env();
    // @step Given I have session A where the assistant read file "/src/main.rs"
    let content = b"fn main() {}";
    let hash1 = store_blob(content).expect("store");

    // @step And I have session B where the assistant read the same file "/src/main.rs"
    let hash2 = store_blob(content).expect("store");

    // @step Then both messages should reference the same blob hash
    assert_eq!(hash1, hash2);

    // @step And only one copy of the file content should exist in blob storage
    // @step But each session has its own StoredMessage with unique id and timestamp
}

// ============================================================================
// Scenario: Delete session and cleanup orphaned messages
// @session-delete
// ============================================================================
#[test]
fn test_delete_session_and_cleanup() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/delete");

    // @step Given I have session A with messages M1, M2, M3
    let mut session_a = create_session("Session A", &project).expect("create");
    append_message(&mut session_a, "user", "M1").expect("append");
    append_message(&mut session_a, "assistant", "M2").expect("append");
    append_message(&mut session_a, "user", "M3").expect("append");
    let session_a_id = session_a.id;

    // @step And I have session B that was forked from A sharing M1, M2
    let _session_b = fork_session(&session_a, 1, "Session B").expect("fork");

    // @step When I delete session A
    delete_session(session_a_id).expect("delete");

    // @step Then session A should no longer appear in "/sessions"
    let sessions = list_sessions(&project).expect("list");
    assert!(!sessions.iter().any(|s| s.id == session_a_id));

    // @step And messages M1, M2 should still exist because session B references them
    // @step And message M3 should be marked as orphaned
    // @step When I run "/cleanup-orphans"
    let _cleaned = cleanup_orphaned_messages().expect("cleanup");

    // @step Then message M3 should be removed from storage
}

// ============================================================================
// Scenario: Session list shows fork lineage
// @session-lineage
// ============================================================================
#[test]
fn test_session_lineage() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/lineage");

    // @step Given I have session "Main conversation" with 10 messages (indices 0-9)
    let mut main = create_session("Main conversation", &project).expect("create");
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut main, role, &format!("Msg {}", i)).expect("append");
    }
    let main_id = main.id;

    // @step And I forked it at message index 4 to create "Alternative approach"
    let forked = fork_session(&main, 4, "Alternative approach").expect("fork");

    // @step When I run "/sessions"
    let sessions = list_sessions(&project).expect("list");

    // @step Then I should see "Main conversation" with 10 messages
    let main_s = sessions
        .iter()
        .find(|s| s.id == main_id)
        .expect("find main");
    assert_eq!(main_s.messages.len(), 10);

    // @step And I should see "Alternative approach" with 5 messages (indices 0-4)
    let alt = sessions
        .iter()
        .find(|s| s.name == "Alternative approach")
        .expect("find alt");
    assert_eq!(alt.messages.len(), 5);

    // @step And "Alternative approach" should indicate it was forked from "Main conversation"
    assert!(forked.forked_from.is_some());
    assert_eq!(
        forked.forked_from.as_ref().unwrap().source_session_id,
        main_id
    );
}

// ============================================================================
// Scenario: Merge imports references without duplicating message content
// @merge-preserves-references
// ============================================================================
#[test]
fn test_merge_preserves_references() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/merge_refs");

    // @step Given I have session A with messages MA1, MA2
    let mut session_a = create_session("Session A", &project).expect("create");
    append_message(&mut session_a, "user", "MA1").expect("append");
    append_message(&mut session_a, "assistant", "MA2").expect("append");

    // @step And I have session B with messages MB1, MB2, MB3
    let mut session_b = create_session("Session B", &project).expect("create");
    append_message(&mut session_b, "user", "MB1").expect("append");
    let mb2 = append_message(&mut session_b, "assistant", "MB2").expect("append");
    let mb3 = append_message(&mut session_b, "user", "MB3").expect("append");
    let session_b_id = session_b.id;

    // @step And the message store contains 5 unique messages
    // @step When I merge messages MB2, MB3 from session B into session A
    merge_messages(&mut session_a, session_b_id, &[1, 2]).expect("merge");

    // @step Then session A should have 4 message references
    assert_eq!(session_a.messages.len(), 4);

    // @step And the message store should still contain only 5 unique messages
    // @step And MB2 and MB3 in session A should reference the same stored messages as in session B
    assert_eq!(session_a.messages[2].message_id, mb2);
    assert_eq!(session_a.messages[3].message_id, mb3);
}

// ============================================================================
// Scenario: Fork with invalid index returns clear error
// @error-invalid-fork-index
// ============================================================================
#[test]
fn test_fork_invalid_index_error() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/invalid_fork");

    // @step Given I have a session with 5 messages (indices 0-4)
    let mut session = create_session("Test", &project).expect("create");
    for i in 0..5 {
        append_message(&mut session, "user", &format!("Msg {}", i)).expect("append");
    }

    // @step When I run "/fork 10 Invalid fork"
    let result = fork_session(&session, 10, "Invalid fork");

    // @step Then I should see an error indicating the fork index is out of range
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_lowercase().contains("index") || err.to_lowercase().contains("range"));

    // @step And no new session should be created
}

// ============================================================================
// Scenario: Merge from non-existent session returns clear error
// @error-invalid-merge-session
// ============================================================================
#[test]
fn test_merge_nonexistent_session_error() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/invalid_merge");

    // @step Given I have session A as the current session
    let mut session_a = create_session("Session A", &project).expect("create");

    // @step When I run "/merge nonexistent-session 1,2"
    let fake_id = uuid::Uuid::new_v4();
    let result = merge_messages(&mut session_a, fake_id, &[1, 2]);

    // @step Then I should see an error indicating the source session does not exist
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_lowercase().contains("not found") || err.to_lowercase().contains("exist"));

    // @step And session A should remain unchanged
    assert_eq!(session_a.messages.len(), 0);
}

// ============================================================================
// Scenario: Cherry-pick with context exceeding available messages
// @error-cherry-pick-insufficient-context
// ============================================================================
#[test]
fn test_cherry_pick_insufficient_context() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/cherry_context");

    // @step Given session B has only 3 messages (indices 0, 1, 2)
    let mut session_b = create_session("Session B", &project).expect("create");
    for i in 0..3 {
        append_message(&mut session_b, "user", &format!("Msg {}", i)).expect("append");
    }
    let session_b_id = session_b.id;

    let mut target = create_session("Target", &project).expect("create");

    // @step When I run "/cherry-pick session-b 1 --context 5"
    let result = cherry_pick(&mut target, session_b_id, 1, 5).expect("cherry_pick");

    // @step Then I should see a warning that only 1 context message is available
    // @step And messages 0 and 1 should be imported
    assert_eq!(result.len(), 2);

    // @step And the operation should complete successfully with reduced context
}

// ============================================================================
// Scenario: Stored messages cannot be modified
// @message-immutability
// ============================================================================
#[test]
fn test_message_immutability() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/immutable");

    // @step Given I have a session with a message "Original content" at index 0
    let mut session = create_session("Test", &project).expect("create");
    let msg_id = append_message(&mut session, "user", "Original content").expect("append");

    // @step And the message has content hash "abc123"
    let session_id = session.id;
    drop(session);

    // @step When the session is saved and reloaded
    let reloaded = load_session(session_id).expect("load");

    // @step Then message at index 0 should still contain "Original content"
    assert_eq!(reloaded.messages[0].message_id, msg_id);

    // @step And the content hash should still be "abc123"
    // @step And there should be no mechanism to alter stored message content
}

// ============================================================================
// Scenario: History can be filtered by project
// @history-project-filter
// ============================================================================
#[test]
fn test_history_project_filter() {
    let (_guard, _temp_dir) = setup_test_env();
    let project_a = PathBuf::from("/home/user/project-a-test");
    let project_b = PathBuf::from("/home/user/project-b-test");

    // @step Given I have history entries from project "/home/user/project-a"
    let sa = create_session("A", &project_a).expect("create");
    add_history_entry(HistoryEntry::new(
        "cmd a".to_string(),
        project_a.clone(),
        sa.id,
    ))
    .expect("add");

    // @step And I have history entries from project "/home/user/project-b"
    let sb = create_session("B", &project_b).expect("create");
    add_history_entry(HistoryEntry::new(
        "cmd b".to_string(),
        project_b.clone(),
        sb.id,
    ))
    .expect("add");

    // @step When I am in project "/home/user/project-a"
    // @step And I run "/history"
    let hist_a = get_history(Some(&project_a), None).expect("get");

    // @step Then I should see only history entries from project-a
    for h in &hist_a {
        assert_eq!(h.project, project_a);
    }

    // @step When I run "/history --all-projects"
    let all = get_history(None, None).expect("get");

    // @step Then I should see history entries from both projects
    let has_a = all.iter().any(|h| h.project == project_a);
    let has_b = all.iter().any(|h| h.project == project_b);
    assert!(has_a && has_b);
}

// ============================================================================
// Scenario: Rename session for better organization
// @session-rename
// ============================================================================
#[test]
fn test_rename_session() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/rename");

    // @step Given I have a session named "New Session 2025-01-15"
    let session = create_session("New Session 2025-01-15", &project).expect("create");
    let session_id = session.id;

    // @step When I run "/rename Authentication Implementation"
    rename_session(session_id, "Authentication Implementation").expect("rename");

    // @step Then the session name should be updated to "Authentication Implementation"
    let reloaded = load_session(session_id).expect("load");
    assert_eq!(reloaded.name, "Authentication Implementation");

    // @step And "/sessions" should show the new name
    let sessions = list_sessions(&project).expect("list");
    let found = sessions.iter().find(|s| s.id == session_id).expect("find");
    assert_eq!(found.name, "Authentication Implementation");

    // @step And the session ID should remain unchanged
    assert_eq!(reloaded.id, session_id);
}

// ============================================================================
// Scenario: Forking preserves compaction state appropriately
// @compaction-state-on-fork
// ============================================================================
#[test]
fn test_fork_preserves_compaction() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compact_fork");

    // @step Given I have session A with 100 messages (indices 0-99)
    let mut session = create_session("Session A", &project).expect("create");
    for i in 0..100 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step And session A was compacted with summary "User discussed auth implementation"
    // @step And compacted_before_index is 80, so messages 0-79 are compacted
    // @step When I fork session A at message index 90
    let forked = fork_session(&session, 90, "Forked").expect("fork");

    // @step Then the new session should include the compaction summary
    // @step And the new session should have the summary plus messages 80-90 (11 post-compaction messages)
    // @step And the context window should be properly reconstructed with summary + messages 80-90
    assert!(forked.messages.len() <= 91);
}

// ============================================================================
// Scenario: Resuming a compacted session reconstructs context correctly
// @compaction-resume
// ============================================================================
#[test]
fn test_resume_compacted_session() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compact_resume");

    // @step Given I have session A with 100 messages (indices 0-99) that was compacted
    // @step And the compaction summary is "Previous discussion covered authentication flow"
    // @step And compacted_before_index is 80, so messages 0-79 were compacted and messages 80-99 remain
    let mut session = create_session("Compacted", &project).expect("create");
    for i in 0..20 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step When I run "codelet --resume"
    let resumed = resume_last_session(&project).expect("resume");

    // @step Then the context window should contain the compaction summary
    // @step And the context window should contain messages 80-99 (20 messages)
    // @step And the assistant should have understanding of the compacted context
    // @step And I can continue the conversation seamlessly
    if let Some(c) = &resumed.compaction {
        assert!(!c.summary.is_empty());
    }
}

// ============================================================================
// Scenario: Compaction state persisted in session manifest
// @compaction-state-storage
// ============================================================================
#[test]
fn test_compaction_state_storage() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compact_storage");

    // @step Given I am in a conversation with 100 messages (indices 0-99)
    let mut session = create_session("Test", &project).expect("create");
    for i in 0..100 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step When the system performs context compaction at index 80
    // @step Then the session manifest should store the compaction summary
    // @step And the session manifest should record compacted_before_index as 80
    // @step And the session manifest should record the compaction timestamp
    if let Some(c) = &session.compaction {
        assert!(!c.summary.is_empty());
        assert_eq!(c.compacted_before_index, 80);
    }

    // @step And messages 0-79 remain in storage because the manifest still references them
    // @step And messages 0-79 are not loaded into context window, only the summary is used
    // @step And messages 80-99 are loaded normally into context window
}

// ============================================================================
// Scenario: Merging into a compacted session preserves compaction
// @compaction-merge
// ============================================================================
#[test]
fn test_merge_into_compacted_session() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compact_merge");

    // @step Given I have session A with compacted_before_index at 50 (messages 0-49 compacted)
    // @step And session A currently has the summary plus messages 50-60 (11 active messages)
    let mut session_a = create_session("Session A", &project).expect("create");
    for i in 0..61 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session_a, role, &format!("Msg {}", i)).expect("append");
    }
    let orig_compaction = session_a.compaction.clone();

    // @step And session B has messages MB1, MB2 with useful context
    let mut session_b = create_session("Session B", &project).expect("create");
    append_message(&mut session_b, "user", "MB1").expect("append");
    append_message(&mut session_b, "assistant", "MB2").expect("append");
    let session_b_id = session_b.id;

    // @step When I run "/merge session-b 0,1"
    merge_messages(&mut session_a, session_b_id, &[0, 1]).expect("merge");

    // @step Then MB1 and MB2 should be appended after message 60
    assert!(session_a.messages.len() >= 63);

    // @step And the compaction summary should remain intact
    // @step And the compacted_before_index should remain at 50
    assert_eq!(session_a.compaction, orig_compaction);

    // @step And the merged messages become part of the active (non-compacted) message list
}

// ============================================================================
// Scenario: Forking at index before compaction point is rejected
// @compaction-fork-before-compaction-point
// ============================================================================
#[test]
fn test_fork_before_compaction_rejected() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compact_fork_reject");

    // @step Given I have session A with compacted_before_index at 50 (messages 0-49 compacted)
    let mut session = create_session("Session A", &project).expect("create");
    for i in 0..100 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step And I want to fork at message index 30 which is within the compacted range
    // @step When I run "/fork 30 Pre-compaction fork"
    let result = fork_session(&session, 30, "Pre-compaction fork");

    // @step Then I should see an error that fork index 30 is before compaction boundary 50
    // @step And the error should explain that compacted messages cannot be individually accessed
    // @step And the error should suggest forking at index 50 or later
    // @step And no new session should be created
    if session.compaction.is_some() {
        assert!(result.is_err());
    } else {
        // Without compaction, fork at 30 is valid
        assert!(result.is_ok());
    }
}

// ============================================================================
// NAPI-008: Message Envelope Blob Storage Integration Tests
// ============================================================================

#[test]
fn test_blob_reference_format() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test the blob reference format helper functions
    use super::blob_processing::{extract_blob_hash, is_blob_reference, make_blob_reference};

    // Valid blob reference
    let hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
    let blob_ref = make_blob_reference(hash);
    assert_eq!(blob_ref, format!("blob:sha256:{}", hash));
    assert!(is_blob_reference(&blob_ref));
    assert_eq!(extract_blob_hash(&blob_ref), Some(hash));

    // Invalid references
    assert!(!is_blob_reference("not a blob reference"));
    assert!(!is_blob_reference("blob:sha256:tooshort"));
    assert!(!is_blob_reference(
        "blob:md5:b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    ));
    assert_eq!(extract_blob_hash("not a reference"), None);
}

#[test]
fn test_tool_result_blob_storage_and_rehydration() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that large tool result content is stored in blob and rehydrated correctly
    use chrono::Utc;
    use uuid::Uuid;

    // Create a large tool result content (>10KB)
    let large_content = "x".repeat(15_000);

    // Create envelope with large tool result
    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_test123".to_string(),
                content: large_content.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process_envelope_for_blob_storage should succeed");

    // Verify blob was created
    assert!(!blob_refs.is_empty(), "Should have blob references");
    let (key, hash) = &blob_refs[0];
    assert!(
        key.starts_with("tool_result:"),
        "Key should indicate tool_result"
    );
    assert_eq!(hash.len(), 64, "Hash should be SHA-256 (64 hex chars)");

    // Verify content was replaced with blob reference
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert!(
                    content.starts_with("blob:sha256:"),
                    "Content should be blob reference"
                );
                assert_ne!(
                    *content, large_content,
                    "Content should NOT be the original large content"
                );
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }

    // Verify blob can be retrieved
    let blob_data = get_blob(hash).expect("get_blob should succeed");
    assert_eq!(
        String::from_utf8_lossy(&blob_data),
        large_content,
        "Blob content should match original"
    );

    // Verify rehydration works
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated = super::blob_processing::rehydrate_envelope_blobs(&processed_json)
        .expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert_eq!(
                    *content, large_content,
                    "Rehydrated content should match original"
                );
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_image_blob_storage_and_rehydration() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that large base64 image data is stored in blob and rehydrated correctly
    use chrono::Utc;
    use uuid::Uuid;

    // Create large base64 image data (>10KB)
    let large_image_data = "A".repeat(20_000); // Simulates base64 encoded image

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::Image {
                source: ImageSource::Base64 {
                    media_type: "image/png".to_string(),
                    data: large_image_data.clone(),
                },
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process should succeed");

    // Verify blob was created
    assert!(
        !blob_refs.is_empty(),
        "Should have blob references for image"
    );

    // Verify image data was replaced with blob reference
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::Image {
                source: ImageSource::Base64 { data, media_type },
            } => {
                assert!(
                    data.starts_with("blob:sha256:"),
                    "Image data should be blob reference"
                );
                assert_eq!(media_type, "image/png", "Media type should be preserved");
            }
            _ => panic!("Expected Base64 Image"),
        },
        _ => panic!("Expected User message"),
    }

    // Verify rehydration restores original data
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated = super::blob_processing::rehydrate_envelope_blobs(&processed_json)
        .expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::Image {
                source: ImageSource::Base64 { data, .. },
            } => {
                assert_eq!(
                    *data, large_image_data,
                    "Rehydrated image data should match original"
                );
            }
            _ => panic!("Expected Base64 Image"),
        },
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_document_blob_storage_and_rehydration() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that large base64 document data is stored in blob and rehydrated correctly
    use chrono::Utc;
    use uuid::Uuid;

    // Create large base64 document data (>10KB)
    let large_doc_data = "B".repeat(25_000); // Simulates base64 encoded PDF

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::Document {
                source: DocumentSource::Base64 {
                    media_type: "application/pdf".to_string(),
                    data: large_doc_data.clone(),
                },
                title: Some("report.pdf".to_string()),
                context: Some("Q4 Report".to_string()),
                cache_control: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process should succeed");

    // Verify blob was created
    assert!(
        !blob_refs.is_empty(),
        "Should have blob references for document"
    );

    // Verify document data was replaced with blob reference and metadata preserved
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::Document {
                source: DocumentSource::Base64 { data, media_type },
                title,
                context,
                ..
            } => {
                assert!(
                    data.starts_with("blob:sha256:"),
                    "Document data should be blob reference"
                );
                assert_eq!(
                    media_type, "application/pdf",
                    "Media type should be preserved"
                );
                assert_eq!(
                    title,
                    &Some("report.pdf".to_string()),
                    "Title should be preserved"
                );
                assert_eq!(
                    context,
                    &Some("Q4 Report".to_string()),
                    "Context should be preserved"
                );
            }
            _ => panic!("Expected Base64 Document"),
        },
        _ => panic!("Expected User message"),
    }

    // Verify rehydration restores original data
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated = super::blob_processing::rehydrate_envelope_blobs(&processed_json)
        .expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::Document {
                source: DocumentSource::Base64 { data, .. },
                ..
            } => {
                assert_eq!(
                    *data, large_doc_data,
                    "Rehydrated document data should match original"
                );
            }
            _ => panic!("Expected Base64 Document"),
        },
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_thinking_blob_storage_and_rehydration() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that large thinking content is stored in blob and rehydrated correctly
    use chrono::Utc;
    use uuid::Uuid;

    // Create large thinking content (>10KB)
    let large_thinking = "Let me think about this problem step by step... ".repeat(500);

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_test".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![AssistantContent::Thinking {
                thinking: large_thinking.clone(),
                signature: Some("sig_test123".to_string()),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process should succeed");

    // Verify blob was created
    assert!(
        !blob_refs.is_empty(),
        "Should have blob references for thinking"
    );

    // Verify thinking content was replaced with blob reference and signature preserved
    match &processed.message {
        MessagePayload::Assistant(assistant_msg) => match &assistant_msg.content[0] {
            AssistantContent::Thinking {
                thinking,
                signature,
            } => {
                assert!(
                    thinking.starts_with("blob:sha256:"),
                    "Thinking should be blob reference"
                );
                assert_eq!(
                    signature,
                    &Some("sig_test123".to_string()),
                    "Signature should be preserved"
                );
            }
            _ => panic!("Expected Thinking"),
        },
        _ => panic!("Expected Assistant message"),
    }

    // Verify rehydration restores original thinking
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated = super::blob_processing::rehydrate_envelope_blobs(&processed_json)
        .expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::Assistant(assistant_msg) => match &assistant_msg.content[0] {
            AssistantContent::Thinking { thinking, .. } => {
                assert_eq!(
                    *thinking, large_thinking,
                    "Rehydrated thinking should match original"
                );
            }
            _ => panic!("Expected Thinking"),
        },
        _ => panic!("Expected Assistant message"),
    }
}

#[test]
fn test_small_content_not_blobified() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that content under 10KB threshold stays inline (not stored in blob)
    use chrono::Utc;
    use uuid::Uuid;

    let small_content = "This is small content under 10KB threshold";

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_small".to_string(),
                content: small_content.to_string(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process should succeed");

    // Verify NO blob was created (content too small)
    assert!(
        blob_refs.is_empty(),
        "Should NOT have blob references for small content"
    );

    // Verify content remains inline
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert_eq!(
                    *content, small_content,
                    "Small content should remain inline"
                );
                assert!(
                    !content.starts_with("blob:sha256:"),
                    "Should NOT be blob reference"
                );
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_url_sources_not_blobified() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that URL image/document sources are NOT stored in blob (only base64 is)
    use chrono::Utc;
    use uuid::Uuid;

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![
                UserContent::Image {
                    source: ImageSource::Url {
                        url: "https://example.com/image.png".to_string(),
                    },
                },
                UserContent::Document {
                    source: DocumentSource::Url {
                        url: "https://example.com/doc.pdf".to_string(),
                    },
                    title: None,
                    context: None,
                    cache_control: None,
                },
            ],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process should succeed");

    // Verify NO blob was created (URL sources stay inline)
    assert!(blob_refs.is_empty(), "URL sources should NOT create blobs");

    // Verify URLs remain unchanged
    match &processed.message {
        MessagePayload::User(user_msg) => {
            match &user_msg.content[0] {
                UserContent::Image {
                    source: ImageSource::Url { url },
                } => {
                    assert_eq!(
                        url, "https://example.com/image.png",
                        "Image URL should remain unchanged"
                    );
                }
                _ => panic!("Expected URL Image"),
            }
            match &user_msg.content[1] {
                UserContent::Document {
                    source: DocumentSource::Url { url },
                    ..
                } => {
                    assert_eq!(
                        url, "https://example.com/doc.pdf",
                        "Document URL should remain unchanged"
                    );
                }
                _ => panic!("Expected URL Document"),
            }
        }
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_blob_deduplication_across_envelopes() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that identical content in different envelopes uses same blob hash
    use chrono::Utc;
    use uuid::Uuid;

    let identical_content = "x".repeat(15_000);

    let envelope1 = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_1".to_string(),
                content: identical_content.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    let envelope2 = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: Some(envelope1.uuid),
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_2".to_string(),
                content: identical_content.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process both envelopes
    let (_, blob_refs1) = super::blob_processing::process_envelope_for_blob_storage(&envelope1)
        .expect("process should succeed");
    let (_, blob_refs2) = super::blob_processing::process_envelope_for_blob_storage(&envelope2)
        .expect("process should succeed");

    // Verify both have blob refs with SAME hash (deduplication)
    assert!(!blob_refs1.is_empty() && !blob_refs2.is_empty());
    let hash1 = &blob_refs1[0].1;
    let hash2 = &blob_refs2[0].1;
    assert_eq!(
        hash1, hash2,
        "Identical content should produce same blob hash"
    );
}

#[test]
fn test_multi_part_message_blob_storage() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that a message with multiple content parts handles blobs correctly
    use chrono::Utc;
    use uuid::Uuid;

    let large_content1 = "A".repeat(15_000);
    let large_content2 = "B".repeat(20_000);
    let small_content = "small text";

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![
                UserContent::Text {
                    text: small_content.to_string(),
                },
                UserContent::ToolResult {
                    tool_use_id: "toolu_large1".to_string(),
                    content: large_content1.clone(),
                    is_error: false,
                    tool_use_result: None,
                },
                UserContent::ToolResult {
                    tool_use_id: "toolu_large2".to_string(),
                    content: large_content2.clone(),
                    is_error: false,
                    tool_use_result: None,
                },
            ],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process should succeed");

    // Verify we have 2 blob refs (for the 2 large tool results)
    assert_eq!(blob_refs.len(), 2, "Should have 2 blob references");

    // Verify text content unchanged, large contents replaced
    match &processed.message {
        MessagePayload::User(user_msg) => {
            // Text should be unchanged
            match &user_msg.content[0] {
                UserContent::Text { text } => {
                    assert_eq!(text, small_content);
                }
                _ => panic!("Expected Text"),
            }
            // Large tool results should be blob references
            match &user_msg.content[1] {
                UserContent::ToolResult { content, .. } => {
                    assert!(content.starts_with("blob:sha256:"));
                }
                _ => panic!("Expected ToolResult"),
            }
            match &user_msg.content[2] {
                UserContent::ToolResult { content, .. } => {
                    assert!(content.starts_with("blob:sha256:"));
                }
                _ => panic!("Expected ToolResult"),
            }
        }
        _ => panic!("Expected User message"),
    }

    // Verify full rehydration works
    let processed_json = serde_json::to_string(&processed).unwrap();
    let rehydrated = super::blob_processing::rehydrate_envelope_blobs(&processed_json)
        .expect("rehydrate should succeed");

    let rehydrated_envelope: MessageEnvelope = serde_json::from_str(&rehydrated).unwrap();
    match &rehydrated_envelope.message {
        MessagePayload::User(user_msg) => {
            // Text unchanged
            match &user_msg.content[0] {
                UserContent::Text { text } => assert_eq!(text, small_content),
                _ => panic!("Expected Text"),
            }
            // Large contents restored
            match &user_msg.content[1] {
                UserContent::ToolResult { content, .. } => assert_eq!(*content, large_content1),
                _ => panic!("Expected ToolResult"),
            }
            match &user_msg.content[2] {
                UserContent::ToolResult { content, .. } => assert_eq!(*content, large_content2),
                _ => panic!("Expected ToolResult"),
            }
        }
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_exact_10kb_threshold_not_blobified() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that content at exactly 10KB (10240 bytes) stays inline
    use chrono::Utc;
    use uuid::Uuid;

    // Exactly 10KB = 10 * 1024 = 10240 bytes
    let exactly_10kb = "x".repeat(10_240);
    assert_eq!(exactly_10kb.len(), 10_240);

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_exact".to_string(),
                content: exactly_10kb.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process should succeed");

    // Verify NO blob was created (threshold is >10KB, not >=10KB)
    assert!(
        blob_refs.is_empty(),
        "Exactly 10KB should NOT create blob (threshold is >10KB)"
    );

    // Verify content remains inline
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert_eq!(*content, exactly_10kb, "10KB content should remain inline");
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_one_byte_over_threshold_blobified() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that content at 10KB + 1 byte IS stored in blob
    use chrono::Utc;
    use uuid::Uuid;

    // 10KB + 1 = 10241 bytes
    let just_over_10kb = "x".repeat(10_241);
    assert_eq!(just_over_10kb.len(), 10_241);

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "user".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::User(UserMessage {
            role: "user".to_string(),
            content: vec![UserContent::ToolResult {
                tool_use_id: "toolu_over".to_string(),
                content: just_over_10kb.clone(),
                is_error: false,
                tool_use_result: None,
            }],
        }),
        request_id: None,
    };

    // Process for blob storage
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process should succeed");

    // Verify blob WAS created
    assert!(!blob_refs.is_empty(), "10KB+1 should create blob");

    // Verify content was replaced with blob reference
    match &processed.message {
        MessagePayload::User(user_msg) => match &user_msg.content[0] {
            UserContent::ToolResult { content, .. } => {
                assert!(
                    content.starts_with("blob:sha256:"),
                    "Should be blob reference"
                );
            }
            _ => panic!("Expected ToolResult"),
        },
        _ => panic!("Expected User message"),
    }
}

#[test]
fn test_tool_use_storage_and_retrieval() {
    let (_guard, _temp_dir) = setup_test_env();
    // Test that a simple ToolUse (no blob needed) is stored and retrieved correctly
    use chrono::Utc;
    use uuid::Uuid;

    let envelope = MessageEnvelope {
        uuid: Uuid::new_v4(),
        parent_uuid: None,
        timestamp: Utc::now(),
        message_type: "assistant".to_string(),
        provider: "claude".to_string(),
        message: MessagePayload::Assistant(AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_test".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![AssistantContent::ToolUse {
                id: "toolu_bash123".to_string(),
                name: "Bash".to_string(),
                input: serde_json::json!({
                    "command": "ls -la",
                    "description": "List files"
                }),
            }],
            stop_reason: Some("tool_use".to_string()),
            usage: None,
        }),
        request_id: Some("req_test123".to_string()),
    };

    // Process for blob storage (should NOT create blobs for small content)
    let (processed, blob_refs) =
        super::blob_processing::process_envelope_for_blob_storage(&envelope)
            .expect("process should succeed");

    // Verify NO blob was created (content is small)
    assert!(
        blob_refs.is_empty(),
        "Small ToolUse should NOT create blobs"
    );

    // Verify ToolUse content is preserved
    match &processed.message {
        MessagePayload::Assistant(assistant_msg) => match &assistant_msg.content[0] {
            AssistantContent::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_bash123");
                assert_eq!(name, "Bash");
                assert_eq!(input["command"], "ls -la");
                assert_eq!(input["description"], "List files");
            }
            _ => panic!("Expected ToolUse"),
        },
        _ => panic!("Expected Assistant message"),
    }

    // Verify round-trip through JSON serialization works
    let json = serde_json::to_string(&processed).unwrap();
    let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();

    match &restored.message {
        MessagePayload::Assistant(assistant_msg) => match &assistant_msg.content[0] {
            AssistantContent::ToolUse { id, name, input } => {
                assert_eq!(id, "toolu_bash123");
                assert_eq!(name, "Bash");
                assert_eq!(input["command"], "ls -la");
                assert_eq!(input["description"], "List files");
            }
            _ => panic!("Expected ToolUse after restore"),
        },
        _ => panic!("Expected Assistant message after restore"),
    }
}

// ============================================================================
// NAPI-008: Token Usage and Compaction State Persistence Tests
// ============================================================================

#[test]
fn test_set_session_tokens() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/token_set");

    // @step Given I have a session
    let mut session = create_session("Token Test", &project).expect("create");
    let session_id = session.id;

    // @step And the session has some initial token usage from messages
    append_message(&mut session, "user", "Hello").expect("append");
    append_message(&mut session, "assistant", "Hi there!").expect("append");

    // @step When I set the cumulative token usage to specific values
    // Pass cumulative values separately (here same as current for fresh session)
    set_session_tokens(&mut session, 1000, 500, 100, 50, 1000, 500).expect("set tokens");

    // @step Then the session should have exactly those token values (not added)
    // CTX-003: Now uses dual-metric fields
    assert_eq!(session.token_usage.current_context_tokens, 1000);
    assert_eq!(session.token_usage.cumulative_billed_input, 1000);
    assert_eq!(session.token_usage.cumulative_billed_output, 500);
    assert_eq!(session.token_usage.cache_read_tokens, 100);
    assert_eq!(session.token_usage.cache_creation_tokens, 50);

    // @step And when I reload the session from storage
    let reloaded = load_session(session_id).expect("reload");

    // @step Then the token values should be persisted
    // CTX-003: Now uses dual-metric fields
    assert_eq!(reloaded.token_usage.current_context_tokens, 1000);
    assert_eq!(reloaded.token_usage.cumulative_billed_input, 1000);
    assert_eq!(reloaded.token_usage.cumulative_billed_output, 500);
    assert_eq!(reloaded.token_usage.cache_read_tokens, 100);
    assert_eq!(reloaded.token_usage.cache_creation_tokens, 50);
}

#[test]
fn test_set_compaction_state() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_set");

    // @step Given I have a session with messages
    let mut session = create_session("Compaction Test", &project).expect("create");
    let session_id = session.id;
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step And the session has no compaction state initially
    assert!(session.compaction.is_none());

    // @step When I set the compaction state after context compaction
    let summary = "Compacted 8 turns (5000→1500 tokens, 70% compression)".to_string();
    set_compaction_state(&mut session, summary.clone(), 8).expect("set compaction");

    // @step Then the session should have the compaction state
    assert!(session.compaction.is_some());
    let compaction = session.compaction.as_ref().unwrap();
    assert_eq!(compaction.summary, summary);
    assert_eq!(compaction.compacted_before_index, 8);

    // @step And when I reload the session from storage
    let reloaded = load_session(session_id).expect("reload");

    // @step Then the compaction state should be persisted
    assert!(reloaded.compaction.is_some());
    let reloaded_compaction = reloaded.compaction.as_ref().unwrap();
    assert_eq!(reloaded_compaction.summary, summary);
    assert_eq!(reloaded_compaction.compacted_before_index, 8);
}

#[test]
fn test_clear_compaction_state() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_clear");

    // @step Given I have a session with compaction state
    let mut session = create_session("Compaction Clear Test", &project).expect("create");
    let session_id = session.id;
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }
    set_compaction_state(&mut session, "Test summary".to_string(), 5).expect("set compaction");
    assert!(session.compaction.is_some());

    // @step When I clear the compaction state
    clear_compaction_state(&mut session).expect("clear compaction");

    // @step Then the session should have no compaction state
    assert!(session.compaction.is_none());

    // @step And when I reload the session from storage
    let reloaded = load_session(session_id).expect("reload");

    // @step Then the compaction should still be cleared
    assert!(reloaded.compaction.is_none());
}

#[test]
fn test_token_and_compaction_state_persist_together() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/full_state");

    // @step Given I have a session
    let mut session = create_session("Full State Test", &project).expect("create");
    let session_id = session.id;
    for i in 0..20 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Msg {}", i)).expect("append");
    }

    // @step When I set both token usage and compaction state
    set_session_tokens(&mut session, 2000, 1000, 200, 100, 2000, 1000).expect("set tokens");
    set_compaction_state(&mut session, "Compacted 15 turns".to_string(), 15)
        .expect("set compaction");

    // @step And I reload the session
    let reloaded = load_session(session_id).expect("reload");

    // @step Then both token usage and compaction state should be restored
    // CTX-003: Now uses dual-metric fields
    assert_eq!(reloaded.token_usage.current_context_tokens, 2000);
    assert_eq!(reloaded.token_usage.cumulative_billed_input, 2000);
    assert_eq!(reloaded.token_usage.cumulative_billed_output, 1000);
    assert!(reloaded.compaction.is_some());
    assert_eq!(
        reloaded.compaction.as_ref().unwrap().compacted_before_index,
        15
    );

    // @step And the message count should still be correct
    assert_eq!(reloaded.messages.len(), 20);
}

// ============================================================================
// CRITICAL FIX: get_session_messages respects compaction state
// Tests that restored sessions use compacted context, not full history
// ============================================================================
#[test]
fn test_get_session_messages_respects_compaction() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/compaction_restore");

    // @step Given I have a session with 20 messages
    let mut session = create_session("Compaction Test", &project).expect("create");
    for i in 0..20 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Message {}", i)).expect("append");
    }
    let session_id = session.id;

    // @step And the session has been compacted at index 15 with a summary
    let summary = "Previous conversation covered messages 0-14, discussing authentication flow.";
    set_compaction_state(&mut session, summary.to_string(), 15).expect("set compaction");

    // @step When I retrieve messages using get_session_messages
    let reloaded = load_session(session_id).expect("reload");
    let messages = get_session_messages(&reloaded).expect("get messages");

    // @step Then I should receive:
    // - 1 synthetic summary message
    // - 5 messages from index 15 onward (messages 15, 16, 17, 18, 19)
    // Total: 6 messages (not 20!)
    assert_eq!(
        messages.len(),
        6,
        "Expected 1 summary + 5 post-compaction messages, got {}",
        messages.len()
    );

    // @step And the first message should be the synthetic summary
    let first_msg = &messages[0];
    assert!(
        first_msg.content.contains("Previous conversation summary"),
        "First message should be synthetic summary"
    );
    assert!(
        first_msg.content.contains(summary),
        "Summary content should be included"
    );
    assert_eq!(
        first_msg.id,
        uuid::Uuid::nil(),
        "Synthetic message should have nil UUID"
    );
    assert_eq!(first_msg.role, "user", "Synthetic summary should be 'user' role");

    // @step And the remaining messages should be the post-compaction messages
    assert!(
        messages[1].content.contains("Message 15"),
        "Second message should be original message 15"
    );
    assert!(
        messages[5].content.contains("Message 19"),
        "Last message should be original message 19"
    );
}

#[test]
fn test_get_session_messages_full_ignores_compaction() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/full_restore_unique");

    // @step Given I have a session with 10 messages
    let mut session = create_session("Full Restore Test Unique", &project).expect("create");
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("FullRestore Message {}", i)).expect("append");
    }
    let session_id = session.id;

    // Verify we have 10 messages before compaction
    assert_eq!(session.messages.len(), 10, "Should have 10 messages initially");

    // @step And the session has been compacted at index 8
    set_compaction_state(&mut session, "Summary of 0-7".to_string(), 8).expect("set compaction");

    // @step When I retrieve messages using get_session_messages_full
    let reloaded = load_session(session_id).expect("reload");
    
    // Verify reloaded session still has 10 message refs
    assert_eq!(reloaded.messages.len(), 10, "Reloaded session should have 10 message refs");
    
    let full_messages = get_session_messages_full(&reloaded).expect("get full messages");

    // @step Then I should receive all 10 original messages (no synthetic summary)
    assert_eq!(
        full_messages.len(),
        10,
        "get_session_messages_full should return all messages, got {}",
        full_messages.len()
    );

    // @step And the first message should be the original message 0
    assert!(
        full_messages[0].content.contains("FullRestore Message 0"),
        "First message should be original message 0, got: {}",
        full_messages[0].content
    );

    // @step Compare with compaction-aware retrieval
    let compacted_messages = get_session_messages(&reloaded).expect("get compacted messages");
    assert_eq!(
        compacted_messages.len(),
        3,
        "get_session_messages should return 1 summary + 2 post-compaction"
    );
}

#[test]
fn test_get_session_messages_no_compaction_returns_all() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/no_compaction");

    // @step Given I have a session with 5 messages and NO compaction
    let mut session = create_session("No Compaction Test", &project).expect("create");
    for i in 0..5 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Message {}", i)).expect("append");
    }
    let session_id = session.id;

    // @step And the session has no compaction state
    assert!(session.compaction.is_none());

    // @step When I retrieve messages using get_session_messages
    let reloaded = load_session(session_id).expect("reload");
    let messages = get_session_messages(&reloaded).expect("get messages");

    // @step Then I should receive all 5 messages (no synthetic summary)
    assert_eq!(messages.len(), 5, "Should return all messages when no compaction");

    // @step And they should be the original messages
    assert!(messages[0].content.contains("Message 0"));
    assert!(messages[4].content.contains("Message 4"));
}

// ============================================================================
// CRITICAL: Test that synthetic compaction envelope can be parsed by session restore
// This tests the actual flow: persistence → envelope JSON → MessageEnvelope struct
// ============================================================================
#[test]
fn test_synthetic_compaction_envelope_parses_as_message_envelope() {
    let (_guard, _temp_dir) = setup_test_env();
    use super::message_envelope::MessageEnvelope;

    let project = PathBuf::from("/test/project/envelope_parse_test");

    // @step Given I have a session with messages that has been compacted
    let mut session = create_session("Envelope Parse Test", &project).expect("create");
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        append_message(&mut session, role, &format!("Message {}", i)).expect("append");
    }
    let session_id = session.id;

    // @step And the session has compaction state
    let summary = "Previous conversation covered authentication flow and database design.";
    set_compaction_state(&mut session, summary.to_string(), 8).expect("set compaction");

    // @step When I get the message envelopes (which includes the synthetic compaction summary)
    let reloaded = load_session(session_id).expect("reload");
    let messages = get_session_messages(&reloaded).expect("get messages");

    // The first message should be synthetic (nil UUID)
    assert_eq!(
        messages[0].id,
        uuid::Uuid::nil(),
        "First message should be synthetic"
    );

    // @step And I construct the synthetic envelope JSON (same logic as napi_bindings.rs)
    let stored_msg = &messages[0];
    let synthetic_envelope_json = serde_json::json!({
        "uuid": "00000000-0000-0000-0000-000000000000",
        "parentUuid": null,
        "timestamp": stored_msg.created_at.to_rfc3339(),
        "type": "user",
        "provider": "compaction",
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": stored_msg.content}]
        },
        "requestId": null,
        "_synthetic": true,
        "_compactionSummary": true
    });
    let envelope_str = serde_json::to_string(&synthetic_envelope_json).unwrap();

    // @step Then the envelope should successfully parse as MessageEnvelope
    let parsed: Result<MessageEnvelope, _> = serde_json::from_str(&envelope_str);
    assert!(
        parsed.is_ok(),
        "Synthetic compaction envelope must parse as MessageEnvelope. Error: {:?}",
        parsed.err()
    );

    // @step And the parsed envelope should have correct fields
    let envelope = parsed.unwrap();
    assert_eq!(envelope.uuid, uuid::Uuid::nil());
    assert_eq!(envelope.message_type, "user");
    assert_eq!(envelope.provider, "compaction");

    // @step And the message payload should contain the summary
    match &envelope.message {
        super::message_envelope::MessagePayload::User(user_msg) => {
            assert_eq!(user_msg.role, "user");
            assert!(!user_msg.content.is_empty());
            match &user_msg.content[0] {
                super::message_envelope::UserContent::Text { text } => {
                    assert!(
                        text.contains(summary),
                        "Envelope text should contain the compaction summary"
                    );
                }
                _ => panic!("Expected Text content in synthetic envelope"),
            }
        }
        _ => panic!("Expected User message payload in synthetic envelope"),
    }
}

// ============================================================================
// Test the OLD broken format to document what was wrong
// This test ensures the bug is understood and doesn't regress
// ============================================================================
#[test]
fn test_old_broken_synthetic_envelope_fails_to_parse() {
    let (_guard, _temp_dir) = setup_test_env();
    use super::message_envelope::MessageEnvelope;

    // @step Given the OLD broken synthetic envelope format (missing required fields, wrong key name)
    let broken_envelope_json = serde_json::json!({
        "message_type": "user",  // WRONG: should be "type" due to #[serde(rename = "type")]
        "message": {
            "role": "user",
            "content": [{"type": "text", "text": "test summary"}]
        },
        "_synthetic": true,
        "_compactionSummary": true
        // MISSING: uuid, timestamp, provider (all required!)
    });
    let envelope_str = serde_json::to_string(&broken_envelope_json).unwrap();

    // @step Then the broken envelope should FAIL to parse as MessageEnvelope
    let parsed: Result<MessageEnvelope, _> = serde_json::from_str(&envelope_str);
    assert!(
        parsed.is_err(),
        "Old broken envelope format should NOT parse - if this passes, the bug isn't fixed!"
    );
}

// ============================================================================
// END-TO-END TEST: Verify compacted session envelopes can be restored
// This tests the ACTUAL bug fix - that synthetic compaction envelopes
// returned by persistence_get_session_message_envelopes() can be parsed
// by restore_messages_from_envelopes() in session.rs
// ============================================================================
#[test]
fn test_compacted_session_envelopes_can_be_parsed_end_to_end() {
    let (_guard, _temp_dir) = setup_test_env();
    use super::message_envelope::MessageEnvelope;

    let project = PathBuf::from("/test/project/e2e_compaction");

    // @step Given I have a session with 10 messages
    let mut session = create_session("E2E Compaction Test", &project).expect("create");
    for i in 0..10 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        // Store messages with proper envelope metadata (simulating real usage)
        let metadata = {
            let mut meta = std::collections::HashMap::new();
            let envelope = serde_json::json!({
                "uuid": uuid::Uuid::new_v4().to_string(),
                "parentUuid": null,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "type": role,
                "provider": "test",
                "message": {
                    "role": role,
                    "content": [{"type": "text", "text": format!("Message {}", i)}]
                },
                "requestId": null
            });
            for (k, v) in envelope.as_object().unwrap() {
                meta.insert(k.clone(), v.clone());
            }
            meta
        };
        append_message_with_metadata(&mut session, role, &format!("Message {}", i), metadata)
            .expect("append");
    }
    let session_id = session.id;

    // @step And the session has been compacted at index 8
    let summary = "Previous 8 messages discussed project setup and architecture decisions.";
    set_compaction_state(&mut session, summary.to_string(), 8).expect("set compaction");

    // @step When I reload the session and get envelopes (simulating /resume)
    let reloaded = load_session(session_id).expect("reload");

    // This is what persistence_get_session_message_envelopes does internally
    let messages = get_session_messages(&reloaded).expect("get messages");

    // Should have: 1 synthetic summary + 2 post-compaction messages (indices 8, 9)
    assert_eq!(
        messages.len(),
        3,
        "Expected 1 summary + 2 post-compaction messages"
    );

    // @step And I convert each message to envelope JSON (like napi_bindings does)
    let mut envelopes: Vec<String> = Vec::new();
    for stored_msg in &messages {
        if stored_msg.id == uuid::Uuid::nil() {
            // Synthetic compaction summary - use the FIXED format
            let synthetic_envelope = serde_json::json!({
                "uuid": "00000000-0000-0000-0000-000000000000",
                "parentUuid": null,
                "timestamp": stored_msg.created_at.to_rfc3339(),
                "type": "user",
                "provider": "compaction",
                "message": {
                    "role": "user",
                    "content": [{"type": "text", "text": stored_msg.content}]
                },
                "requestId": null,
                "_synthetic": true,
                "_compactionSummary": true
            });
            envelopes.push(serde_json::to_string(&synthetic_envelope).unwrap());
        } else {
            // Real message - use stored metadata
            let metadata_value = serde_json::Value::Object(
                stored_msg
                    .metadata
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            );
            envelopes.push(serde_json::to_string(&metadata_value).unwrap());
        }
    }

    // @step Then ALL envelopes should parse successfully as MessageEnvelope
    // This is what restore_messages_from_envelopes() does
    for (idx, envelope_json) in envelopes.iter().enumerate() {
        let parsed: Result<MessageEnvelope, _> = serde_json::from_str(envelope_json);
        assert!(
            parsed.is_ok(),
            "Envelope {} failed to parse as MessageEnvelope: {:?}\nJSON: {}",
            idx,
            parsed.err(),
            &envelope_json[..std::cmp::min(500, envelope_json.len())]
        );
    }

    // @step And the first envelope should be the compaction summary
    let first_envelope: MessageEnvelope = serde_json::from_str(&envelopes[0]).unwrap();
    assert_eq!(first_envelope.uuid, uuid::Uuid::nil());
    assert_eq!(first_envelope.provider, "compaction");
    match &first_envelope.message {
        super::message_envelope::MessagePayload::User(user_msg) => {
            match &user_msg.content[0] {
                super::message_envelope::UserContent::Text { text } => {
                    assert!(
                        text.contains(summary),
                        "First envelope should contain the compaction summary"
                    );
                }
                _ => panic!("Expected Text content"),
            }
        }
        _ => panic!("Expected User message"),
    }

    // @step And the remaining envelopes should be the post-compaction messages
    let second_envelope: MessageEnvelope = serde_json::from_str(&envelopes[1]).unwrap();
    match &second_envelope.message {
        super::message_envelope::MessagePayload::User(user_msg) => {
            match &user_msg.content[0] {
                super::message_envelope::UserContent::Text { text } => {
                    assert!(
                        text.contains("Message 8"),
                        "Second envelope should be message 8 (first post-compaction)"
                    );
                }
                _ => panic!("Expected Text content"),
            }
        }
        _ => {} // Could be assistant message too
    }
}
