@watcher
@codelet
@WATCH-005
Feature: Watcher Agent Loop with Dual Input

  """
  Create WatcherState enum { Idle, Observing, Processing } in session_manager.rs
  Create ObservationBuffer struct to accumulate StreamChunks with timestamp tracking
  Add is_natural_breakpoint() function to detect TurnComplete, ToolResult, or silence timeout
  Add format_evaluation_prompt() function to create prompt from accumulated observations and role context
  Watcher loop is internal to session_manager.rs - not exposed via NAPI yet (WATCH-007 handles that)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Watcher agent loop uses tokio::select! to wait on both user input channel AND parent broadcast receiver
  #   2. Observation buffer accumulates StreamChunks from parent until a natural breakpoint is detected
  #   3. Natural breakpoints are: TurnComplete, ToolResult, or silence timeout (configurable, default 5 seconds)
  #   4. When a breakpoint is detected, accumulated observations are formatted into an evaluation prompt
  #   5. User prompts take priority over observations - direct user input is processed immediately
  #   6. Evaluation prompt includes: accumulated observations, watcher role context, and instruction to evaluate/respond
  #   7. Watcher can be in one of three states: Idle (waiting), Observing (accumulating), Processing (running agent)
  #
  # EXAMPLES:
  #   1. Parent sends TextDelta chunks → watcher accumulates in buffer → TurnComplete received → watcher formats evaluation prompt with accumulated text → runs agent stream
  #   2. User sends prompt to watcher while observations are buffered → user prompt processed immediately → accumulated observations processed after
  #   3. Parent sends ToolUse, then ToolResult → watcher waits for ToolResult breakpoint → formats prompt with tool execution context
  #   4. Parent goes silent for 5 seconds (configurable) → silence timeout breakpoint triggers → watcher processes accumulated observations
  #   5. Watcher receives broadcast RecvError::Lagged(n) → logs warning about missed n chunks → continues observing from current position
  #   6. Empty observation buffer when breakpoint occurs → no evaluation prompt generated → continue waiting
  #
  # ========================================

  Background: User Story
    As a watcher session
    I want to receive input from both user prompts and parent session observations
    So that I can provide contextual assistance based on what I observe in the parent session while still responding to direct user queries

  @unit
  Scenario: Accumulate observations until TurnComplete breakpoint
    Given a watcher session is observing a parent session
    And the watcher has an empty observation buffer
    When the parent sends TextDelta chunks "Hello" and "World"
    And the parent sends TurnComplete
    Then the watcher should have accumulated "HelloWorld" in the buffer
    And the watcher should detect a natural breakpoint
    And the watcher should format an evaluation prompt with the accumulated text
    And the observation buffer should be cleared after processing

  @unit
  Scenario: User prompt takes priority over buffered observations
    Given a watcher session is observing a parent session
    And the watcher has accumulated observations in the buffer
    When the user sends a prompt "What do you think?"
    Then the user prompt should be processed immediately
    And the accumulated observations should remain in the buffer for later processing

  @unit
  Scenario: ToolResult triggers natural breakpoint
    Given a watcher session is observing a parent session
    And the watcher has an empty observation buffer
    When the parent sends ToolUse for tool "bash"
    And the parent sends ToolResult with output "command output"
    Then the watcher should detect a natural breakpoint at ToolResult
    And the watcher should format an evaluation prompt with tool execution context

  @unit
  Scenario: Silence timeout triggers breakpoint
    Given a watcher session is observing a parent session
    And the silence timeout is configured to 5 seconds
    And the watcher has accumulated observations in the buffer
    When no chunks are received for 5 seconds
    Then the watcher should detect a silence timeout breakpoint
    And the watcher should process the accumulated observations

  @unit
  Scenario: Handle broadcast lag gracefully
    Given a watcher session is observing a parent session
    When the watcher receives RecvError::Lagged with 10 missed chunks
    Then the watcher should log a warning about 10 missed chunks
    And the watcher should continue observing from the current position

  @unit
  Scenario: Empty buffer at breakpoint does not trigger evaluation
    Given a watcher session is observing a parent session
    And the watcher has an empty observation buffer
    When the parent sends TurnComplete
    Then no evaluation prompt should be generated
    And the watcher should continue waiting for observations
