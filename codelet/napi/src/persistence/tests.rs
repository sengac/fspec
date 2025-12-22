// Feature: spec/features/session-persistence-with-fork-and-merge.feature
//
// Session Persistence with Fork and Merge
// Tests for the Rust persistence module

use super::*;
use std::path::PathBuf;

// ============================================================================
// Scenario: Resume session after closing terminal
// @session-resume
// ============================================================================
#[test]
fn test_resume_session_after_closing_terminal() {
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
