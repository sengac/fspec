@done
@rust
@agent-view
@tui-component
@RPC-382
Feature: Port AgentView turn content modal (Enter on selected turn) to Rust

  """
  Modal state lives on AgentView as turn_modal_seq: Option<u64> (Some(seq) => open for that turn, keyed by stable chunk seq like RPC-381 selection). Full content sourced from the selected chunk's ChunkSource.text (re-wrapped to modal width), falling back to joining RenderedChunk.lines when source is None. New TurnContentModal widget in views/agent/turn_modal.rs follows confirm_dialog.rs/merge_confirm_dialog.rs overlay conventions; title colored by the turn's ChunkKind/role color; painted on top in views/agent.rs render after scrollback.
  New actions OpenTurnModal(u64) and CloseTurnModal in components/mod.rs, reduced on the App task to set/clear AgentView.turn_modal_seq. Enter routing: in views/agent/dispatch_select.rs, replace the Enter-suppression (RPC-381) with emit OpenTurnModal(selected_seq). Esc cascade in dispatch_select.rs/dispatch.rs: when turn_modal_seq.is_some() emit CloseTurnModal and consume (do NOT exit select mode); else if turn_select_mode exit mode; else AgentEscPressed. Tab tear-down: ToggleTurnSelectMode disable path also clears turn_modal_seq.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. While in turn-selection mode, pressing Enter on the selected turn opens the turn content modal for that turn
  #   2. The turn content modal displays the full (un-truncated) content of the selected turn
  #   3. While the turn content modal is open, scrollback and turn-navigation keys are gated (do not move the underlying selection)
  #   4. Pressing Esc while the modal is open closes the modal but stays in turn-selection mode; a second Esc then exits turn-selection mode
  #   5. Toggling turn-selection mode off with Tab while the modal is open also closes the modal
  #
  # EXAMPLES:
  #   1. In select mode with the second turn selected, user presses Enter; a modal opens showing the full text of the second turn
  #   2. A turn that is collapsed in the scrollback is selected and Enter is pressed; the modal shows the complete content, not the collapsed view
  #   3. With the modal open, user presses Esc; the modal closes and the [SELECT] badge is still shown (still in select mode)
  #   4. With the modal closed but still in select mode, user presses Esc again; select mode exits and the [SELECT] badge disappears
  #   5. With the modal open, user presses Tab; the modal closes and select mode is turned off
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to press Enter on a selected turn to open a modal showing the turn's full content, and press Esc to close it
    So that I can read a long past turn in full without it being collapsed in the scrollback (matching the TypeScript reference TUI)

  Scenario: Enter on the selected turn opens the turn content modal
    Given an AgentView in turn-selection mode with the second of three turns selected
    When I press the Enter key
    Then the turn content modal is open
    And the modal shows the full text of the second turn

  Scenario: The modal shows the full content of a collapsed turn
    Given an AgentView in turn-selection mode with a collapsed turn selected
    When I press the Enter key
    Then the turn content modal is open
    And the modal shows the complete content of the turn
    And the modal does not show the collapsed placeholder

  Scenario: Esc closes the modal but stays in turn-selection mode
    Given an AgentView in turn-selection mode with the turn content modal open
    When I press the Esc key
    Then the turn content modal is closed
    And turn-selection mode is still active
    And the session header still shows the [SELECT] badge

  Scenario: A second Esc after closing the modal exits turn-selection mode
    Given an AgentView in turn-selection mode with the turn content modal closed
    When I press the Esc key
    Then turn-selection mode becomes inactive
    And the session header does not show the [SELECT] badge

  Scenario: Tab while the modal is open closes the modal and exits select mode
    Given an AgentView in turn-selection mode with the turn content modal open
    When I press the Tab key
    Then the turn content modal is closed
    And turn-selection mode becomes inactive

  Scenario: Scrollback keys are gated while the modal is open
    Given an AgentView in turn-selection mode with the second of three turns selected
    And the turn content modal is open
    When I press the Up arrow key
    Then the selected turn is still the second turn
