@done
@rust
@tui
@agent-view
@TUI-110
Feature: Key input doubles on Windows (crossterm emits Press + Release)
  """
  Shared stream filter in tui::create_event_stream (rust/tui/src/events.rs): the Event::Key match arm only yields TuiEvent::Key for KeyEventKind::Press events; Paste and Resize pass through unchanged. Known upstream references: ratatui#347, ratatui#1810, crossterm#772 — Windows console delivers both Press and Release; the documented fix is a Press-only filter at the event source.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The TUI processes ONLY KeyEventKind::Press key events; KeyEventKind::Release and KeyEventKind::Repeat are ignored everywhere (they are only ever generated on Windows / under the kitty enhancement protocol).
  #   2. The central App::run event loop (fspec-tui/src/app/events.rs) drops any Event::Key whose kind is not KeyEventKind::Press before calling App::handle_event, so no view, dialog, or app-shortcut path can ever see a Release/Repeat event.
  #   3. The shared event stream tui::create_event_stream (rust/tui/src/events.rs) yields TuiEvent::Key ONLY for KeyEventKind::Press events — Release/Repeat key events are never emitted to consumers (CLI stream loop, any future consumer).
  #   4. The two existing `key.kind != KeyEventKind::Release` guards in fspec-tui/src/app/events.rs (app-shortcut stage and DisconnectDialog stage) are normalized to `key.kind != KeyEventKind::Press` so they match the central filter exactly (they currently also admit Repeat events).
  #   5. Existing per-view KeyEventKind::Press filters (agent dispatch.rs RPC-402, multiline_input.rs, init_selector.rs) are PRESERVED as defense-in-depth for direct widget callers — the central filter is additive, not a replacement.
  #
  # EXAMPLES:
  #   1. Windows Terminal: user presses the Down arrow once in the board view and the selection moves down exactly one row (before the fix it skipped two rows, looking like the key was held down).
  #   2. Windows Terminal: user presses ? once in the board view and the Help dialog opens exactly once (before the fix the second synthetic ? could have been seen by a lower-priority handler after the dialog consumed the first).
  #   3. Linux/macOS: typing and navigating behaves exactly as before — no key is dropped, no new filtering side effects, since those platforms only ever generated Press events.
  #
  # ========================================
  Background: User Story
    As a TUI user on Windows
    I want to press keys in the ratatui TUI (board, dialogs, agent view) and have each keystroke registered exactly once
    So that the interface does not appear to have a stuck/repeat key on Windows Terminal or cmd

  # ========================================
  # CENTRAL APP EVENT LOOP (App::run / handle_event)
  # ========================================
  Scenario: A single Down-arrow press moves the board selection exactly one row
    Given the TUI is running in the board view with a work unit selected
    When a Down-arrow key event with kind Press arrives at the app event loop
    And a Down-arrow key event with kind Release arrives at the app event loop
    Then the board selection has moved down exactly one row

  Scenario: A key release event is dropped by the central app event loop
    Given the TUI is running in the board view with a work unit selected
    When a Down-arrow key event with kind Release arrives at the app event loop
    Then the board selection has not moved

  Scenario: A key repeat event is dropped by the central app event loop
    Given the TUI is running in the board view with a work unit selected
    When a Down-arrow key event with kind Repeat arrives at the app event loop
    Then the board selection has not moved

  Scenario: A single ? press opens the Help dialog exactly once
    Given the TUI is running in the board view with no dialog open
    When a ? key event with kind Press arrives at the app event loop
    And a ? key event with kind Release arrives at the app event loop
    Then the Help dialog is open
    And the Help dialog is the only dialog open

  Scenario: A ? release event does not open the Help dialog
    Given the TUI is running in the board view with no dialog open
    When a ? key event with kind Release arrives at the app event loop
    Then no dialog is open

  Scenario: A ? repeat event does not open the Help dialog
    Given the TUI is running in the board view with no dialog open
    When a ? key event with kind Repeat arrives at the app event loop
    Then no dialog is open

  Scenario: Press key events still flow to views and dialogs unchanged
    Given the TUI is running in the board view with a work unit selected
    When a Down-arrow key event with kind Press arrives at the app event loop
    Then the board selection has moved down exactly one row

  # ========================================
  # APP-LEVEL SHORTCUT GUARDS (normalized to Press)
  # ========================================
  Scenario: A repeat ? event does not trigger the app-level Help shortcut
    Given the TUI is running in the board view with no dialog open
    When a ? key event with kind Repeat arrives at the app event loop
    Then no dialog is open

  Scenario: A repeat q event does not quit the Disconnect dialog
    Given the Disconnect dialog is open
    When a q key event with kind Repeat arrives at the app event loop
    Then the app has not quit
    And the Disconnect dialog is still open
