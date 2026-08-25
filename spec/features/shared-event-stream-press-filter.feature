@done
@rust
@tui
@TUI-110
Feature: Shared TUI event stream filters to Press-only key events
  """
  Shared stream filter in tui::create_event_stream (rust/tui/src/events.rs): the Event::Key match arm only yields TuiEvent::Key for KeyEventKind::Press events; Paste and Resize pass through unchanged. Known upstream references: ratatui#347, ratatui#1810, crossterm#772 — Windows console delivers both Press and Release; the documented fix is a Press-only filter at the event source.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The shared event stream tui::create_event_stream (rust/tui/src/events.rs) yields TuiEvent::Key ONLY for KeyEventKind::Press events — Release/Repeat key events are never emitted to consumers (CLI stream loop, any future consumer).
  #   2. Paste and Resize events pass through the stream unchanged.
  #
  # EXAMPLES:
  #   1. Windows Terminal: the CLI interactive stream loop only ever sees Press key events from the shared stream, so an Esc press is registered exactly once.
  #   2. Linux/macOS: the shared stream behaves exactly as before since those platforms only ever generated Press events.
  #
  # ========================================
  Background: User Story
    As a TUI user on Windows
    I want the shared event stream to deliver each keystroke exactly once
    So that consumers (the CLI interactive loop, any future consumer) never see a doubled key

  # ========================================
  # SHARED EVENT STREAM (tui::create_event_stream)
  # ========================================
  Scenario: The shared event stream emits key press events
    Given the shared TUI event stream is running
    When a key event with kind Press is delivered by the terminal
    Then the stream yields a Key event for that key

  Scenario: The shared event stream does not emit key release events
    Given the shared TUI event stream is running
    When a key event with kind Release is delivered by the terminal
    Then the stream yields no Key event for that key

  Scenario: The shared event stream does not emit key repeat events
    Given the shared TUI event stream is running
    When a key event with kind Repeat is delivered by the terminal
    Then the stream yields no Key event for that key

  Scenario: The shared event stream still emits paste events
    Given the shared TUI event stream is running
    When a paste event is delivered by the terminal
    Then the stream yields a Paste event with the pasted text
