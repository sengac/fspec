@BUG-171
@tui
@tool-execution
@wip
Feature: Exec-stdin push chunks drive the TUI composer overlay

  """
  BUG-171 (TUI layer): handle_stream_chunk_state_updates branches on the two new state-only StreamChunk variants — ExecStdinRequest → Action::ExecStdinPromptFetched, ExecStdinRequestCleared → Action::ExecStdinDismissed. Reuses the existing reducer, precedence chain (HITL > exec-stdin > pause > composer), key handling and render path from TOOL-022 P2. The two existing pull probe sites (focus switch + Paused probe) are retained as belt-and-braces.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT (BUG-171)
  # ========================================
  #
  # BUSINESS RULES:
  #   3. The two existing pull probe sites (focus switch + Paused-state probe) are retained as belt-and-braces; the push chunk is the primary trigger
  #
  # ========================================

  Background: User Story
    As a user of the Rust TUI agent
    I want to have the exec-stdin composer overlay appear automatically when a running command goes quiet waiting for input
    So that I can type into the running command's stdin without switching sessions or waiting for a status change

  Scenario: Exec-stdin request chunk populates the composer overlay while the session stays Running
    Given the agent session is Running with no exec-stdin slot
    When an exec-stdin request StreamChunk arrives for that session
    Then the exec-stdin composer overlay is visible in the focused pane's input area
    And the slot precedence is respected (HITL > exec-stdin > pause > composer)
    And no Paused state change chunk was emitted

  Scenario: The cleared chunk clears the TUI slot without sending anything
    Given the exec-stdin composer overlay is visible for a session
    When an exec-stdin cleared StreamChunk arrives for that session
    Then the exec-stdin slot is cleared
    And nothing was written to any exec session stdin

  Scenario: State-only chunks never land in the transcript scrollback
    Given a session context that records chunks
    When an exec-stdin request chunk and a cleared chunk are recorded for the session
    Then no scrollback chunk is created for either variant

  Scenario: Existing pull probe sites still surface and clear the overlay
    Given a Running agent session with a live quiet exec session
    When the user switches focus away and back
    Then the overlay is re-probed on focus return
    And a Paused state change probe still reads the pending request

  @integration
  Scenario: End-to-end — interactive command surfaces the overlay without a focus switch or status flip
    Given the agent runs an interactive Bash command that reads stdin and the session stays focused and Running
    When the command goes quiet for at least the detector threshold
    Then the exec-stdin composer overlay appears in the focused pane without any session switch or status change
    And typing a value and pressing Enter sends the value plus newline to the command stdin
