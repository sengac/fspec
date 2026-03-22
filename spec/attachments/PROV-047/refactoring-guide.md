# PROV-047: Refactoring Guide — Loop Detection Extraction

## Refactoring Opportunity

PROV-047 is small (2 story points) but provides a clean extraction vehicle. The response loop detector belongs in its own module, not wedged into the god function.

## What to Create: `loop_detection.rs` (~50 lines)

```rust
/// Minimum response length to consider for loop detection.
/// Short responses like "OK" or "Done" are not considered loops.
const MIN_LOOP_DETECTION_LENGTH: usize = 20;

/// Number of recent assistant messages to compare against.
const LOOP_LOOKBACK_DEPTH: usize = 2;

/// Detects if the model is repeating itself by comparing the latest
/// assistant text against recent assistant messages in history.
///
/// Returns true if a repetitive response is detected.
pub fn detect_response_loop(
    current_text: &str,
    messages: &[Message],
) -> bool {
    // Skip short responses
    let normalized = normalize_whitespace(current_text);
    if normalized.len() < MIN_LOOP_DETECTION_LENGTH {
        return false;
    }

    // Walk messages in reverse, find last N assistant texts
    let mut assistant_count = 0;
    for msg in messages.iter().rev() {
        if let Message::Assistant { content } = msg {
            let prev_text = extract_text_from_assistant(content);
            let prev_normalized = normalize_whitespace(&prev_text);

            if prev_normalized == normalized {
                return true;
            }

            assistant_count += 1;
            if assistant_count >= LOOP_LOOKBACK_DEPTH {
                break;
            }
        }
    }

    false
}

fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_text_from_assistant(content: &OneOrMany<AssistantContent>) -> String {
    content.iter()
        .filter_map(|c| match c {
            AssistantContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
```

## Integration Point

In `stream_processor.rs` (after PROV-043 extraction), inside the `FinalResponse` handling:

```rust
Some(Ok(MultiTurnStreamItem::FinalResponse(resp))) => {
    handle_final_response(&ctx.assistant_text, &mut session.messages)?;

    // PROV-047: Check for response loop before finalizing
    if detect_response_loop(&ctx.assistant_text, &session.messages) {
        output.emit_status("Repetitive response detected — breaking loop");
        return StreamOutcome::Completed {
            stop_reason: Some("loop_detected".to_string()),
        };
    }

    // ... Gemini continuation check, thinking exhaustion check ...
}
```

If PROV-043 hasn't been done yet, this goes in `stream_loop.rs` at ~line 1587, between `handle_final_response()` and the Gemini continuation check.

## Why Its Own Module

- The detection logic is **pure** (no side effects, no I/O)
- It's independently testable
- It will likely grow (fuzzy matching, similarity thresholds, tool call loop detection)
- Adding it inline to stream_loop.rs would make the god function even larger

## Tests

```rust
#[test]
fn detects_exact_duplicate() {
    let messages = vec![
        Message::User { content: user_text("Hello") },
        Message::Assistant { content: assistant_text("The answer is 42.") },
    ];
    assert!(detect_response_loop("The answer is 42.", &messages));
}

#[test]
fn ignores_short_responses() {
    let messages = vec![
        Message::Assistant { content: assistant_text("OK") },
    ];
    assert!(!detect_response_loop("OK", &messages));
}

#[test]
fn normalizes_whitespace() {
    let messages = vec![
        Message::Assistant { content: assistant_text("The  answer\nis  42.") },
    ];
    assert!(detect_response_loop("The answer is 42.", &messages));
}

#[test]
fn no_false_positive_on_different_text() {
    let messages = vec![
        Message::Assistant { content: assistant_text("Something completely different and long enough") },
    ];
    assert!(!detect_response_loop("Another response that is long enough to check", &messages));
}
```

## Estimated Impact

- **Lines added to stream_loop.rs**: 0 (5 lines in stream_processor.rs or 5 lines at integration point)
- **New module**: `loop_detection.rs` (~50 lines)
- **Pure function**: no side effects, trivially testable
