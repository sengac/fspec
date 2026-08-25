@GIT-033
Feature: File Search Popup Uses Worktree Path for Isolated Sessions
  """
  The @file search popup must search the correct directory based on session state:
  - Isolated sessions: search the worktree (.fspec/worktrees/<session-id>/)
  - Non-isolated sessions: search the project root
  - No session yet: fallback to project root

  Implementation: useFileSearchInput hook calls sessionGetEffectiveCwd(sessionId) NAPI
  which already exists and handles all the logic. No new NAPI functions needed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When session is isolated, file search MUST use the worktree path, not the project root
  #   2. When session is non-isolated, file search MUST use the project root (sessionGetEffectiveCwd returns project root)
  #   3. When session ID is not yet available (before first message), file search uses project root as fallback
  #   4. File search must call sessionGetEffectiveCwd NAPI function (already exists) to get the correct search path
  #
  # EXAMPLES:
  #   1. ISOLATED - NEW FILE: User types @newfile → popup searches worktree and finds newfile.ts that AI just created
  #   2. ISOLATED - DELETED FILE: AI deletes legacy.js from worktree → user types @legacy → popup does NOT show it
  #   3. NON-ISOLATED: User types @config → sessionGetEffectiveCwd returns project root → popup searches project root
  #   4. NO SESSION YET: User types @README before first message → sessionId is null → fallback to project root
  #   5. FALLBACK: sessionGetEffectiveCwd returns null (edge case) → fallback to project root
  #   6. WORKTREE STATE: AI modifies package.json in worktree → popup finds it in worktree (reflects worktree state)
  #
  # ========================================
  Background: User Story
    As a user typing @ to search for files
    I want the search to use the same directory where the AI operates
    So that I can find files the AI created and reference files the AI can actually see

  # ========================================
  # ISOLATED SESSION SCENARIOS
  # ========================================
  @isolated
  @file-search
  Scenario: File search in isolated session finds files AI created in worktree
    Given I have an active isolated session "abc-123"
    And sessionGetEffectiveCwd("abc-123") returns ".fspec/worktrees/abc-123/"
    And the AI has created "src/newfile.ts" in the worktree
    When I type "@newfile" in the input field
    Then the file search popup should show "src/newfile.ts"
    And the glob search should have used path ".fspec/worktrees/abc-123/"

  @isolated
  @file-search
  Scenario: File search in isolated session does NOT show files AI deleted
    Given I have an active isolated session "abc-123"
    And sessionGetEffectiveCwd("abc-123") returns ".fspec/worktrees/abc-123/"
    And "src/legacy.js" exists in the main project but was deleted from the worktree
    When I type "@legacy" in the input field
    Then the file search popup should NOT show "src/legacy.js"

  @isolated
  @file-search
  Scenario: File search reflects worktree filesystem state
    Given I have an active isolated session "abc-123"
    And sessionGetEffectiveCwd("abc-123") returns ".fspec/worktrees/abc-123/"
    And the AI has modified "package.json" in the worktree
    When I type "@package" in the input field
    Then the file search popup should show "package.json"
    And the search results should reflect the worktree's current state

  @non-isolated
  @file-search
  Scenario: File search in non-isolated session searches project root
  # ========================================
  # NON-ISOLATED SESSION SCENARIOS
  # ========================================
    Given I have an active non-isolated session "def-456"
    And sessionGetEffectiveCwd("def-456") returns the project root path
    And "src/config.ts" exists in the project root
    When I type "@config" in the input field
    Then the file search popup should show "src/config.ts"
    And the glob search should have used the project root path

  @no-session
  @file-search
  Scenario: File search before session creation uses project root fallback
  # ========================================
  # FALLBACK SCENARIOS
  # ========================================
    Given no session has been created yet
    And sessionId is null
    And "README.md" exists in the project root
    When I type "@README" in the input field
    Then the file search popup should show "README.md"
    And the glob search should have used the project root path

  @fallback
  @file-search
  Scenario: File search falls back to project root when sessionGetEffectiveCwd returns null
    Given I have an active session "ghi-789"
    But sessionGetEffectiveCwd("ghi-789") returns null
    And "src/app.ts" exists in the project root
    When I type "@app" in the input field
    Then the file search popup should show "src/app.ts"
    And the glob search should have used the project root path
