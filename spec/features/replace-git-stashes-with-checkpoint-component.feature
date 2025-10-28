@board-visualization
@tui
@ITF-006
Feature: Replace Git Stashes with Checkpoint Component
  """
  Uses CheckpointPanel component to replace GitStashesPanel. Watches .git/fspec-checkpoints-index/ directory with chokidar for cross-platform file watching. Counts checkpoints by reading JSON index files and distinguishing auto (🤖) vs manual (📌) checkpoints based on naming pattern. Displays format: 'Manual: X, Auto: Y' or 'Checkpoints: None' when both are zero. Integrates with BoardView and UnifiedBoardLayout by passing cwd prop.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Git Stashes component must be completely removed from TUI
  #   2. Checkpoint component must replace Git Stashes in the same position
  #   3. Checkpoint count must be tracked dynamically using fs.watch
  #   4. Display format must be 'Checkpoints: None' (0), 'Checkpoints: 1' (1), 'Checkpoints: N' (N>1)
  #   5. Keybinding 'S View Stashes' must change to 'C View Checkpoints'
  #   6. Keybinding 'C View Changed Files' must change to 'F View Changed Files'
  #   7. Checkpoints stored in two places: (1) Git refs at refs/fspec-checkpoints/{workUnitId}/{checkpointName}, (2) Index files at .git/fspec-checkpoints-index/{workUnitId}.json. For TUI watching, use chokidar to watch .git/fspec-checkpoints-index/ directory (cross-platform compatible, handles atomic operations like fs.rename). Example: chokidar.watch(path.join(cwd, '.git', 'fspec-checkpoints-index'), { ignoreInitial: true, persistent: false })
  #   8. Checkpoints are git commit objects (in .git/objects/) referenced by git refs (refs/fspec-checkpoints/{workUnitId}/{checkpointName}). Index files are JSON at .git/fspec-checkpoints-index/{workUnitId}.json with format: {checkpoints: [{name, message}]}. To count: read all *.json files in index directory, parse each, sum checkpoint array lengths.
  #   9. Count separately with labels. Display format: 'Manual: 2, Auto: 3'. When both are zero: 'Checkpoints: None'. When only one type exists, show both anyway for clarity (e.g., 'Manual: 0, Auto: 3' or 'Manual: 2, Auto: 0').
  #   10. Real-time updates, no debouncing. Update checkpoint count immediately when chokidar detects changes in .git/fspec-checkpoints-index/ directory.
  #   11. Show both auto (🤖) and manual (📌) checkpoints together in the list view when pressing 'C' key. Display them with their respective icons to distinguish between types.
  #
  # EXAMPLES:
  #   1. TUI starts with no checkpoints, displays 'Checkpoints: None'
  #   2. User creates one checkpoint via CLI, TUI automatically updates to 'Checkpoints: 1'
  #   3. User creates second checkpoint, TUI updates to 'Checkpoints: 2'
  #   4. User presses 'C' key, checkpoint list view opens (not stashes)
  #   5. User presses 'F' key, changed files view opens
  #   6. Checkpoint directory is watched with fs.watch, changes trigger count update automatically
  #
  # QUESTIONS (ANSWERED):
  #   Q: Where exactly are checkpoints stored? Is it .fspec/checkpoints/<work-unit-id>/ directory?
  #   A: true
  #
  #   Q: What file format are checkpoints? Individual JSON files, directories, or something else?
  #   A: true
  #
  #   Q: Should we count auto checkpoints (🤖) and manual checkpoints (📌) separately or together in the count?
  #   A: true
  #
  #   Q: Should checkpoint count update in real-time or with a debounce to avoid excessive updates?
  #   A: true
  #
  #   Q: When pressing 'C' to view checkpoints, should we show both auto and manual checkpoints in the list?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to see checkpoint count dynamically in the status bar
    So that I know how many checkpoints are available for rollback

  Scenario: TUI starts with no checkpoints
    Given the TUI is running
    And there are no checkpoints for the current work unit
    When the checkpoint panel renders
    Then it should display "Checkpoints: None"

  Scenario: User creates one manual checkpoint via CLI
    Given the TUI is running with checkpoint watching enabled
    And there are no checkpoints initially
    When a user creates a manual checkpoint via CLI
    And the checkpoint index file is updated
    Then the TUI should automatically update to display "Checkpoints: 1 Manual, 0 Auto"

  Scenario: User creates second checkpoint
    Given the TUI is displaying "Checkpoints: 1 Manual, 0 Auto"
    When a user creates another manual checkpoint via CLI
    And the checkpoint index file is updated
    Then the TUI should automatically update to display "Checkpoints: 2 Manual, 0 Auto"

  Scenario: User presses C key to view checkpoints
    Given the TUI is displaying checkpoint counts
    When the user presses the 'C' key
    Then a checkpoint list view should open
    And it should display both manual (📌) and automatic (🤖) checkpoints
    And it should NOT show a stashes view

  Scenario: User presses F key to view changed files
    Given the TUI is displaying the board view
    When the user presses the 'F' key
    Then a changed files view should open

  Scenario: Checkpoint directory changes trigger real-time count update
    Given the TUI is running with chokidar watching .git/fspec-checkpoints-index/
    And the TUI displays current checkpoint counts
    When a checkpoint is created or deleted externally
    And chokidar detects the change in the index directory
    Then the checkpoint count should update immediately without debouncing
