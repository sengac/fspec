use super::*;

/// Test StreamChunk::user_input creates correct variant
#[test]
fn test_user_input_chunk_creation() {
    let user_message = "Hello, can you help me with this task?";
    let chunk = StreamChunk::user_input(user_message.to_string());

    match chunk {
        StreamChunk::UserInput { text } => {
            assert_eq!(text, user_message);
        }
        _ => panic!("Expected UserInput variant"),
    }
}

/// Test empty user input is handled correctly
#[test]
fn test_empty_user_input_chunk() {
    let chunk = StreamChunk::user_input(String::new());

    match chunk {
        StreamChunk::UserInput { text } => {
            assert_eq!(text, "");
        }
        _ => panic!("Expected UserInput variant"),
    }
}

/// Test user input with multiline content
#[test]
fn test_multiline_user_input_chunk() {
    let multiline_message =
        "First line\nSecond line\nThird line with code:\n```rust\nfn main() {}\n```";
    let chunk = StreamChunk::user_input(multiline_message.to_string());

    match chunk {
        StreamChunk::UserInput { text } => {
            assert_eq!(text, multiline_message);
            assert!(text.contains('\n'));
        }
        _ => panic!("Expected UserInput variant"),
    }
}

/// Test user input with special characters
#[test]
fn test_special_characters_in_user_input() {
    let special_message = "Test with émojis 🎉 and symbols: <>&\"' and unicode: 你好世界";
    let chunk = StreamChunk::user_input(special_message.to_string());

    match chunk {
        StreamChunk::UserInput { text } => {
            assert_eq!(text, special_message);
        }
        _ => panic!("Expected UserInput variant"),
    }
}

/// Test UserInput chunk is distinct from Text chunk
#[test]
fn test_user_input_distinct_from_text() {
    let message = "Same content";
    let user_chunk = StreamChunk::user_input(message.to_string());
    let text_chunk = StreamChunk::text(message.to_string());

    match (&user_chunk, &text_chunk) {
        (StreamChunk::UserInput { .. }, StreamChunk::Text { .. }) => {
            // They are different variants - good!
        }
        _ => panic!("Expected different variants"),
    }
}

/// NAPI-010: Test SessionStateChange for compacting state
#[test]
fn test_session_state_change_compacting() {
    let chunk = StreamChunk::session_state_change(SessionState::Compacting);

    match chunk {
        StreamChunk::SessionStateChange { state } => {
            assert_eq!(state, SessionState::Compacting);
        }
        _ => panic!("Expected SessionStateChange variant"),
    }
}

/// NAPI-010: Test UserNotification with severity
#[test]
fn test_user_notification_with_severity() {
    let chunk = StreamChunk::user_notification(
        "API rate limit exceeded".to_string(),
        NotificationSeverity::Warning,
    );

    match chunk {
        StreamChunk::UserNotification { message, severity } => {
            assert_eq!(message, "API rate limit exceeded");
            assert_eq!(severity, NotificationSeverity::Warning);
        }
        _ => panic!("Expected UserNotification variant"),
    }
}
