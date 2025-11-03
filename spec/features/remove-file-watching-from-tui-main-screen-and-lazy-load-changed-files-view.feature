@board-view
@tui
@ui
@performance
@refactoring
@file-watching
@TUI-014
Feature: Remove file watching from TUI main screen and lazy-load changed files view
  """
  Removes .git/index and .git/HEAD file watchers (BoardView.tsx lines 143-188). Preserves checkpoint stash watcher (lines 112-141). ChangedFilesViewer lazy-loads git status on mount using getStagedFiles/getUnstagedFiles from src/git/status.ts. Uses isomorphic-git (no git CLI subprocess). Changed files section removed from UnifiedBoardLayout header (lines 532-545). KEEP GIT-004 scenarios unchanged (F key behavior preserved).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The main TUI board displays: (1) Counter: 'Changed Files (X staged, Y unstaged)' (2) File preview showing up to 3 files with status indicators (+ for staged, M for unstaged) (3) Empty state: 'No changes' when no files changed. This appears in the header panel at lines 532-545 of UnifiedBoardLayout.tsx.
  #   2. Yes, when user presses 'F View Changed Files', it should calculate changed files at that moment using the exact same mechanism: Call Promise.all([getStagedFiles(cwd), getUnstagedFiles(cwd)]) from src/git/status.ts. This uses isomorphic-git to read .git/index directly (no git CLI subprocess). The same functions, same parameters, just triggered on-demand instead of by file watchers.
  #   3. There are file watchers currently running. Remove ONLY the .git/index and .git/HEAD watcher (BoardView.tsx lines 143-188) that watches for changed files. KEEP the git stash watcher (BoardView.tsx lines 112-141) which displays checkpoints in the header, and KEEP the checkpoint panel watcher (CheckpointPanel.tsx lines 64-92) which shows checkpoint counts.
  #   4. MODIFY 5 source files: (1) BoardView.tsx - remove .git/index/.git/HEAD watcher lines 143-188, remove loadFileStatus calls (2) UnifiedBoardLayout.tsx - remove changed files section lines 532-545 (3) ChangedFilesPanel.tsx - remove file counts, keep only keyboard shortcuts (4) ChangedFilesViewer.tsx - add lazy loading on mount via loadFileStatus (5) fspecStore.ts - KEEP unchanged. MODIFY 3 test files: BoardView-file-watchers.test.tsx and BoardView-git-watcher-fix.test.tsx (remove .git/index/.git/HEAD scenarios, keep checkpoint tests), ChangedFilesViewer.test.tsx (switch to store-based testing). MODIFY 1 feature file: ITF-005 (remove 3 scenarios about file watching). KEEP unchanged: GIT-004 feature file (F key behavior unchanged). Total: 12 files modified, 0 deleted, ~270 net lines removed.
  #
  # EXAMPLES:
  #   1. User opens TUI board → Header shows 'Git Stashes (2)' with checkpoint names → NO 'Changed Files' counter visible
  #   2. User presses F key → ChangedFilesViewer opens → Component calls loadFileStatus() on mount → Displays '2 staged, 1 unstaged' with file list and diffs
  #   3. User stages file externally (git add) → Main board header does NOT update automatically → User presses F key → Viewer loads fresh git status showing newly staged file
  #   4. User creates checkpoint via 'fspec checkpoint' → Git stashes section updates automatically (watcher still active) → Changed files section removed entirely from header
  #   5. BoardView-file-watchers.test.tsx: Remove test 'TUI auto-updates when file is staged via git add' → Keep test 'TUI auto-updates stash panel when git stash is created externally'
  #
  # QUESTIONS (ANSWERED):
  #   Q: What specific UI elements on the main TUI board display changed files information currently (counter, list, etc.)?
  #   A: true
  #
  #   Q: When user presses 'F View Changed Files', should it calculate changed files using git status at that moment (lazy loading)?
  #   A: true
  #
  #   Q: Are there file watchers currently running in the background, and should ALL file watching be removed from the TUI?
  #   A: true
  #
  #   Q: What code files, test files, and feature files need to be removed or refactored as part of this cleanup?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer viewing the TUI board
    I want to remove file watching overhead and see changed files only when needed
    So that the main board loads faster and file system watchers don't consume resources unnecessarily

  Scenario: Main board header shows checkpoints but not changed files counter
    Given the TUI is running and displaying the main board
    And there are 2 checkpoints in the git stash
    When I view the header panel
    Then I should see "Checkpoints" header
    And I should NOT see any "Changed Files" counter or file preview
    And the changed files section should be completely removed from the header

  Scenario: F key opens changed files viewer with lazy-loaded git status
    Given the TUI is running on the main board
    And there are 2 staged files and 1 unstaged file in the working directory
    When I press the "F" key
    Then the ChangedFilesViewer should open in full-screen mode
    And the component should use lazy loading (reads from store, not props)
    And the viewer should show file names with status indicators (+ for staged, M for unstaged)
    And the diff pane should render git diffs for selected files

  Scenario: Main board does not auto-update when files are staged externally
    Given the TUI is running and displaying the main board
    And the header shows "Checkpoints"
    When a file is staged externally using "git add"
    Then the main board header should NOT update automatically
    And no changed files counter should appear
    When I press the "F" key to open the changed files viewer
    Then the viewer should load fresh git status using getStagedFiles/getUnstagedFiles
    And the viewer should show the newly staged file in the file list

  Scenario: Checkpoint stash watcher remains active and updates header
    Given the TUI is running and displaying the main board
    And the header shows "Checkpoints"
    When a checkpoint is created using "fspec checkpoint WORK-001 baseline"
    Then the git stash watcher should detect the new checkpoint
    And the header should update automatically to show updated checkpoint count
    And the changed files section should remain absent from the header

  Scenario: Remove changed files watching tests but keep checkpoint tests
    Given the test file "BoardView-file-watchers.test.tsx" exists
    And it contains test "TUI auto-updates when file is staged via git add"
    And it contains test "TUI auto-updates stash panel when git stash is created externally"
    When the file watching removal refactoring is complete
    Then the test "TUI auto-updates when file is staged via git add" should be removed
    And the test "TUI auto-updates stash panel when git stash is created externally" should be kept
    And the test file should only test checkpoint stash watching (not .git/index/.git/HEAD watching)
