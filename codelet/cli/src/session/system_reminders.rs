//! System Reminder Management (CLI-012, CLI-013)
//!
//! Provides functions for managing system reminders in conversation history
//! with prompt cache preservation.
//!
//! One-for-one port of codelet's system-reminder system from:
//! - src/agent/system-reminders.ts
//! - src/agent/context.ts
//!
//! CLI-012: Initial system-reminder infrastructure
//! CLI-013: Fix replacement to preserve prompt cache prefix
//!
//! CRITICAL: System-reminder replacement must NOT remove old reminders during
//! normal operation. Old reminders stay in place to preserve the prompt cache
//! prefix. New reminders are appended with a supersession marker. Only during
//! compaction are old reminders cleaned up (keeping only the latest per type).

use rig::message::{Message, UserContent};
use rig::OneOrMany;

/// Valid system reminder types for categorization.
///
/// Matches codelet's SystemReminderType from system-reminders.ts:14-19
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemReminderType {
    /// Project documentation (CLAUDE.md/AGENTS.md)
    ClaudeMd,
    /// Platform, arch, shell, user, cwd
    Environment,
    /// Git working directory state
    GitStatus,
    /// Current token usage/capacity/remaining
    TokenStatus,
}

impl SystemReminderType {
    /// Convert to string representation for type markers
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClaudeMd => "claudeMd",
            Self::Environment => "environment",
            Self::GitStatus => "gitStatus",
            Self::TokenStatus => "tokenStatus",
        }
    }

    /// Parse from string representation
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "claudeMd" => Some(Self::ClaudeMd),
            "environment" => Some(Self::Environment),
            "gitStatus" => Some(Self::GitStatus),
            "tokenStatus" => Some(Self::TokenStatus),
            _ => None,
        }
    }
}

/// Metadata pattern used to identify system reminders in messages.
/// Uses the same pattern as Claude CLI's P1A function.
///
/// Matches codelet's constants from system-reminders.ts:25-32
const SYSTEM_REMINDER_TAG: &str = "<system-reminder>";
const SYSTEM_REMINDER_TAG_END: &str = "</system-reminder>";

/// Type marker prefix used within system reminder content to identify the type.
const TYPE_MARKER_PREFIX: &str = "<!-- type:";
const TYPE_MARKER_SUFFIX: &str = " -->";

/// Create a system reminder tag wrapper
///
/// Matches codelet's createSystemReminder from context.ts:13-15
///
/// # Arguments
/// * `text` - Text to wrap
///
/// # Returns
/// Text wrapped in system-reminder tags
pub fn create_system_reminder(text: &str) -> String {
    format!(
        "{SYSTEM_REMINDER_TAG}\n{text}\n{SYSTEM_REMINDER_TAG_END}"
    )
}

/// Creates a supersession marker for replacement reminders.
///
/// CLI-013: When a reminder of the same type already exists, the new reminder
/// should contain this marker so the LLM knows to use the latest version.
///
/// # Arguments
/// * `reminder_type` - The type being superseded
///
/// # Returns
/// Supersession marker text
fn create_supersession_marker(reminder_type: SystemReminderType) -> String {
    format!(
        "This supersedes earlier {} reminder",
        reminder_type.as_str()
    )
}

/// Creates a system reminder message with embedded type marker.
///
/// Matches codelet's createSystemReminderContent from system-reminders.ts:37-42
///
/// # Arguments
/// * `reminder_type` - The type of system reminder
/// * `content` - The content to include
/// * `is_replacement` - If true, adds supersession marker
///
/// # Returns
/// Formatted system reminder with type marker
fn create_system_reminder_content(
    reminder_type: SystemReminderType,
    content: &str,
    is_replacement: bool,
) -> String {
    let supersession = if is_replacement {
        format!("\n{}", create_supersession_marker(reminder_type))
    } else {
        String::new()
    };

    format!(
        "{}\n{}{}{}\n{}{}\n{}",
        SYSTEM_REMINDER_TAG,
        TYPE_MARKER_PREFIX,
        reminder_type.as_str(),
        TYPE_MARKER_SUFFIX,
        content,
        supersession,
        SYSTEM_REMINDER_TAG_END
    )
}

/// Extracts the type from a system reminder message content, if present.
///
/// Matches codelet's extractTypeFromContent from system-reminders.ts:47-75
///
/// # Arguments
/// * `content` - Message content to parse
///
/// # Returns
/// SystemReminderType if found and valid, None otherwise
fn extract_type_from_content(content: &str) -> Option<SystemReminderType> {
    if !content.contains(TYPE_MARKER_PREFIX) {
        return None;
    }

    let start_idx = content.find(TYPE_MARKER_PREFIX)? + TYPE_MARKER_PREFIX.len();
    let end_idx = content[start_idx..].find(TYPE_MARKER_SUFFIX)? + start_idx;

    let type_str = &content[start_idx..end_idx];
    SystemReminderType::parse(type_str)
}

/// Checks if a message is a system reminder (any type).
///
/// Used for partition logic during compaction to separate system-reminders
/// from compactable messages.
///
/// # Arguments
/// * `msg` - Message to check
///
/// # Returns
/// true if message is a system reminder (has system-reminder tags and type marker)
pub fn is_system_reminder(msg: &Message) -> bool {
    match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => {
                t.text.contains(SYSTEM_REMINDER_TAG) && t.text.contains(TYPE_MARKER_PREFIX)
            }
            _ => false,
        },
        _ => false,
    }
}

/// Extracts the type from a Message if it's a system reminder.
fn get_reminder_type(msg: &Message) -> Option<SystemReminderType> {
    match msg {
        Message::User { content } => match content.first() {
            UserContent::Text(t) => extract_type_from_content(&t.text),
            _ => None,
        },
        _ => None,
    }
}

/// Partitions messages into system-reminders and compactable messages.
///
/// CLI-013: Only extracts the LATEST reminder of each type for preservation.
/// Earlier reminders of the same type are treated as compactable (will be
/// summarized away during compaction).
///
/// This is the core mechanism for preserving system-reminders through compaction:
/// 1. Separate LATEST system-reminders from compactable messages
/// 2. Summarize compactable messages (via LLM)
/// 3. Reconstruct: system-reminders + summary + recent messages
///
/// # Arguments
/// * `messages` - Message array to partition
///
/// # Returns
/// Tuple of (latest system-reminders only, compactable messages including old reminders)
pub fn partition_for_compaction(messages: &[Message]) -> (Vec<Message>, Vec<Message>) {
    use std::collections::HashMap;

    // First pass: find the LAST occurrence index of each reminder type
    let mut last_occurrence: HashMap<SystemReminderType, usize> = HashMap::new();
    for (i, msg) in messages.iter().enumerate() {
        if let Some(reminder_type) = get_reminder_type(msg) {
            last_occurrence.insert(reminder_type, i);
        }
    }

    // Second pass: partition - only keep reminders at their last occurrence index
    let mut system_reminders = Vec::new();
    let mut compactable = Vec::new();

    for (i, msg) in messages.iter().enumerate() {
        if let Some(reminder_type) = get_reminder_type(msg) {
            // Only preserve this reminder if it's the LAST one of its type
            if last_occurrence.get(&reminder_type) == Some(&i) {
                system_reminders.push(msg.clone());
            } else {
                // Older versions go to compactable (will be summarized away)
                compactable.push(msg.clone());
            }
        } else {
            // Non-reminder messages are compactable
            compactable.push(msg.clone());
        }
    }

    (system_reminders, compactable)
}

/// Checks if a message is a system reminder of a specific type.
///
/// Matches codelet's isSystemReminderOfType from system-reminders.ts:80-98
///
/// # Arguments
/// * `msg` - Message to check
/// * `reminder_type` - Type to match against
///
/// # Returns
/// true if message is a system reminder of the specified type
fn is_system_reminder_of_type(msg: &Message, reminder_type: SystemReminderType) -> bool {
    // Check if message is user role and extract content string
    let content_text = match msg {
        Message::User { content } => {
            // Get first item from OneOrMany
            match content.first() {
                UserContent::Text(text) => text.text,
                _ => return false,
            }
        }
        _ => return false,
    };

    // Check for system-reminder tags
    if !content_text.contains(SYSTEM_REMINDER_TAG) {
        return false;
    }

    // Extract and match type
    extract_type_from_content(&content_text) == Some(reminder_type)
}

/// Counts the number of system reminders of a specific type in message history.
///
/// Matches codelet's countSystemRemindersByType from system-reminders.ts:103-108
///
/// # Arguments
/// * `messages` - Message history to scan
/// * `reminder_type` - Type to count
///
/// # Returns
/// Number of reminders of the specified type
pub fn count_system_reminders_by_type(
    messages: &[Message],
    reminder_type: SystemReminderType,
) -> usize {
    messages
        .iter()
        .filter(|msg| is_system_reminder_of_type(msg, reminder_type))
        .count()
}

/// Checks if a reminder of the specified type already exists in messages.
///
/// CLI-013: Used to determine if supersession marker is needed.
///
/// # Arguments
/// * `messages` - Message history to check
/// * `reminder_type` - Type to look for
///
/// # Returns
/// true if a reminder of this type exists
fn has_reminder_of_type(messages: &[Message], reminder_type: SystemReminderType) -> bool {
    messages
        .iter()
        .any(|msg| is_system_reminder_of_type(msg, reminder_type))
}

/// Adds a system reminder to the conversation history.
///
/// CLI-013: Does NOT remove existing reminders of the same type.
/// Instead, appends the new reminder with a supersession marker if a reminder
/// of the same type already exists. This preserves the prompt cache prefix.
///
/// CRITICAL for prompt caching:
/// - Old reminders stay in place (prefix unchanged = cache valid)
/// - New reminders are APPENDED to the end
/// - Supersession marker tells LLM to use the latest version
/// - Cleanup happens during compaction (partition_for_compaction keeps only latest)
///
/// # Arguments
/// * `messages` - Current conversation history
/// * `reminder_type` - The type of system reminder
/// * `content` - The content to include in the system reminder
///
/// # Returns
/// New message array with the system reminder appended (old ones preserved)
pub fn add_system_reminder(
    messages: &[Message],
    reminder_type: SystemReminderType,
    content: &str,
) -> Vec<Message> {
    // Check if a reminder of this type already exists
    let is_replacement = has_reminder_of_type(messages, reminder_type);

    // Create the new system reminder message (with supersession marker if replacement)
    let reminder_content = create_system_reminder_content(reminder_type, content, is_replacement);
    let reminder_message = Message::User {
        content: OneOrMany::one(UserContent::text(&reminder_content)),
    };

    // APPEND the new reminder - DO NOT remove old ones (preserves prompt cache prefix)
    let mut result = messages.to_vec();
    result.push(reminder_message);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_message(content: &str) -> Message {
        Message::User {
            content: OneOrMany::one(UserContent::text(content)),
        }
    }

    #[test]
    fn test_system_reminder_type_as_str() {
        assert_eq!(SystemReminderType::ClaudeMd.as_str(), "claudeMd");
        assert_eq!(SystemReminderType::Environment.as_str(), "environment");
        assert_eq!(SystemReminderType::GitStatus.as_str(), "gitStatus");
        assert_eq!(SystemReminderType::TokenStatus.as_str(), "tokenStatus");
    }

    #[test]
    fn test_system_reminder_type_parse() {
        assert_eq!(
            SystemReminderType::parse("claudeMd"),
            Some(SystemReminderType::ClaudeMd)
        );
        assert_eq!(
            SystemReminderType::parse("environment"),
            Some(SystemReminderType::Environment)
        );
        assert_eq!(
            SystemReminderType::parse("gitStatus"),
            Some(SystemReminderType::GitStatus)
        );
        assert_eq!(
            SystemReminderType::parse("tokenStatus"),
            Some(SystemReminderType::TokenStatus)
        );
        assert_eq!(SystemReminderType::parse("invalid"), None);
    }

    #[test]
    fn test_create_system_reminder() {
        let text = "Test content";
        let reminder = create_system_reminder(text);
        assert!(reminder.contains("<system-reminder>"));
        assert!(reminder.contains("</system-reminder>"));
        assert!(reminder.contains(text));
    }

    #[test]
    fn test_create_system_reminder_content() {
        let content = "Token usage: 50%";
        // Test without supersession marker
        let reminder =
            create_system_reminder_content(SystemReminderType::TokenStatus, content, false);

        assert!(reminder.contains("<system-reminder>"));
        assert!(reminder.contains("<!-- type:tokenStatus -->"));
        assert!(reminder.contains(content));
        assert!(reminder.contains("</system-reminder>"));
        assert!(
            !reminder.contains("supersedes"),
            "First reminder should not have supersession marker"
        );
    }

    #[test]
    fn test_create_system_reminder_content_with_supersession() {
        let content = "Token usage: 75%";
        // Test WITH supersession marker (replacement)
        let reminder =
            create_system_reminder_content(SystemReminderType::TokenStatus, content, true);

        assert!(reminder.contains("<system-reminder>"));
        assert!(reminder.contains("<!-- type:tokenStatus -->"));
        assert!(reminder.contains(content));
        assert!(reminder.contains("</system-reminder>"));
        assert!(
            reminder.contains("supersedes"),
            "Replacement reminder should have supersession marker"
        );
        assert!(
            reminder.contains("tokenStatus"),
            "Should mention the type being superseded"
        );
    }

    #[test]
    fn test_extract_type_from_content() {
        let content =
            "<system-reminder>\n<!-- type:tokenStatus -->\nToken usage: 50%\n</system-reminder>";
        assert_eq!(
            extract_type_from_content(content),
            Some(SystemReminderType::TokenStatus)
        );

        let content_no_marker = "<system-reminder>\nToken usage: 50%\n</system-reminder>";
        assert_eq!(extract_type_from_content(content_no_marker), None);

        let content_invalid =
            "<system-reminder>\n<!-- type:invalid -->\nContent\n</system-reminder>";
        assert_eq!(extract_type_from_content(content_invalid), None);
    }

    #[test]
    fn test_is_system_reminder_of_type() {
        let reminder_content = create_system_reminder_content(
            SystemReminderType::TokenStatus,
            "Token usage: 50%",
            false,
        );
        let msg = create_test_message(&reminder_content);

        assert!(is_system_reminder_of_type(
            &msg,
            SystemReminderType::TokenStatus
        ));
        assert!(!is_system_reminder_of_type(
            &msg,
            SystemReminderType::Environment
        ));

        let regular_msg = create_test_message("Hello world");
        assert!(!is_system_reminder_of_type(
            &regular_msg,
            SystemReminderType::TokenStatus
        ));
    }

    #[test]
    fn test_count_system_reminders_by_type() {
        let reminder1 = create_test_message(&create_system_reminder_content(
            SystemReminderType::TokenStatus,
            "Token usage: 50%",
            false,
        ));
        let reminder2 = create_test_message(&create_system_reminder_content(
            SystemReminderType::Environment,
            "Platform: linux",
            false,
        ));
        let regular = create_test_message("Hello");

        let messages = vec![reminder1, regular.clone(), reminder2];

        assert_eq!(
            count_system_reminders_by_type(&messages, SystemReminderType::TokenStatus),
            1
        );
        assert_eq!(
            count_system_reminders_by_type(&messages, SystemReminderType::Environment),
            1
        );
        assert_eq!(
            count_system_reminders_by_type(&messages, SystemReminderType::GitStatus),
            0
        );
    }

    #[test]
    fn test_add_system_reminder_preserves_old_and_appends_new() {
        // CLI-013: Old reminder should be PRESERVED, new one APPENDED with supersession marker
        let old_reminder = create_test_message(&create_system_reminder_content(
            SystemReminderType::TokenStatus,
            "old content",
            false,
        ));
        let regular = create_test_message("Hello");

        let messages = vec![old_reminder, regular];

        let updated =
            add_system_reminder(&messages, SystemReminderType::TokenStatus, "new content");

        // CLI-013: Should have 3 messages (old reminder preserved, regular, new reminder appended)
        assert_eq!(updated.len(), 3);

        // First message should be old reminder (PRESERVED for prompt cache)
        assert!(is_system_reminder_of_type(
            &updated[0],
            SystemReminderType::TokenStatus
        ));
        if let Message::User { content } = &updated[0] {
            if let UserContent::Text(text) = content.first() {
                assert!(
                    text.text.contains("old content"),
                    "Old reminder should be preserved"
                );
            }
        }

        // Second message should be regular
        if let Message::User { content } = &updated[1] {
            if let UserContent::Text(text) = content.first() {
                assert_eq!(text.text, "Hello");
            }
        }

        // Third message should be new reminder (appended) with supersession marker
        assert!(is_system_reminder_of_type(
            &updated[2],
            SystemReminderType::TokenStatus
        ));
        if let Message::User { content } = &updated[2] {
            if let UserContent::Text(text) = content.first() {
                assert!(
                    text.text.contains("new content"),
                    "New reminder should have new content"
                );
                assert!(
                    text.text.contains("supersedes"),
                    "Replacement should have supersession marker"
                );
            }
        }
    }

    #[test]
    fn test_add_system_reminder_appends() {
        let regular = create_test_message("Hello");
        let messages = vec![regular];

        let updated = add_system_reminder(&messages, SystemReminderType::Environment, "env data");

        // Original message should be first (prefix preserved for cache)
        if let Message::User { content } = &updated[0] {
            if let UserContent::Text(text) = content.first() {
                assert_eq!(text.text, "Hello");
            }
        }

        // Reminder should be last (appended)
        assert!(is_system_reminder_of_type(
            &updated[1],
            SystemReminderType::Environment
        ));
    }

    #[test]
    fn test_multiple_reminder_types_coexist() {
        let messages = vec![];

        let with_env = add_system_reminder(&messages, SystemReminderType::Environment, "env");
        let with_token = add_system_reminder(&with_env, SystemReminderType::TokenStatus, "tokens");

        // Should have both reminders
        assert_eq!(with_token.len(), 2);
        assert_eq!(
            count_system_reminders_by_type(&with_token, SystemReminderType::Environment),
            1
        );
        assert_eq!(
            count_system_reminders_by_type(&with_token, SystemReminderType::TokenStatus),
            1
        );
    }
}
