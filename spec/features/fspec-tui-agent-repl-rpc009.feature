@done
@tui
@rust
@infrastructure
@parity
@rpc
@RPC-009
@critical
Feature: Agent REPL view (RPC-009)
  """
  Two sub-areas: (a) scrollback rendered as `Paragraph::new(Text::from_iter(...)).scroll((y_offset, 0)).wrap(Wrap { trim: false })` against an alt-screen Rect (NOT a virtualised list, NOT a custom BubbleList, NOT a HistoryCell trait — those are RPC-002 follow-ons); (b) single-line input rendered as `Paragraph::new(input.value()).block(Block::default().borders(Borders::ALL))` with cursor positioned via `frame.set_cursor_position` based on `input.visual_cursor()` from the `tui_input::Input` field. State: `AgentReplView { active_session: Option<SessionId>, scrollback: Vec<RenderedChunk>, input: tui_input::Input, scroll_offset: u16, stick_to_bottom: bool, focused: bool, action_tx: UnboundedSender<Action> }` where `RenderedChunk { seq: u64, lines: Vec<Line<'static>> }` (oatmeal-style nod, no caching machinery). chunks_rx subscriber filters by active session id — chunks for OTHER sessions are dropped before becoming Action::ChunkReceived. KeyCode::Enter on non-empty input emits Action::InputSubmitted then `input.reset()`; KeyCode::Char('c') with KeyModifiers::CONTROL emits Action::Interrupt; everything else forwards to `input.handle_event(event.into())`. tui-input is the ONLY new production dependency in this card.
  """

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want AgentReplView to render a scrollback Paragraph plus a single-line tui-input box, append RenderedChunks for the active session, dispatch send_input on Enter and Action::Interrupt on Ctrl+C, hold a stick_to_bottom flag (tenere pattern), and accept input only when focused
    So that the RIGHT pane of the basic frontend reads `backend.chunks_rx()` filtered by active session and exposes a working REPL on either transport

  Scenario: Action::SessionCreated sets the active_session
    Given a freshly constructed AgentReplView with active_session = None
    When the view receives `Action::SessionCreated(SessionId("s-1"))` via `update`
    Then the view's active_session field equals `Some(SessionId("s-1"))`
    And the view's scrollback is still empty
    And the view's stick_to_bottom flag is true

  Scenario: Action::ChunkReceived for the active session is appended to the scrollback
    Given an AgentReplView with active_session = Some(SessionId("s-1")) and an empty scrollback
    When the view receives `Action::ChunkReceived(SessionId::new("s-1"), StreamChunk::text("Hello!".into()))` via `update`
    Then the view's scrollback length increases to 1
    And the rendered buffer's right pane contains the substring "Hello!"

  Scenario: Action::ChunkReceived for a DIFFERENT session is silently dropped
    Given an AgentReplView with active_session = Some(SessionId("s-1")) and an empty scrollback
    When the view receives `Action::ChunkReceived(SessionId::new("s-other"), StreamChunk::text("not for us".into()))` via `update`
    Then the view's scrollback length is still 0
    And the rendered buffer's right pane does NOT contain the substring "not for us"

  Scenario: Enter on a non-empty input emits Action::InputSubmitted then resets the input
    Given a focused AgentReplView with active_session = Some(SessionId("s-1")) and the input value "hi"
    When the view processes a synthetic Key(Enter) event
    Then the action bus receives `Action::InputSubmitted("hi".into())`
    And the view's input.value() is the empty string
    And the view's input.visual_cursor() is 0

  Scenario: Enter on an empty input does NOT emit Action::InputSubmitted
    Given a focused AgentReplView with the input value ""
    When the view processes a synthetic Key(Enter) event
    Then the action bus receives no `Action::InputSubmitted`
    And handle_event returns `EventResult::Ignored(None)`

  Scenario: Ctrl+C while focused emits Action::Interrupt
    Given a focused AgentReplView with active_session = Some(SessionId("s-1"))
    When the view processes a synthetic Key('c') event with `KeyModifiers::CONTROL`
    Then the action bus receives `Action::Interrupt`
    And the App's `should_quit` flag is unchanged

  Scenario: Input character keys are forwarded to tui-input only when focused
    Given a focused AgentReplView with the input value ""
    When the view processes synthetic Key('h'), Key('i') events with `KeyModifiers::NONE`
    Then the view's input.value() equals "hi"
    And the view's input.visual_cursor() equals 2

  Scenario: Input character keys are ignored when the view is not focused
    Given an AgentReplView with `focused = false` and the input value ""
    When the view processes a synthetic Key('h') event
    Then the view's input.value() is still ""
    And handle_event returns `EventResult::Ignored(None)`

  Scenario: stick_to_bottom is true by default and follows new chunks
    Given a freshly constructed AgentReplView
    Then the view's stick_to_bottom flag is true
    When the view receives ten Action::ChunkReceived messages whose combined rendered lines exceed the visible scrollback height
    Then the view's stick_to_bottom flag is still true
    And the rendered scrollback shows the most recent chunk on the bottom row of the scrollback area

  Scenario: stick_to_bottom flips false when the user scrolls up and back to true on reaching the end
    Given an AgentReplView whose scrollback contains 50 chunks and stick_to_bottom = true
    When the view processes a synthetic PageUp event while focused
    Then the view's stick_to_bottom flag is false
    And subsequent Action::ChunkReceived messages append to scrollback without moving the viewport
    When the view processes synthetic PageDown events until scroll_offset reaches the end of the scrollback
    Then the view's stick_to_bottom flag is true again
