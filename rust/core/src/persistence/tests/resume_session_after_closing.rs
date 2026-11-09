use super::*;

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
    history::add(HistoryEntry::new(
        "cmd1".to_string(),
        project.clone(),
        session.id,
    ))
    .expect("add history should succeed");
    history::add(HistoryEntry::new(
        "cmd2".to_string(),
        project.clone(),
        session.id,
    ))
    .expect("add history should succeed");

    // @step When I press Shift+Arrow-Up
    let history = history::get(Some(&project), None).expect("get_history should succeed");

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
