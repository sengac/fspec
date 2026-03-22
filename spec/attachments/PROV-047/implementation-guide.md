# PROV-047: Response Loop Detection — Implementation Guide

## Problem

If the model enters a repetitive response loop (same assistant message 2+ times consecutively), fspec will loop indefinitely until the user interrupts or context fills up. This wastes tokens and time.

## VTCode Reference

### check_for_response_loop (`vtcode-core/src/core/agent/completion.rs` lines 68–108)

```rust
pub fn check_for_response_loop(response_text: &str, session_state: &mut AgentSessionState) -> bool {
    if response_text.len() < 10 {
        return false;
    }

    // Normalize whitespace for comparison
    let normalized_current = response_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Check against last 2 assistant messages (skipping the current one just added)
    let repeated = session_state
        .messages
        .iter()
        .rev()
        .filter(|m| m.role == MessageRole::Assistant)
        .skip(1)   // Skip the message we just added
        .take(2)    // Check last 2 prior assistant messages
        .any(|m| {
            let normalized_prev = m
                .content
                .as_text()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            normalized_prev == normalized_current
        });

    if repeated {
        let warning = "Repetitive assistant response detected. Breaking potential loop.".to_string();
        session_state.warnings.push(warning);
        session_state.consecutive_idle_turns =
            session_state.consecutive_idle_turns.saturating_add(1);
        return true;
    }

    false
}
```

### Usage in execute.rs (lines 519–534)

```rust
if check_for_response_loop(response.content_text(), &mut controller.state) {
    self.runner_println(format_args!(
        "[{}] {}",
        self.agent_type,
        style("Repetitive assistant response detected. Breaking potential loop.")
            .red().bold()
    ));
    controller.state.outcome = TaskOutcome::LoopDetected;
    controller.state.record_turn(&turn_started_at, &mut turn_recorded);
    break;
}
```

### Tests (`completion.rs` lines 133–155)

```rust
#[test]
fn response_loop_ignores_current_assistant_message() {
    let repeated_response = "The task is complete";
    let mut state = AgentSessionState::new("session".to_string(), 8, 4, 128_000);
    state.messages.push(Message::assistant(repeated_response.to_string()));
    // Only 1 message — no prior duplicate to match
    assert!(!check_for_response_loop(repeated_response, &mut state));
}

#[test]
fn response_loop_still_detects_prior_duplicate_assistant_message() {
    let repeated_response = "The task is complete";
    let mut state = AgentSessionState::new("session".to_string(), 8, 4, 128_000);
    state.messages.push(Message::assistant(repeated_response.to_string()));
    state.messages.push(Message::assistant(repeated_response.to_string()));
    // 2 prior copies → should detect loop
    assert!(check_for_response_loop(repeated_response, &mut state));
}
```

## Proposed Implementation for fspec

### 1. Add detection function to stream_loop.rs (or a new helpers module)

```rust
/// PROV-047: Detect repetitive assistant responses.
///
/// Compares normalized (whitespace-collapsed) current response against
/// the last 2 assistant messages in session history. Returns true if
/// an exact duplicate is found.
pub fn is_response_loop(assistant_text: &str, messages: &[rig::message::Message]) -> bool {
    if assistant_text.len() < 20 {
        return false; // Too short to meaningfully detect loops
    }

    let normalized_current: String = assistant_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Walk backward through messages, find last 2 assistant messages
    let mut assistant_count = 0;
    for msg in messages.iter().rev() {
        if let rig::message::Message::Assistant { content } = msg {
            // Extract text from assistant content
            let msg_text: String = content.iter().filter_map(|c| {
                if let rig::message::AssistantContent::Text(t) = c {
                    Some(t.text.as_str())
                } else {
                    None
                }
            }).collect::<Vec<_>>().join(" ");

            let normalized_prev: String = msg_text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

            if normalized_prev == normalized_current {
                return true;
            }

            assistant_count += 1;
            if assistant_count >= 2 {
                break;
            }
        }
    }

    false
}
```

### 2. Call from FinalResponse handler (stream_loop.rs ~line 1587)

Insert after `handle_final_response()` and before the thinking exhaustion check:

```rust
// Normal case: add assistant text to history and finish
handle_final_response(&assistant_text, &mut session.messages)?;

// PROV-047: Check for response loop before proceeding
if is_response_loop(&assistant_text, &session.messages) {
    warn!("PROV-047: Repetitive assistant response detected — breaking loop");
    output.emit_status(
        "Repetitive response detected. The model appears to be stuck in a loop."
    );
    // Don't retry — just emit done and break
    output.emit_done_with_stop_reason(Some("loop_detected".to_string()));
    break;
}
```

### 3. Tests

```rust
#[test]
fn response_loop_not_detected_for_first_message() {
    let messages = vec![];
    assert!(!is_response_loop("Hello world", &messages));
}

#[test]
fn response_loop_not_detected_for_short_text() {
    let messages = vec![
        Message::Assistant { content: OneOrMany::one(AssistantContent::text("ok")) },
    ];
    assert!(!is_response_loop("ok", &messages));
}

#[test]
fn response_loop_detected_for_duplicate() {
    let text = "I'll help you with that task. Let me check the files.";
    let messages = vec![
        Message::Assistant { content: OneOrMany::one(AssistantContent::text(text)) },
    ];
    assert!(is_response_loop(text, &messages));
}

#[test]
fn response_loop_detected_with_whitespace_normalization() {
    let messages = vec![
        Message::Assistant {
            content: OneOrMany::one(AssistantContent::text("Hello   world\n\nfoo"))
        },
    ];
    assert!(is_response_loop("Hello world\nfoo", &messages));
}

#[test]
fn response_loop_not_detected_for_different_text() {
    let messages = vec![
        Message::Assistant {
            content: OneOrMany::one(AssistantContent::text("First response"))
        },
    ];
    assert!(!is_response_loop("Second response", &messages));
}
```

## Estimated Effort: 2 story points
