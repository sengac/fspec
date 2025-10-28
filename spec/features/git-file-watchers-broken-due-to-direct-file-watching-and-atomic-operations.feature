@done
@high
@tui
@file-watching
@real-time
@bug-fix
@interactive-cli
@BOARD-018
Feature: Git file watchers broken due to direct file watching and atomic operations
  """
  Architecture notes:
  - CRITICAL BUG: fs.watch() has cross-platform reliability issues
  - Root cause 1: fs.watch watches inodes, not paths. Atomic renames create new inodes
  - Root cause 2: fs.watch filename parameter is unreliable on macOS (often null/undefined)
  - Solution: Use chokidar library for cross-platform file watching
  - Chokidar normalizes events across platforms (macOS FSEvents, Linux inotify, Windows)
  - Watch specific files: .git/refs/stash, .git/index, .git/HEAD
  - Chokidar config: ignoreInitial: true (prevent initial scan triggering events)
  - Chokidar handles atomic operations automatically across all platforms
  - Add error handlers for watcher failures
  - Broken code locations: BoardView.tsx:104-130 (refs watcher), 132-162 (git dir watcher)
  - Dependencies: Add chokidar to package.json
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Git file watchers MUST watch parent directories (not files directly) to handle atomic rename operations
  #   2. Watchers MUST listen to ALL event types (both 'change' and 'rename'), not filter for specific types
  #   3. File change handlers MUST be debounced to batch rapid git operations (100-200ms delay)
  #   4. Watchers MUST have error event handlers to prevent silent failures
  #   5. Implementation MUST follow the working pattern from work-units.json watcher (BoardView.tsx:81-102)
  #
  # EXAMPLES:
  #   1. User runs 'git stash push', watcher detects 'rename' event in .git/refs/ directory, triggers debounced loadStashes() after 100ms
  #   2. User runs 'git add .' (10 files), watcher detects multiple rapid events in .git/ directory, debouncing batches into single loadFileStatus() call
  #   3. Watcher encounters permission error on .git/index, error handler logs warning, continues monitoring other files
  #   4. Directory watcher filters events: only responds when filename='index' or 'stash' or 'HEAD', ignores other .git files
  #   5. User runs TUI on Linux, atomic rename triggers 'rename' event (not 'change'), watcher correctly handles both event types
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to see real-time git status updates that actually work
    So that the TUI reflects changes without missing events due to atomic file operations

  Scenario: Chokidar watcher detects git stash on macOS
    Given the TUI is running with chokidar file watchers active
    And chokidar is watching .git/refs/stash file
    And I am on macOS where fs.watch filename parameter is unreliable
    When I run "git stash push" which triggers atomic rename operation
    Then chokidar should detect the file change event
    And loadStashes() should be called
    And the stash panel should update with the new stash

  Scenario: Error handler prevents silent watcher failures
    Given the TUI is running with chokidar file watchers active
    And watchers have error event handlers configured
    When a permission error occurs or watcher fails
    Then the error event handler should catch the error
    And a warning should be logged to the console
    And the watcher should continue monitoring other git files
    And the TUI should not crash or hang

  Scenario: Chokidar watches only specific git files
    Given the TUI is running with chokidar file watchers active
    And chokidar is watching .git/index, .git/HEAD, and .git/refs/stash
    When git writes to .git/index
    Then chokidar should detect the change
    And loadFileStatus() should be called
    When git writes to .git/config
    Then chokidar should NOT detect any change
    And no reload functions should be called
    When git writes to .git/refs/stash
    Then chokidar should detect the change
    And loadStashes() should be called

  Scenario: Chokidar handles atomic operations cross-platform
    Given the TUI is running with chokidar file watchers active
    And chokidar is watching .git/index and .git/HEAD
    When git performs atomic rename for .git/index on Linux
    Then chokidar should detect the change (normalizes rename to change)
    And loadFileStatus() should be triggered
    When git updates .git/HEAD in-place on macOS
    Then chokidar should detect the change
    And loadFileStatus() and loadStashes() should be triggered
    And cross-platform event normalization should work correctly
