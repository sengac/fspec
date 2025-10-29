@tui
@git
@performance
@bug
@ui-refinement
@GIT-007
Feature: Diff loading system has rendering bugs and performance issues
  """
  Root cause: Object reference instability in React dependencies. allFiles array recreated every render causing selectedFile to have new reference every time, triggering useEffect infinitely. Fix: 1) Use useMemo for allFiles 2) Depend on selectedFileIndex primitive 3) Use AbortController for cancellation 4) Consider worker threads for git operations to prevent main thread blocking.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. useEffect dependency on selectedFile object causes infinite re-renders because allFiles array is recreated on every render with new object instances
  #   2. Race conditions: multiple getFileDiff calls can run simultaneously when user navigates quickly, last one to complete wins but might show wrong file's diff
  #   3. Git operations (isomorphic-git file I/O) run on main thread blocking React rendering and causing UX hangs
  #   4. No debouncing: every arrow key press triggers immediate diff load, should debounce rapid file navigation
  #   5. FIX: useEffect should depend on selectedFileIndex and derive file path, not selectedFile object
  #   6. FIX: Implement AbortController or cancellation token to cancel previous diff loads when selection changes
  #   7. FIX: Consider worker threads (worker_threads module) to offload git diff operations from main thread
  #
  # EXAMPLES:
  #   1. User navigates to file1.ts, sees diff loading, presses down arrow to file2.ts immediately - both diffs load simultaneously, file1.ts diff might display even though file2.ts is selected
  #   2. User opens ChangedFilesViewer with 10 files - diff pane flickers constantly showing 'Loading diff...' over and over because useEffect runs on every render
  #   3. SOLUTION: useEffect depends on [selectedFileIndex, stagedFiles, unstagedFiles, cwd], AbortController cancels previous loads, diff loads once per file selection without flicker
  #
  # ========================================
  Background: User Story
    As a developer using diff viewer
    I want to see accurate, non-flickering diffs without UX lag
    So that I can efficiently review code changes

  Scenario: Race condition causes wrong file diff to display
    Given ChangedFilesViewer is open with files file1.ts and file2.ts
    And file1.ts is currently selected
    When user presses down arrow to navigate to file2.ts immediately while file1.ts diff is loading
    Then both file1.ts and file2.ts diffs should load simultaneously
    But only file2.ts diff should be displayed (not file1.ts)
    And previous file1.ts diff load should be cancelled

  Scenario: Infinite re-render flickering due to object reference instability
    Given ChangedFilesViewer is open with 10 changed files
    When the component renders
    Then allFiles array should be memoized with useMemo
    And useEffect should NOT trigger on every render
    And diff pane should NOT flicker with "Loading diff..." repeatedly
    And diff should load exactly once per file selection

  Scenario: Fixed dependency management prevents re-render loop
    Given useEffect depends on selectedFileIndex, stagedFiles, unstagedFiles, and cwd
    And allFiles array is memoized with useMemo
    When user navigates between files
    Then diff should load only when selectedFileIndex actually changes
    And no infinite re-render loop should occur
    And AbortController should cancel previous loads

  Scenario: Worker thread offloads git operations from main thread
    Given git diff operations are computationally expensive
    When diff is loaded for a large file
    Then git operations should run in a worker thread
    And React rendering should NOT be blocked
    And UX should remain responsive during diff loading
