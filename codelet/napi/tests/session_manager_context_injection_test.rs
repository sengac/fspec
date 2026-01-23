#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/background-session-management-with-attach-detach.feature
//!
//! Tests for Context Injection in Background Sessions
//!
//! These tests verify that background sessions created via SessionManager
//! have context reminders (CLAUDE.md discovery, environment info) properly
//! injected, matching the behavior of the original CodeletSession.
//!
//! Bug found: NAPI-009 introduced SessionManager but forgot to call
//! inject_context_reminders() in create_session_with_id(), causing the LLM
//! to not receive environment info (platform, arch, shell, user, cwd).

use codelet_cli::session::Session;
use rig::message::{Message, UserContent};

/// Helper function to count system reminder messages in a session
fn count_system_reminder_messages(session: &Session) -> usize {
    session
        .messages
        .iter()
        .filter(|msg| {
            if let Message::User { content, .. } = msg {
                content.iter().any(|item| {
                    if let UserContent::Text(text) = item {
                        text.text.contains("<system-reminder>")
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        })
        .count()
}

/// Helper function to check if environment info is present in session messages
fn has_environment_reminder(session: &Session) -> bool {
    session.messages.iter().any(|msg| {
        if let Message::User { content, .. } = msg {
            content.iter().any(|item| {
                if let UserContent::Text(text) = item {
                    text.text.contains("<system-reminder>")
                        && text.text.contains("<!-- type:environment -->")
                        && text.text.contains("Working directory:")
                } else {
                    false
                }
            })
        } else {
            false
        }
    })
}

/// Test that Session::from_provider_manager + inject_context_reminders() works
///
/// This is the pattern that SessionManager::create_session_with_id should use.
///
/// Scenario: Context reminders injected in session created from provider manager
///
/// @step Given a ProviderManager instance
/// @step When I create a Session using from_provider_manager
/// @step And I call inject_context_reminders
/// @step Then the session should have environment info in messages
#[tokio::test]
async fn test_session_from_provider_manager_with_context_injection() {
    // Skip test if no provider configured
    let provider_manager = match codelet_providers::ProviderManager::with_model_support().await {
        Ok(pm) => pm,
        Err(_) => {
            eprintln!("Skipping test - no provider configured");
            return;
        }
    };

    // @step When I create a Session using from_provider_manager
    let mut session = Session::from_provider_manager(provider_manager);

    // Before injection, should have no system reminders
    assert_eq!(
        count_system_reminder_messages(&session),
        0,
        "Session should have no reminders before injection"
    );

    // @step And I call inject_context_reminders
    session.inject_context_reminders();

    // @step Then the session should have environment info in messages
    assert!(
        has_environment_reminder(&session),
        "Session should have environment reminder with Working directory after injection"
    );

    // Should have at least 1 system reminder (environment info)
    // May have 2 if CLAUDE.md is found in the project
    let reminder_count = count_system_reminder_messages(&session);
    assert!(
        reminder_count >= 1,
        "Session should have at least 1 system reminder (environment info), got {}",
        reminder_count
    );
}

/// Test that environment info contains working directory
///
/// Scenario: Environment info includes current working directory
///
/// @step Given a session with context reminders injected
/// @step When I examine the environment reminder
/// @step Then it should contain "Working directory:" with a path
#[tokio::test]
async fn test_environment_reminder_contains_cwd() {
    // Skip test if no provider configured
    let provider_manager = match codelet_providers::ProviderManager::with_model_support().await {
        Ok(pm) => pm,
        Err(_) => {
            eprintln!("Skipping test - no provider configured");
            return;
        }
    };

    let mut session = Session::from_provider_manager(provider_manager);
    session.inject_context_reminders();

    // Find the environment reminder and verify it contains cwd
    let env_reminder = session.messages.iter().find(|msg| {
        if let Message::User { content, .. } = msg {
            content.iter().any(|item| {
                if let UserContent::Text(text) = item {
                    text.text.contains("<!-- type:environment -->")
                } else {
                    false
                }
            })
        } else {
            false
        }
    });

    assert!(env_reminder.is_some(), "Environment reminder should exist");

    if let Some(Message::User { content, .. }) = env_reminder {
        if let UserContent::Text(text) = content.first() {
            assert!(
                text.text.contains("Working directory:"),
                "Environment reminder should contain 'Working directory:'. Got: {}",
                &text.text[..text.text.len().min(500)]
            );
            assert!(
                text.text.contains("Platform:"),
                "Environment reminder should contain 'Platform:'"
            );
            assert!(
                text.text.contains("Architecture:"),
                "Environment reminder should contain 'Architecture:'"
            );
        }
    }
}

/// Test that Session::new() also injects context (for comparison)
///
/// Scenario: Original Session::new path also has context reminders
///
/// @step Given Session::new is called
/// @step When I examine the session
/// @step Then context reminders should NOT be automatically injected
/// @step Because Session::new doesn't call inject_context_reminders internally
#[test]
fn test_session_new_does_not_auto_inject() {
    // Skip test if no provider configured
    let session = match Session::new(None) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Skipping test - no provider configured");
            return;
        }
    };

    // Session::new does NOT call inject_context_reminders internally
    // The caller (CodeletSession::new in NAPI or run_interactive_mode in CLI)
    // is responsible for calling it
    assert_eq!(
        count_system_reminder_messages(&session),
        0,
        "Session::new should not auto-inject context reminders"
    );
}

/// Test that multiple inject_context_reminders calls don't duplicate
///
/// Scenario: Calling inject_context_reminders multiple times appends reminders
///
/// @step Given a session with context reminders already injected
/// @step When I call inject_context_reminders again
/// @step Then old reminders are preserved and new ones appended (for cache stability)
#[tokio::test]
async fn test_inject_context_reminders_preserves_old() {
    // Skip test if no provider configured
    let provider_manager = match codelet_providers::ProviderManager::with_model_support().await {
        Ok(pm) => pm,
        Err(_) => {
            eprintln!("Skipping test - no provider configured");
            return;
        }
    };

    let mut session = Session::from_provider_manager(provider_manager);

    // First injection
    session.inject_context_reminders();
    let count_after_first = count_system_reminder_messages(&session);

    // Second injection (should append, not replace, for cache stability)
    session.inject_context_reminders();
    let count_after_second = count_system_reminder_messages(&session);

    // Per CLI-013: system reminders are APPENDED to preserve prompt cache prefix
    // So count should double
    assert_eq!(
        count_after_second,
        count_after_first * 2,
        "Second inject should append reminders (for cache stability). First: {}, Second: {}",
        count_after_first,
        count_after_second
    );
}
