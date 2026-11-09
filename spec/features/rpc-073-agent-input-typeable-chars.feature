@done
@RPC-073
@tui
@agent-view
@bug
Feature: RPC-073 Agent Input Typeable Chars
  """
  Bug 2: ? and q were trapped globally by App::handle_app_shortcut in
  rust/fspec-tui/src/app/events.rs:46-54. They were dispatched at Stage 2
  BEFORE the Compositor and Navigator, so the AgentView's MultiLineInput
  never received them.

  TS reference: src/tui/input/InputManager.tsx routes text input at
  InputPriority.MEDIUM (500) ABOVE view-level shortcuts at
  InputPriority.LOW (200), so typing ? into the AgentModal input field is
  always type-able.

  Fix: invert the dispatch order in App::handle_event:
  1. DisconnectDialog (critical, unchanged — RPC-011 CR-1)
  2. Compositor.handle_event
  3. Navigator.handle_event
  4. handle_app_shortcut (?, q, Ctrl+D) — only if 2 and 3 returned Ignored

  Reference: spec/attachments/RPC-073/research-bug2-key-dispatch-ts-vs-rust.md
  TUI-132 (2026-11-09): the DisconnectDialog stage is removed (never shown since RPC-416); dispatch order is Compositor, Navigator, app shortcuts.
  """

  Background: User Story
    As a fspec user driving the Rust binary
    I want every printable character (including ? and q) to land in the AgentView input buffer when the input is focused
    So that I can compose any message including questions and the letter q

  Scenario: Typing ? while the AgentView input is focused appends ? to the buffer and does not open the HelpDialog
    Given an App is constructed with active_view = Agent and a focused MultiLineInput with empty buffer
    When the app handles a KeyCode::Char('?') event with KeyModifiers::NONE
    Then the AgentView input buffer contains the literal '?' character
    Then the Compositor does not contain a HelpDialog layer

  Scenario: Typing q while the AgentView input is focused appends q to the buffer and does not quit the app
    Given an App is constructed with active_view = Agent and a focused MultiLineInput with empty buffer
    When the app handles a KeyCode::Char('q') event with KeyModifiers::NONE
    Then the AgentView input buffer contains the literal 'q' character
    Then App::should_quit remains false

  Scenario: Pressing ? while the BoardView is focused still opens the HelpDialog
    Given an App is constructed with active_view = Board
    When the app handles a KeyCode::Char('?') event with KeyModifiers::NONE
    Then the Compositor contains a HelpDialog layer at the top of the stack

  Scenario: Pressing q while the BoardView is focused still quits the app
    Given an App is constructed with active_view = Board and no critical dialog topmost
    When the app handles a KeyCode::Char('q') event with KeyModifiers::NONE
    Then App::should_quit is set to true

  Scenario: Pressing Ctrl+D while the AgentView input is focused still quits the app
    Given an App is constructed with active_view = Agent and a focused MultiLineInput
    When the app handles a KeyCode::Char('d') event with KeyModifiers::CONTROL
    Then App::should_quit is set to true
