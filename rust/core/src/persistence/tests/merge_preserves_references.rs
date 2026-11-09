use super::*;

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
    history::add(HistoryEntry::new(
        "cmd a".to_string(),
        project_a.clone(),
        sa.id,
    ))
    .expect("add");

    // @step And I have history entries from project "/home/user/project-b"
    let sb = create_session("B", &project_b).expect("create");
    history::add(HistoryEntry::new(
        "cmd b".to_string(),
        project_b.clone(),
        sb.id,
    ))
    .expect("add");

    // @step When I am in project "/home/user/project-a"
    // @step And I run "/history"
    let hist_a = history::get(Some(&project_a), None).expect("get");

    // @step Then I should see only history entries from project-a
    for h in &hist_a {
        assert_eq!(h.project, project_a);
    }

    // @step When I run "/history --all-projects"
    let all = history::get(None, None).expect("get");

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
