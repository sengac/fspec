@done
@navigation
@tui
@RPC-023
Feature: MouseTrackingToggle — TUI-078 native text-selection scaffolding
  """
  Decision (Q6): MouseTrackingToggle is generic over W: Write + Send with `with_stdout()` as the production constructor; tests inject Vec<u8> and assert exact escape bytes.
  Decision (Q9): BoardView does NOT opt into TUI-078 button-press (native text selection deferred to RPC-019 AgentView/VirtualList).
  RPC-023 only builds the scaffolding; RPC-019 wires it into VirtualList scrollback for real.
  """

  Background: User Story
    As a future consumer of the MouseTrackingToggle (RPC-019 VirtualList scrollback)
    I want a tested, writer-injectable debounced toggle that disables/re-enables crossterm mouse capture
    So that native terminal text selection can coexist with mouse-wheel scrolling

  Scenario: MouseTrackingToggle::temporarily_disable writes DisableMouseCapture bytes
    Given a MouseTrackingToggle constructed with a Vec<u8> writer
    And an UnboundedSender<Action> bound to a test receiver
    When temporarily_disable is called
    Then the Vec<u8> writer contains the exact DisableMouseCapture escape bytes "\x1b[?1006l\x1b[?1000l"
    And is_disabled() returns true

  Scenario: MouseTrackingToggle::re_enable writes EnableMouseCapture bytes once
    Given a MouseTrackingToggle whose temporarily_disable has already run against a Vec<u8> writer
    When re_enable is called
    Then the writer's tail bytes match the EnableMouseCapture escape "\x1b[?1000h\x1b[?1006h"
    And is_disabled() returns false
    When re_enable is called a second time while already enabled
    Then no further bytes are written to the writer

  Scenario: MouseTrackingToggle::Drop re-enables capture when still disabled
    Given a MouseTrackingToggle constructed with a Vec<u8> writer
    And temporarily_disable has been called so disabled is true
    When the toggle is dropped
    Then the EnableMouseCapture escape bytes are written to the writer during Drop

  Scenario: Repeated temporarily_disable restarts the debounce timer
    Given a MouseTrackingToggle with tokio::time paused
    When temporarily_disable is called
    And four seconds of virtual time elapse
    And temporarily_disable is called again
    And another four seconds of virtual time elapse
    Then no Action::ReEnableMouseTracking has been emitted yet
    When two more seconds of virtual time elapse
    Then exactly one Action::ReEnableMouseTracking is delivered to the receiver
