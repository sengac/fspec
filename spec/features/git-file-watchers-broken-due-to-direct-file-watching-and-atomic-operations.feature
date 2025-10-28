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
  - CRITICAL BUG: fs.watch on files directly breaks with atomic operations (git uses write-temp + rename pattern)
  - Root cause: fs.watch watches inodes, not paths. Atomic renames create new inodes, breaking watchers
  - Solution: Watch parent directories (.git/, .git/refs/) and filter by filename (like work-units.json pattern)
  - MUST remove event type filtering: listen to both 'change' and 'rename' events (atomic ops emit 'rename')
  - MUST add error event handlers for permission issues and watcher failures
  - Platform behavior: macOS (FSEvents) more reliable, Linux (inotify) breaks easily, Windows different event types
  - Working reference pattern: BoardView.tsx:81-102 (work-units.json directory watcher)
  - Broken code locations: BoardView.tsx:104-123 (stash), 125-144 (index), 146-164 (HEAD)
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

  Scenario: Watcher detects git stash via directory watching
    Given the TUI is running with git file watchers active
    And watchers are monitoring .git/refs/ directory (not .git/refs/stash file directly)
    When I run "git stash push" which triggers atomic rename operation
    Then the watcher should detect an event in .git/refs/ directory
    And the watcher should filter for filename='stash'
    And loadStashes() should be called
    And the stash panel should update with the new stash

  Scenario: Error handler prevents silent watcher failures
    Given the TUI is running with git file watchers active
    And watchers have error event handlers configured
    When a permission error occurs accessing .git/index
    Then the error event handler should catch the error
    And a warning should be logged to the console
    And the watcher should continue monitoring other git files
    And the TUI should not crash or hang

  Scenario: Directory watcher filters by filename
    Given the TUI is running with git file watchers active
    And watchers are monitoring .git/ and .git/refs/ directories
    When git writes to .git/index (filename='index')
    Then the .git/ directory watcher should trigger loadFileStatus()
    When git writes to .git/config (filename='config')
    Then the .git/ directory watcher should ignore the event
    When git writes to .git/refs/stash (filename='stash')
    Then the .git/refs/ directory watcher should trigger loadStashes()

  Scenario: Watchers handle both change and rename events
    Given the TUI is running on Linux with inotify-based fs.watch
    And watchers are listening to ALL event types (no filtering)
    When git performs atomic rename for .git/index
    Then the watcher should detect 'rename' event type
    And loadFileStatus() should be triggered
    When git updates .git/HEAD in-place
    Then the watcher should detect 'change' event type
    And loadFileStatus() and loadStashes() should be triggered
