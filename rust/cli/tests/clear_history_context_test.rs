#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/clear-context-command-for-session-reset.feature
//!
//! Tests for AGENT-003: Clear context command for session reset
//!
//! These tests verify that clear_history() properly resets the session
//! while preserving system context (AGENTS.md, environment info).

#[path = "support/offline_session.rs"]
mod offline_session;

use codelet_cli::session::Session;
use rig::message::{Message, UserContent};
use rig::OneOrMany;

/// Helper function to count system reminder messages in session
fn count_system_reminder_messages(session: &Session) -> usize {
    session
        .messages
        .iter()
        .filter(|msg| {
            if let Message::User { content, .. } = msg {
                // Check if any content item contains system-reminder tags
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

/// Helper function to clear session state (DRY - extracted from duplicated code)
///
/// This mirrors what the session-level clear_history() does to the
/// codelet-cli Session (messages, turns, token tracker).
/// Note: Does NOT call inject_context_reminders() - tests that need reinjection
/// should call it separately after clearing.
fn clear_session_state(session: &mut Session) {
    session.messages.clear();
    session.turns.clear();
    session.token_tracker = codelet_core::compaction::TokenTracker::default();
}

#[path = "clear_history_context_test/preserve.rs"]
mod preserve;
#[path = "clear_history_context_test/reset.rs"]
mod reset;
