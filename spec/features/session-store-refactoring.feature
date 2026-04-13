@TUI-086
Feature: Refactor sessionStore — split file, extract DRY helper, fix state desync

  """
  Split sessionStore.ts into: sessionStore.ts (store+types+actions with immer), sessionSelectors.ts (named selector hooks), sessionActions.ts (useSessionActions with useShallow). Re-export all from sessionStore.ts for backward compatibility. Extract clearAndResetSession helper to DRY the 4 duplicate patterns.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. sessionStore.ts must be under 300 lines after refactoring
  #   2. All 4 duplicate sessionClearActive+set patterns must use a shared helper
  #   3. setIsolationState must reset pendingIsolatedSession to prevent state desync
  #   4. All existing imports from sessionStore must continue working via re-exports
  #
  # EXAMPLES:
  #   1. sessionStore split into store+selectors+actions → 261+19+31 lines, all tests pass
  #   2. navigateToNewSession and navigateToNewSessionIsolated merged into single function with parameter
  #
  # ========================================

  Background: User Story
    As a developer
    I want to refactor sessionStore into focused modules
    So that maintain files under 300-line limit and eliminate DRY violations

  Scenario: sessionStore is split into focused modules under 300 lines
    Given sessionStore.ts contains state, actions, selectors, and action hooks
    And sessionStore.ts exceeds 300 lines
    When the store is refactored into sessionStore, sessionSelectors, and sessionActions
    Then sessionStore.ts should be under 300 lines
    And sessionSelectors.ts should contain all selector hooks
    And sessionActions.ts should contain the useSessionActions hook
    And all existing imports from sessionStore should continue to work via re-exports

  Scenario: Duplicate sessionClearActive patterns use shared helper
    Given prepareForNewSession, reset, navigateToNewSession, and navigateToNewSessionIsolated each duplicate sessionClearActive try-catch and overlapping set calls
    When a clearAndResetSession helper is extracted
    Then all 4 functions should delegate to the shared helper
    And navigateToNewSession and navigateToNewSessionIsolated should be merged into one function with an isolated parameter

  Scenario: setIsolationState resets pendingIsolatedSession
    Given pendingIsolatedSession is set to true by navigateToNewSessionIsolated
    When setIsolationState is called with new isolation state
    Then pendingIsolatedSession should be reset to false
    And isIsolated and worktreePath should be updated to the new values
