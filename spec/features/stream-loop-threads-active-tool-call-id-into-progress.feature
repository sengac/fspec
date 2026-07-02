@done
@BUG-149
@tool-call
@streaming
@rust
@tool-display
@cli
Feature: Stream loop threads active tool call id into progress
  """
  The stream loop tracks the active tool_call_id (set on ToolCall, cleared on ToolResult) and the progress callback emits it instead of an empty string. Serial tool execution within a turn makes a single active id unambiguous.
  """

  Background: User Story
    As a developer using the fspec-tui
    I want the stream loop to tag live tool progress with the active tool call id
    So that the TUI can fold live output into the correct tool-call card

  Scenario: Progress emitted during tool execution carries the active tool_call_id
    Given a tool call "tc-1" has started and is the active tool call
    When tool progress is emitted through the stream-loop progress callback
    Then the emitted ToolProgress carries tool_call_id "tc-1"

  Scenario: Active tool_call_id is set on ToolCall and cleared on ToolResult
    Given no tool call is active
    When a tool call "tc-1" starts
    Then the active tool_call_id is "tc-1"
    When the tool call "tc-1" produces its result
    Then no tool_call_id is active

  Scenario: Stray progress with no active tool call is dropped without panic
    Given no tool call is active
    When tool progress is emitted through the stream-loop progress callback
    Then no panic occurs
    Then the card "tc-1" body is unchanged
