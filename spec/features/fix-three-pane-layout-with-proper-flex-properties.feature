@diff-viewer
@interactive
@ui
@done
@layout
@tui
@critical
@TUI-004
Feature: Fix three-pane layout with proper flex properties

  """
  The CheckpointViewer currently renders FileDiffViewer (which contains file list + diff panes). This must be refactored to extract the diff pane from FileDiffViewer and render it separately alongside a new left column containing stacked checkpoint list and file list panes. The left column must use identical flex properties to FileDiffViewer's file list pane (minWidth=30, flexBasis='25%', flexShrink=1). Both panes within the left column use flexGrow=1 to split vertical space equally.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The CheckpointViewer must have exactly 3 panes: checkpoint list (top-left), file list (bottom-left), diff viewer (right)
  #   2. The two left panes (checkpoint list and file list) must use the same width properties as FileDiffViewer: minWidth, flexBasis, flexGrow, flexShrink - NO percentages
  #   3. The checkpoint list and file list must be stacked vertically on the left side, with both panes using flexGrow to split the vertical space
  #   4. The diff viewer pane on the right must use flexGrow to fill remaining horizontal space
  #   5. Both panes use flexGrow={1} to split the vertical space equally (50/50), matching the horizontal behavior of FileDiffViewer
  #   6. Use EXACTLY the same values as FileDiffViewer: minWidth={30}, flexBasis='25%', flexShrink={1} for the left column container
  #
  # EXAMPLES:
  #   1. When user opens CheckpointViewer, they see checkpoint list at top-left (30 char minWidth), file list at bottom-left (30 char minWidth), and diff viewer on right (grows to fill space)
  #   2. When terminal is narrow (80 chars), left panes respect 30 char minimum width, diff pane shrinks but remains visible
  #   3. When terminal is wide (200 chars), left panes maintain proportional width using flexBasis and flexGrow (no percentage values), diff pane expands to use remaining space
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the checkpoint list and file list panes split the left side 50/50 vertically, or should one be weighted larger?
  #   A: true
  #
  #   Q: What specific flexBasis and flexGrow values should the left panes use? Should they match FileDiffViewer's values exactly (minWidth=30, flexBasis='25%', flexShrink=1) or use different values without percentages?
  #   A: true
  #
  #   Q: The current FileDiffViewer uses flexBasis='25%' which IS a percentage. Should I remove percentages from FileDiffViewer too, or just ensure CheckpointViewer matches whatever FileDiffViewer uses?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Keep FileDiffViewer as-is with flexBasis='25%'. The work unit description saying 'NO PERCENTAGES' was incorrect - we should match FileDiffViewer exactly, which does use a percentage
  #
  # ========================================

  Background: User Story
    As a developer using the CheckpointViewer
    I want to view checkpoints, changed files, and diffs in a three-pane layout
    So that I can efficiently navigate through checkpoint data with proper flex-based responsive layout

  Scenario: Display three-pane layout with correct structure
    Given the CheckpointViewer component is rendered
    When I view the layout structure
    Then I should see exactly 3 panes
    And the checkpoint list should be at the top-left
    And the file list should be at the bottom-left
    And the diff viewer should be on the right
    And the left column should stack the checkpoint and file lists vertically

  Scenario: Left column uses same flex properties as FileDiffViewer
    Given the CheckpointViewer component is rendered
    When I inspect the left column flex properties
    Then minWidth should be 30
    And flexBasis should be "25%"
    And flexShrink should be 1
    And these values should match FileDiffViewer's file list pane exactly

  Scenario: Left panes split vertical space equally
    Given the CheckpointViewer component is rendered with checkpoints and files
    When I inspect the vertical split between checkpoint list and file list
    Then both panes should use flexGrow of 1
    And the vertical space should be divided 50/50

  Scenario: Diff viewer fills remaining horizontal space
    Given the CheckpointViewer component is rendered
    When I inspect the diff viewer pane
    Then it should use flexGrow of 1
    And it should fill all remaining horizontal space after the left column
