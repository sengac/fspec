@done
@ui
@board-visualization
@tui
@TUI-008
Feature: Vertical layout for changed files viewer
  """
  This feature changes the layout direction of FileDiffViewer from horizontal (row) to vertical (column). FileDiffViewer is used by both ChangedFilesViewer and CheckpointViewer. For CheckpointViewer, the left column also changes from vertical to horizontal orientation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Files list must be positioned above the diff view in vertical layout
  #   2. Height proportions must remain 33% for files list and 67% for diff view
  #   3. Layout changes from horizontal (row) to vertical (column) flexbox direction
  #   4. Yes, vertical border between panes becomes horizontal border
  #   5. Both viewers get vertical layout. CheckpointViewer's left-side components also change to opposite direction (vertical)
  #
  # EXAMPLES:
  #   1. FileDiffViewer changes from horizontal (files left, diff right) to vertical (files top, diff bottom)
  #   2. CheckpointViewer left column changes from vertical stack (checkpoints top, files bottom) to horizontal row (checkpoints left, files right)
  #   3. FileDiffViewer: Files pane maintains 33% height, diff pane maintains 67% height
  #   4. CheckpointViewer: Checkpoints pane gets 33% width, files pane gets 67% width in top row
  #   5. Vertical border in FileDiffViewer becomes horizontal border separating top and bottom
  #   6. CheckpointViewer maintains top row at 33% height and bottom diff pane at 67% height
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the section headings ('Files' and 'Diff') remain in the same style with the same focus indication (green background)?
  #   A: true
  #
  #   Q: Should the keyboard navigation (Tab to switch panes, arrow keys to navigate) remain exactly the same?
  #   A: true
  #
  #   Q: Should the border styling change? Currently there's a vertical border between panes - should this become a horizontal border?
  #   A: true
  #
  #   Q: This change affects FileDiffViewer which is used by both ChangedFilesViewer and CheckpointViewer. Should BOTH viewers get the vertical layout, or just the ChangedFilesViewer?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Yes, section headings maintain same style with green background focus indication
  #   2. Yes, keyboard navigation (Tab, arrow keys) remains exactly the same
  #
  # ========================================
  Background: User Story
    As a developer using the changed files viewer
    I want to view files list above the diff instead of to the left
    So that the layout better fits narrower terminal windows while maintaining the same proportional space allocation

  Scenario: FileDiffViewer renders with vertical layout
    Given FileDiffViewer is rendered with files and diff content
    When the component layout is measured
    Then the container flexDirection should be "column"
    And the files pane should be positioned above the diff pane

  Scenario: FileDiffViewer maintains correct height proportions
    Given FileDiffViewer is rendered with 100px total height
    When the component calculates flexGrow ratios
    Then the files pane should have flexGrow of 1 (33% height)
    And the diff pane should have flexGrow of 2 (67% height)

  Scenario: FileDiffViewer uses horizontal borders between panes
    Given FileDiffViewer is rendered with vertical layout
    When borders are rendered between panes
    Then the files pane should have borderBottom set to true
    And the files pane should have borderRight set to false

  Scenario: CheckpointViewer renders with horizontal top row and vertical overall layout
    Given CheckpointViewer is rendered with checkpoints, files, and diff
    When the component layout is measured
    Then the overall container flexDirection should be "column"
    And the top row flexDirection should be "row" with checkpoints and files side-by-side

  Scenario: CheckpointViewer maintains correct width proportions in top row
    Given CheckpointViewer is rendered with 100px top row width
    When the top row calculates flexGrow ratios
    Then the checkpoints pane should have flexGrow of 1 (33% width)
    And the files pane should have flexGrow of 2 (67% width)

  Scenario: CheckpointViewer maintains correct height proportions overall
    Given CheckpointViewer is rendered with 100px total height
    When the component calculates overall flexGrow ratios
    Then the top row should have flexGrow of 1 (33% height)
    And the bottom diff pane should have flexGrow of 2 (67% height)
