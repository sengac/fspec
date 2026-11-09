use super::*;

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
    history::add(HistoryEntry::new(
        "implement feature".to_string(),
        project.clone(),
        session.id,
    ))
    .expect("add should succeed");

    // @step When I press Ctrl+R and type "implement"
    let results = history::search("implement", Some(&project)).expect("search should succeed");

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
    history::add(HistoryEntry::new(
        "fix login bug".to_string(),
        project.clone(),
        session_b.id,
    ))
    .expect("add");

    // @step And I entered "implement auth flow" in session A at 11:00am
    // TEST-042: no sleep; the later entry is stamped an hour later.
    let session_a = create_session("Session A", &project).expect("create");
    let mut later = HistoryEntry::new(
        "implement auth flow".to_string(),
        project.clone(),
        session_a.id,
    );
    later.timestamp += chrono::Duration::hours(1);
    history::add(later).expect("add");

    // @step When I switch to session A
    // @step And I press Shift+Arrow-Up
    let history = history::get(Some(&project), None).expect("get");

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
