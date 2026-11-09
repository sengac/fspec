@done
@BUG-359
@tui
@tui-component
@agent-view
@bug
@regression
@source-shape
Feature: paint_spinner computes no discarded glyph
  """
  BUG-359: in rust/fspec-tui/src/views/agent/input_transition.rs, paint_spinner painted the spinner row through paint_spinner_line (from the frame index it computes) and then evaluated `let _ = current_frame_glyph(elapsed_ms)`, a #[must_use] call whose glyph was thrown away. The fix removes the dead call and its now-unused import. The spinner row is unchanged.
  Test file: rust/fspec-tui/tests/bug359_paint_spinner_no_discarded_glyph.rs.
  """

  Background: User Story
    As a developer maintaining the agent input transitions
    I want paint_spinner to compute only what it paints
    So that the spinner code has no dead calls that hide which glyph source is real

  Scenario: paint_spinner does not compute and discard a spinner glyph
    Given the rust/fspec-tui/src/views/agent/input_transition.rs source
    When a developer reads the body of paint_spinner
    Then it paints the spinner row through paint_spinner_line
    And it contains no discarded current_frame_glyph call
