//! Integration tests for blocklist and stage permissions modules
//!
//! These tests validate the blocking logic independent of the facade wrapper integration.
//! The main feature tests are in rust/napi/tests/block_notifications_emission_test.rs
//!
//! BLOCK-006: Block Notifications

#![allow(clippy::unwrap_used, clippy::expect_used)]

use codelet_tools::blocklist::{
    check_bash_command, clear_session_allowances, init_blocklist, BlocklistAction, BlocklistConfig,
    BlocklistRule,
};
use codelet_tools::stage_permissions::{check_write_permission, init_stage_permissions};
use serial_test::serial;
use std::io::Write;
use tempfile::TempDir;

/// Feature: Block Notifications
/// Background: Block Notifications Enabled
///
/// Scenario: Notify user when AI command is blocked
#[test]
#[serial]
fn test_notify_user_when_ai_command_is_blocked() {
    // @step Given the user has fspec TUI running with notifications enabled
    // (TUI running is a prerequisite for the test environment)

    // @step Given a blocklist rule exists blocking "git checkout" with reason "Use git switch instead"
    let tmp = TempDir::new().expect("failed to create temp dir");
    let fspec_dir = tmp.path().join(".fspec");
    std::fs::create_dir_all(&fspec_dir).expect("create .fspec dir");

    let config = BlocklistConfig {
        version: "1.0.0".to_string(),
        rules: vec![BlocklistRule {
            id: "git-checkout-block".to_string(),
            pattern: r"^git\s+checkout".to_string(),
            action: BlocklistAction::Block,
            reason: "Use git switch instead".to_string(),
            guidance: None,
        }],
    };

    let path = fspec_dir.join("blocklist.json");
    let mut file = std::fs::File::create(&path).expect("create blocklist.json");
    let json = serde_json::to_string_pretty(&config).expect("serialize config");
    file.write_all(json.as_bytes())
        .expect("write blocklist.json");

    // Initialize blocklist with the temp project root
    init_blocklist(Some(tmp.path()));

    // @step When the AI runs "git checkout main" via Bash
    let result = check_bash_command("git checkout main", uuid::Uuid::nil());

    // @step Then the command should be blocked
    assert!(result.is_err(), "Command should be blocked");

    // @step And the user should see a notification "AI was blocked from git checkout - Use git switch instead"
    let blocked = result.unwrap_err();
    // The blocked error contains the reason from the blocklist rule
    assert!(
        blocked.reason.contains("Use git switch"),
        "Reason should contain guidance 'Use git switch', got: {}",
        blocked.reason
    );

    // @step And the notification should auto-dismiss
    // (Auto-dismiss is handled by TUI NotificationDialog with timeout - tested in TUI tests)

    // Cleanup
    clear_session_allowances();
    init_blocklist(None);
}

/// Feature: Block Notifications
/// Background: Block Notifications Enabled
///
/// Scenario: Notify user when AI file write is blocked by stage permissions
#[test]
#[serial]
fn test_notify_user_when_ai_file_write_is_blocked_by_stage_permissions() {
    // @step Given the user has fspec TUI running with notifications enabled
    // (TUI running is a prerequisite for the test environment)

    // @step Given the current work unit is in "testing" stage
    let stage = "testing";

    // @step And "testing" stage only allows writing to "spec" and "test" categories
    let tmp = TempDir::new().expect("failed to create temp dir");
    init_stage_permissions(Some(tmp.path())); // Uses default ACDD config

    // @step When the AI tries to write to "src/auth.ts"
    let result = check_write_permission("src/auth.ts", Some(stage));

    // @step Then the write should be blocked
    assert!(result.is_err(), "Write should be blocked");

    // @step And the user should see a notification "AI was blocked from writing src/auth.ts - Cannot write impl files in testing stage"
    let blocked = result.unwrap_err();
    assert!(
        blocked.reason.contains("testing stage"),
        "Reason should mention testing stage, got: {}",
        blocked.reason
    );

    // @step And the notification should auto-dismiss
    // (Auto-dismiss is handled by TUI NotificationDialog with timeout - tested in TUI tests)
}

/// Additional test: Verify allowed files are NOT blocked in testing stage
#[test]
#[serial]
fn test_test_files_not_blocked_in_testing_stage() {
    // @step Given the current work unit is in "testing" stage
    let stage = "testing";

    // Initialize stage permissions with default ACDD config
    let tmp = TempDir::new().expect("failed to create temp dir");
    init_stage_permissions(Some(tmp.path()));

    // @step When the AI tries to write to a test file
    let result = check_write_permission("src/__tests__/auth.test.ts", Some(stage));

    // @step Then the write should be allowed
    assert!(
        result.is_ok(),
        "Test file write should be allowed in testing stage"
    );
}

/// Additional test: Verify no block when no work unit is active
#[test]
#[serial]
fn test_no_block_when_no_work_unit_active() {
    // Initialize stage permissions
    let tmp = TempDir::new().expect("failed to create temp dir");
    init_stage_permissions(Some(tmp.path()));

    // @step Given no work unit is set for the session
    let stage: Option<&str> = None;

    // @step When the AI tries to write to an impl file
    let result = check_write_permission("src/auth.ts", stage);

    // @step Then the write should be allowed (no ACDD enforcement without work unit)
    assert!(
        result.is_ok(),
        "Write should be allowed when no work unit is active"
    );
}

/// Test notification message format
#[test]
#[serial]
fn test_block_notification_message_format() {
    // @step Given a blocklist rule blocking "cat" commands
    let tmp = TempDir::new().expect("failed to create temp dir");
    let fspec_dir = tmp.path().join(".fspec");
    std::fs::create_dir_all(&fspec_dir).expect("create .fspec dir");

    let config = BlocklistConfig {
        version: "1.0.0".to_string(),
        rules: vec![BlocklistRule {
            id: "cat-block".to_string(),
            pattern: r"^cat\s+".to_string(),
            action: BlocklistAction::Block,
            reason: "Use Read tool instead".to_string(),
            guidance: Some("The Read tool provides better encoding and line numbers.".to_string()),
        }],
    };

    let path = fspec_dir.join("blocklist.json");
    let mut file = std::fs::File::create(&path).expect("create blocklist.json");
    let json = serde_json::to_string_pretty(&config).expect("serialize config");
    file.write_all(json.as_bytes())
        .expect("write blocklist.json");

    init_blocklist(Some(tmp.path()));

    // @step When the AI runs "cat src/file.ts"
    let result = check_bash_command("cat src/file.ts", uuid::Uuid::nil());

    // @step Then the error should follow format "Blocked: [reason] [guidance]"
    assert!(result.is_err());
    let error = result.unwrap_err();
    let message = error.to_string();

    assert!(
        message.starts_with("Blocked:"),
        "Message should start with 'Blocked:', got: {message}",
    );
    assert!(
        message.contains("Use Read tool"),
        "Message should contain the reason, got: {message}",
    );

    // Cleanup
    clear_session_allowances();
    init_blocklist(None);
}
