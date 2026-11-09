#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: System Reminder Preservation Through Compaction and Session Restore
//!
//! These tests verify that system reminders (environment, systemPrompt, fspecWorkflow)
//! are properly preserved through compaction and restored on session resume.
//!
//! CRITICAL BEHAVIOR:
//! 1. On session creation: inject_context_reminders() adds fresh reminders
//! 2. During compaction: partition_for_compaction() preserves reminders at start
//! 3. On session restore: Old reminders skipped (fresh ones injected on creation)
//!
//! This ensures the LLM always has access to:
//! - Platform/architecture/shell/user/working directory (environment)
//! - Project documentation (systemPrompt)
//! - Workflow guidance (fspecWorkflow)

use codelet_cli::session::system_reminders::{
    add_system_reminder, is_system_reminder, partition_for_compaction, SystemReminderType,
};
use rig::message::{AssistantContent, Message, UserContent};
use rig::OneOrMany;

/// Helper to create a user message
fn create_user_message(content: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::text(content)),
    }
}

/// Helper to create an assistant message
fn create_assistant_message(content: &str) -> Message {
    Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text(content)),
    }
}

/// Helper to create a system reminder message with proper format
fn create_system_reminder_message(reminder_type: SystemReminderType, content: &str) -> Message {
    let messages: Vec<Message> = vec![];
    let result = add_system_reminder(&messages, reminder_type, content);
    result.into_iter().next().unwrap()
}

/// Helper to check if a message is a system reminder of a specific type
fn is_reminder_of_type(msg: &Message, type_marker: &str) -> bool {
    match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => {
                t.text.contains("<system-reminder>") && t.text.contains(type_marker)
            }
            _ => false,
        },
        _ => false,
    }
}

/// Helper to extract text from a message
fn get_message_text(msg: &Message) -> Option<String> {
    match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => Some(t.text),
            _ => None,
        },
        Message::Assistant { content, .. } => match content.first() {
            AssistantContent::Text(t) => Some(t.text),
            _ => None,
        },
    }
}

#[path = "system_reminder_restoration_test/compaction.rs"]
mod compaction;
#[path = "system_reminder_restoration_test/edge_cases.rs"]
mod edge_cases;
#[path = "system_reminder_restoration_test/restore.rs"]
mod restore;
