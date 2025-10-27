@done
@high
@tui
@real-time
@file-watching
@interactive-cli
@ITF-005
Feature: Real-time file and git status watching in TUI
  """
  Store structure: Add stashes, stagedFiles, unstagedFiles state arrays. Add loadStashes(), loadFileStatus() actions. BoardView subscribes to these via useFspecStore hooks. Watchers in BoardView call store actions on file change events. Store updates trigger React component re-renders automatically.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. TUI must watch .git/refs/stash for real-time git stash changes
  #   2. TUI must watch .git/index for real-time file status changes (staged/unstaged)
  #   3. Watchers must update Zustand store state to trigger React re-renders
  #   4. MUST reuse existing isomorphic-git functions: getStagedFiles, getUnstagedFiles from src/git/status.ts
  #   5. MUST reuse existing git.log({ ref: 'refs/stash' }) pattern for loading stashes (DRY principle)
  #   6. Watchers must be cleaned up on component unmount to prevent memory leaks
  #   7. Store must provide actions: loadStashes(), loadFileStatus() to be called by watchers
  #   8. TUI must watch .git/HEAD for branch changes and commits on current branch
  #
  # EXAMPLES:
  #   1. User opens TUI, creates git stash via terminal, TUI automatically updates stash panel without restart
  #   2. User stages a file with git add, TUI immediately shows file in staged section (green)
  #   3. User unstages a file with git reset, TUI moves file from staged to unstaged section (yellow)
  #   4. User creates checkpoint via fspec command, TUI stash panel updates showing new stash
  #   5. User modifies work-units.json AND stages a file, both panels update in real-time
  #   6. User exits TUI, watchers are cleaned up and don't leak memory
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should watchers debounce rapid changes (e.g., multiple git add commands in quick succession)?
  #   A: true
  #
  #   Q: Should we also watch HEAD ref for branch changes or commits?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to see real-time updates to git stashes and file status
    So that the board reflects current state without manual refresh

  Scenario: TUI auto-updates stash panel when git stash is created externally
    Given the TUI is running and showing the stash panel
    And there are no existing git stashes
    When I create a git stash via terminal command
    Then the TUI stash panel should automatically update
    And the new stash should be displayed in the stash list
    And I should not need to restart the TUI

  Scenario: TUI auto-updates when file is staged via git add
    Given the TUI is running and showing the files panel
    And I have an unstaged file "src/auth.ts"
    When I stage the file with "git add src/auth.ts"
    Then the TUI files panel should automatically update
    And "src/auth.ts" should appear in the staged section with green indicator
    And "src/auth.ts" should be removed from the unstaged section

  Scenario: TUI auto-updates when file is unstaged via git reset
    Given the TUI is running and showing the files panel
    And I have a staged file "src/utils.ts"
    When I unstage the file with "git reset src/utils.ts"
    Then the TUI files panel should automatically update
    And "src/utils.ts" should appear in the unstaged section with yellow indicator
    And "src/utils.ts" should be removed from the staged section

  Scenario: TUI auto-updates when checkpoint is created via fspec command
    Given the TUI is running and showing the stash panel
    And work unit "AUTH-001" exists
    When I create a checkpoint with "fspec checkpoint AUTH-001 baseline"
    Then the TUI stash panel should automatically update
    And the new checkpoint stash should be displayed in the stash list

  Scenario: TUI auto-updates multiple panels simultaneously
    Given the TUI is running showing both work units and files panels
    And I have an unstaged file "spec/work-units.json"
    When I modify "spec/work-units.json" to update a work unit status
    And I stage "spec/work-units.json" with "git add spec/work-units.json"
    Then the work units panel should automatically update with new status
    And the files panel should automatically update showing "spec/work-units.json" as staged
    And both updates should happen in real-time without manual refresh

  Scenario: Watchers are cleaned up on component unmount
    Given the TUI is running with active file watchers
    And watchers are monitoring ".git/refs/stash", ".git/index", and ".git/HEAD"
    When I exit the TUI
    Then all fs.watch watchers should be closed
    And no watcher instances should remain in memory
    And no memory leaks should be detected
