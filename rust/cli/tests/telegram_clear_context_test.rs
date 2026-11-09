#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/telegram-clear-context-reset.feature
//!
//! Tests for AGENT-022: Telegram bridge /clear command must properly reset session context
//!
//! This test validates that when /clear is sent from Telegram, the bridge control handler
//! properly clears messages, turns, token_tracker, AND reinjects context reminders.
//!
//! The block_in_place control-handler pattern tests from the retired
//! codelet-napi copy exercised only tokio primitives and were not relocated.

#[path = "support/offline_session.rs"]
mod offline_session;

use codelet_cli::session::Session;
use codelet_core::compaction::TokenTracker;
use rig::message::{Message, UserContent};
use rig::OneOrMany;

/// Helper to simulate the control handler's clear action as it SHOULD work
fn simulate_correct_clear_action(session: &mut Session) {
    // Clear actual session state (what the fix should do)
    session.messages.clear();
    session.turns.clear();
    session.token_tracker = TokenTracker::default();

    // CRITICAL: Reinject context reminders so AI retains project context
    session.inject_context_reminders("/work".as_ref());
}

/// Helper to count system reminder messages in session
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

#[path = "telegram_clear_context_test/context.rs"]
mod context;
#[path = "telegram_clear_context_test/state.rs"]
mod state;
