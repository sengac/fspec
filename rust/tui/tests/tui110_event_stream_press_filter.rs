//! TUI-110 — Windows key-doubling fix: shared event stream Press filter.
//!
//! Feature: spec/features/windows-key-release-duplication.feature
//!
//! `tui::create_event_stream` reads from the real terminal stdin, so the
//! stream-level scenarios are pinned by source-shape assertions: the
//! `Event::Key` arm must only yield `TuiEvent::Key` for
//! `KeyEventKind::Press` events (ratatui#347 / crossterm#772 — Windows
//! emits both Press and Release; the documented fix is a Press-only
//! filter at the event source).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

fn events_src() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("events.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Scenario: The shared event stream emits key press events
#[test]
fn the_shared_event_stream_emits_key_press_events() {
    // @step Given the shared TUI event stream is running
    let src = events_src();
    // @step When a key event with kind Press is delivered by the terminal
    // (the Press path is the default for crossterm `KeyEvent::new`, so
    // the arm yields for every Press event)
    // @step Then the stream yields a Key event for that key
    assert!(
        src.contains("yield TuiEvent::Key("),
        "events.rs must still yield TuiEvent::Key for key events"
    );
    assert!(
        src.contains("KeyEventKind::Press"),
        "events.rs must reference KeyEventKind::Press as the filter gate"
    );
}

/// Scenario: The shared event stream does not emit key release events
#[test]
fn the_shared_event_stream_does_not_emit_key_release_events() {
    // @step Given the shared TUI event stream is running
    let src = events_src();
    // @step When a key event with kind Release is delivered by the terminal
    // (Release events must be filtered out before yielding)
    // @step Then the stream yields no Key event for that key
    // The filter gate must be a Press-equality guard in the Key arm —
    // i.e. the `yield TuiEvent::Key(` line must sit inside a guard that
    // checks `kind == KeyEventKind::Press` (or `!= ...Release` is NOT
    // the gate, since that would admit Repeat).
    let key_arm = src
        .split("Event::Key(")
        .nth(1)
        .unwrap_or_else(|| panic!("events.rs must match on Event::Key"));
    assert!(
        key_arm.contains("KeyEventKind::Press"),
        "the Event::Key arm must gate the yield on KeyEventKind::Press"
    );
    assert!(
        !key_arm.contains("KeyEventKind::Release"),
        "the Event::Key arm must NOT use `!= Release` (that admits Repeat)"
    );
}

/// Scenario: The shared event stream does not emit key repeat events
#[test]
fn the_shared_event_stream_does_not_emit_key_repeat_events() {
    // @step Given the shared TUI event stream is running
    let src = events_src();
    // @step When a key event with kind Repeat is delivered by the terminal
    // (Repeat events must be filtered out — only Press passes)
    // @step Then the stream yields no Key event for that key
    // The gate must be an equality check (`== KeyEventKind::Press`),
    // which excludes both Release and Repeat.
    let key_arm = src
        .split("Event::Key(")
        .nth(1)
        .unwrap_or_else(|| panic!("events.rs must match on Event::Key"));
    assert!(
        key_arm.contains("== KeyEventKind::Press"),
        "the Event::Key arm must use an equality guard `== KeyEventKind::Press` so Repeat is excluded"
    );
}

/// Scenario: The shared event stream still emits paste events
#[test]
fn the_shared_event_stream_still_emits_paste_events() {
    // @step Given the shared TUI event stream is running
    let src = events_src();
    // @step When a paste event is delivered by the terminal
    // @step Then the stream yields a Paste event with the pasted text
    assert!(
        src.contains("Event::Paste(pasted)"),
        "events.rs must still match Event::Paste"
    );
    assert!(
        src.contains("yield TuiEvent::Paste("),
        "events.rs must still yield TuiEvent::Paste"
    );
}
