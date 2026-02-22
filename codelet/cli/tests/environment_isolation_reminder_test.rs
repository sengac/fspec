//! Feature: spec/features/ai-system-reminder-includes-isolation-state-and-worktree-path.feature
//!
//! Tests for GIT-034: AI System Reminder Includes Isolation State and Worktree Path
//!
//! This test file validates that the environment system reminder correctly includes
//! isolation fields when running in an isolated worktree session.

use codelet_cli::session::context_gathering::{
    gather_environment_info, gather_environment_info_with_isolation, IsolationContext,
};

// Scenario: Isolated session environment reminder includes isolation fields
// @GIT-034 @isolated @system-reminder
#[test]
fn test_isolated_session_environment_reminder_includes_isolation_fields() {
    // @step Given a session is created with isolated mode enabled
    let isolation = IsolationContext {
        is_isolated: true,
        worktree_path: Some(".fspec/worktrees/abc-123/".to_string()),
        base_commit: Some("7a8b9c0def123456".to_string()),
    };

    // @step And the worktree is at ".fspec/worktrees/abc-123/"
    // @step And the base commit SHA is "7a8b9c0def123456"
    // (Already set above)

    // @step When the environment system-reminder is generated
    let env_info = gather_environment_info_with_isolation(Some(&isolation));
    let content = env_info.to_reminder_content();

    // @step Then the reminder should contain "Isolation: ACTIVE"
    assert!(
        content.contains("Isolation: ACTIVE"),
        "Reminder should contain 'Isolation: ACTIVE'. Got:\n{content}"
    );

    // @step And the reminder should contain "Worktree: .fspec/worktrees/abc-123/"
    assert!(
        content.contains("Worktree: .fspec/worktrees/abc-123/"),
        "Reminder should contain worktree path. Got:\n{content}"
    );

    // @step And the reminder should contain "Base commit: 7a8b9c0d"
    // Note: Base commit is displayed as short SHA (first 8 chars)
    assert!(
        content.contains("Base commit: 7a8b9c0d"),
        "Reminder should contain short base commit SHA. Got:\n{content}"
    );
}

// Scenario: Non-isolated session environment reminder excludes isolation fields
// @GIT-034 @non-isolated @system-reminder
#[test]
fn test_non_isolated_session_environment_reminder_excludes_isolation_fields() {
    // @step Given a session is created with isolated mode disabled
    // (No isolation context provided - simulates non-isolated session)

    // @step When the environment system-reminder is generated
    let env_info = gather_environment_info();
    let content = env_info.to_reminder_content();

    // @step Then the reminder should NOT contain "Isolation:"
    assert!(
        !content.contains("Isolation:"),
        "Non-isolated reminder should NOT contain 'Isolation:'. Got:\n{content}"
    );

    // @step And the reminder should NOT contain "Worktree:"
    assert!(
        !content.contains("Worktree:"),
        "Non-isolated reminder should NOT contain 'Worktree:'. Got:\n{content}"
    );

    // @step And the reminder should NOT contain "Base commit:"
    assert!(
        !content.contains("Base commit:"),
        "Non-isolated reminder should NOT contain 'Base commit:'. Got:\n{content}"
    );

    // @step And the reminder should contain "Working directory:"
    assert!(
        content.contains("Working directory:"),
        "Non-isolated reminder should contain 'Working directory:'. Got:\n{content}"
    );
}

// Scenario: AI can explain worktree location to user
// @GIT-034 @isolated @user-interaction
#[test]
fn test_ai_has_worktree_context_for_user_explanation() {
    // @step Given a session is created with isolated mode enabled
    let isolation = IsolationContext {
        is_isolated: true,
        worktree_path: Some(".fspec/worktrees/abc-123/".to_string()),
        base_commit: Some("7a8b9c0d".to_string()),
    };

    // @step And the worktree is at ".fspec/worktrees/abc-123/"
    // @step And the environment reminder has been injected into context
    let env_info = gather_environment_info_with_isolation(Some(&isolation));
    let content = env_info.to_reminder_content();

    // @step When the user asks "where are my changes?"
    // @step Then the AI has context to respond with the worktree path
    // This is verified by checking the reminder contains the worktree path
    assert!(
        content.contains(".fspec/worktrees/abc-123/"),
        "AI should have worktree path in context for user explanation"
    );

    // @step And the AI has context to explain that merging is required
    // This is verified by the presence of "Isolation: ACTIVE" which signals
    // changes are isolated and need merging
    assert!(
        content.contains("Isolation: ACTIVE"),
        "AI should have isolation state to explain merging is required"
    );
}

// Scenario: AI can advise user about merge/discard options
// @GIT-034 @isolated @user-interaction
#[test]
fn test_ai_has_isolation_context_for_merge_discard_advice() {
    // @step Given a session is created with isolated mode enabled
    let isolation = IsolationContext {
        is_isolated: true,
        worktree_path: Some(".fspec/worktrees/def-456/".to_string()),
        base_commit: Some("abcd1234".to_string()),
    };

    // @step And the AI has completed making changes
    // @step And the environment reminder includes isolation state
    let env_info = gather_environment_info_with_isolation(Some(&isolation));
    let content = env_info.to_reminder_content();

    // @step When the AI responds to task completion
    // @step Then the AI has context to suggest "/merge" to apply changes
    // @step And the AI has context to suggest "/discard" to abandon changes
    // This is verified by checking the reminder contains isolation context
    assert!(
        content.contains("Isolation: ACTIVE"),
        "AI needs isolation context to advise on merge/discard"
    );
    assert!(
        content.contains("Worktree:"),
        "AI needs worktree info to advise on merge/discard"
    );
}

// Additional test: Isolation fields appear between Working directory and Date
#[test]
#[allow(clippy::unwrap_used)]
fn test_isolation_fields_appear_in_correct_position() {
    // @step Given isolation context with all fields
    let isolation = IsolationContext {
        is_isolated: true,
        worktree_path: Some(".fspec/worktrees/test-123/".to_string()),
        base_commit: Some("deadbeef12345678".to_string()),
    };

    // @step When the environment reminder is generated
    let env_info = gather_environment_info_with_isolation(Some(&isolation));
    let content = env_info.to_reminder_content();
    let lines: Vec<&str> = content.lines().collect();

    // Find positions of key fields
    let cwd_pos = lines
        .iter()
        .position(|l| l.starts_with("Working directory:"));
    let isolation_pos = lines.iter().position(|l| l.starts_with("Isolation:"));
    let worktree_pos = lines.iter().position(|l| l.starts_with("Worktree:"));
    let commit_pos = lines.iter().position(|l| l.starts_with("Base commit:"));
    let date_pos = lines.iter().position(|l| l.starts_with("Date:"));

    // @step Then Isolation should appear after Working directory
    assert!(
        cwd_pos.is_some() && isolation_pos.is_some(),
        "Both Working directory and Isolation should be present"
    );
    assert!(
        isolation_pos.unwrap() > cwd_pos.unwrap(),
        "Isolation should appear after Working directory"
    );

    // @step And Worktree should appear after Isolation
    assert!(
        worktree_pos.is_some() && worktree_pos.unwrap() > isolation_pos.unwrap(),
        "Worktree should appear after Isolation"
    );

    // @step And Base commit should appear after Worktree
    assert!(
        commit_pos.is_some() && commit_pos.unwrap() > worktree_pos.unwrap(),
        "Base commit should appear after Worktree"
    );

    // @step And Date should appear last
    assert!(
        date_pos.is_some() && date_pos.unwrap() > commit_pos.unwrap(),
        "Date should appear after Base commit (last)"
    );
}

// Test that short SHA is used for base commit display
#[test]
fn test_base_commit_displays_short_sha() {
    // @step Given a long SHA
    let isolation = IsolationContext {
        is_isolated: true,
        worktree_path: Some(".fspec/worktrees/test/".to_string()),
        base_commit: Some("abcdef1234567890abcdef".to_string()), // Long SHA
    };

    // @step When the environment reminder is generated
    let env_info = gather_environment_info_with_isolation(Some(&isolation));
    let content = env_info.to_reminder_content();

    // @step Then the reminder should contain short SHA (first 8 chars)
    assert!(
        content.contains("Base commit: abcdef12"),
        "Base commit should display short SHA (8 chars). Got:\n{content}"
    );

    // @step And the reminder should NOT contain the full SHA
    assert!(
        !content.contains("abcdef1234567890abcdef"),
        "Base commit should NOT display full SHA"
    );
}
