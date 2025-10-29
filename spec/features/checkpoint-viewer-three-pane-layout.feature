@done
@diff-viewer
@ui
@interactive
@high
@tui
@TUI-002
Feature: Checkpoint Viewer Three-Pane Layout
  """
  Reuse existing diff utilities: src/git/diff-worker.ts (worker thread), src/git/diff-parser.ts (parsing), src/git/diff.ts (git commands)
  Create shared FileDiffViewer component (file list pane + diff pane) to be used by both ChangedFilesViewer and CheckpointViewer - eliminates code duplication
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Checkpoint viewer must have three panes: checkpoint list (top-left), file list (bottom-left), and diff view (right)
  #   2. Checkpoints must be organized by time with most recent first
  #   3. Left side has two vertically stacked panes (checkpoints above, files below)
  #   4. Selecting a checkpoint updates the file list to show that checkpoint's files
  #   5. Selecting a file shows its diff in the right pane
  #   6. Yes - show timestamp, work unit ID, and number of files in checkpoint list
  #   7. Tab key cycles through all three panes (checkpoints -> files -> diff -> checkpoints)
  #   8. Load actual git diffs using worker threads (same as ChangedFilesViewer) - no mocks!
  #   9. Display appropriate message when checkpoint has no files
  #   10. Yes - reuse VirtualList with scroll acceleration and mouse wheel support (DRY principle)
  #   11. Use flexbox layout matching ChangedFilesViewer - NO PERCENTAGES! Use flexBasis, flexGrow, flexShrink, minWidth
  #   12. Must reuse shared diff-loading utilities from ChangedFilesViewer (DRY principle)
  #   13. Must reuse VirtualList component for all three panes (checkpoints, files, diff)
  #   14. Checkpoint list must display: name, timestamp, work unit ID, and file count
  #   15. ESC key exits checkpoint viewer and returns to previous view
  #   16. CRITICAL: Extract file list and diff viewer into a shared reusable component (FileDiffViewer) to avoid duplication between ChangedFilesViewer and CheckpointViewer
  #   17. ChangedFilesViewer must be refactored to use the new shared FileDiffViewer component
  #
  # EXAMPLES:
  #   1. User opens checkpoint viewer with 5 checkpoints, sees most recent checkpoint selected by default with timestamp and file count
  #   2. User navigates down in checkpoint list using arrow keys, file list updates to show files for newly selected checkpoint
  #   3. User presses Tab to switch focus from checkpoint list to file list, border color changes to indicate active pane
  #   4. User selects a file in file list, diff pane loads and displays git diff using worker thread (same as ChangedFilesViewer)
  #   5. User presses Tab again to focus diff pane, can scroll through diff with arrow keys and PgUp/PgDn
  #   6. User encounters checkpoint with no files, sees 'No files in this checkpoint' message in file list pane
  #   7. User scrolls rapidly in checkpoint list using mouse wheel, scroll acceleration kicks in for smooth navigation
  #   8. Refactor ChangedFilesViewer to use new shared FileDiffViewer component (file list + diff pane logic extracted)
  #   9. CheckpointViewer uses shared FileDiffViewer component, adds checkpoint list pane on top-left
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the checkpoint list show any metadata besides the checkpoint name (e.g., timestamp, work unit ID, number of files)?
  #   A: true
  #
  #   Q: How should keyboard navigation work between the three panes (Tab to cycle through all three, or something else)?
  #   A: true
  #
  #   Q: Should we load actual git diffs like ChangedFilesViewer does (using worker threads), or is mock data acceptable initially?
  #   A: true
  #
  #   Q: What should happen when a checkpoint has no files (empty state)?
  #   A: true
  #
  #   Q: Should we support the same scroll acceleration and mouse wheel support that VirtualList has?
  #   A: true
  #
  #   Q: Should the pane sizes be fixed or adjustable (like 30% top-left, 30% bottom-left, 40% right)?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to view and navigate checkpoints with their files and diffs
    So that I can review checkpoint history and restore previous states efficiently

  Scenario: Extract FileDiffViewer shared component
    Given ChangedFilesViewer has dual-pane file list and diff functionality
    When I extract the file list and diff pane logic into FileDiffViewer component
    Then FileDiffViewer should accept files list and render dual-pane layout
    And FileDiffViewer should use VirtualList for both file list and diff panes
    And FileDiffViewer should use worker threads to load git diffs
    And FileDiffViewer should use flexbox layout with flexBasis, flexGrow, flexShrink, minWidth

  Scenario: Refactor ChangedFilesViewer to use FileDiffViewer
    Given FileDiffViewer shared component exists
    When I refactor ChangedFilesViewer to use FileDiffViewer
    Then ChangedFilesViewer should render FileDiffViewer component with staged and unstaged files
    And ChangedFilesViewer should maintain existing keyboard navigation behavior
    And ChangedFilesViewer should maintain existing diff loading with worker threads
    And ChangedFilesViewer should have no code duplication with CheckpointViewer

  Scenario: Display checkpoint list with metadata
    Given I have 5 checkpoints for work unit TUI-001
    When I open the checkpoint viewer
    Then the checkpoint list pane should be displayed in top-left position
    And checkpoints should be sorted by timestamp with most recent first
    And each checkpoint should display name, timestamp, work unit ID, and file count
    And the most recent checkpoint should be selected by default

  Scenario: Navigate checkpoint list with arrow keys
    Given I am viewing checkpoints in the checkpoint viewer
    And the checkpoint list pane is focused
    When I press the down arrow key
    Then the next checkpoint should be selected
    And the file list pane should update to show files for the newly selected checkpoint
    And the checkpoint list border should remain cyan to indicate focus

  Scenario: Switch focus between three panes with Tab key
    Given I am viewing checkpoints with checkpoint list focused
    When I press Tab
    Then focus should move to the file list pane
    And the file list border should change to cyan
    And the checkpoint list border should change to gray
    When I press Tab again
    Then focus should move to the diff pane
    And the diff border should change to cyan
    And the file list border should change to gray
    When I press Tab again
    Then focus should cycle back to the checkpoint list pane

  Scenario: Load and display git diff using worker threads
    Given I have selected a checkpoint with files
    And the file list pane is focused
    When I press down arrow to select a file
    Then the diff pane should display "Loading diff..."
    And a worker thread should be spawned to load the git diff
    And the diff should be loaded using existing diff utilities from src/git/diff-worker.ts
    And the diff pane should display the git diff with syntax highlighting
    And added lines should be displayed with green background
    And removed lines should be displayed with red background
    And hunk headers should be displayed with cyan text

  Scenario: Scroll through diff with keyboard navigation
    Given I am viewing a checkpoint file diff
    And the diff pane is focused
    When I press down arrow key
    Then the diff should scroll down one line
    When I press PgDn
    Then the diff should scroll down one page
    And VirtualList scroll acceleration should apply for rapid scrolling

  Scenario: Handle checkpoint with no files
    Given I have a checkpoint with zero files
    When I select that checkpoint
    Then the file list pane should display "No files in this checkpoint"
    And the diff pane should display "No file selected"
    And the viewer should not attempt to load any diffs

  Scenario: Support mouse wheel scrolling in checkpoint list
    Given I am viewing checkpoints
    And the checkpoint list pane is focused
    When I scroll the mouse wheel rapidly
    Then VirtualList scroll acceleration should apply
    And the checkpoint list should scroll smoothly
    And the mouse wheel support should be identical to ChangedFilesViewer behavior

  Scenario: Exit checkpoint viewer with ESC key
    Given I am viewing checkpoints
    When I press ESC
    Then the checkpoint viewer should exit
    And I should return to the previous view
