@GIT-033
Feature: File Search Popup Uses Worktree Path for Isolated Sessions
  """
  useFileSearchInput hook needs sessionId prop, calls getSessionEffectiveCwd NAPI, passes result to callGlobTool
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When session is isolated, file search MUST use the worktree path, not the main project path
  #   2. When session is non-isolated, file search MUST use the project root (current behavior)
  #   3. When session ID is not yet available (before first message), file search uses project root as fallback
  #   4. File search must call getSessionEffectiveCwd NAPI function to get the correct search path
  #
  # EXAMPLES:
  #   1. User types @auth in isolated session, popup searches .fspec/worktrees/<session-id>/ and finds auth.ts that AI created
  #   2. User types @config in non-isolated session, popup searches project root (same as current behavior)
  #   3. User types @ before sending first message (no session yet), popup searches project root as fallback
  #   4. AI deletes file.txt in worktree, user types @file, popup does NOT show file.txt (correctly reflects worktree state)
  #
  # ========================================
  Background: User Story
    As a user in an isolated session
    I want to search for files using the @ popup
    So that find files in the worktree where the AI is actually working

  @isolated
  @file-search
  Scenario: File search in isolated session searches the worktree
    Given I am in an isolated session with worktree at ".fspec/worktrees/abc-123/"
    And the AI has created a file "src/auth.ts" in the worktree
    When I type "@auth" in the input field
    Then the file search popup should appear
    And the popup should show "src/auth.ts" in the results
    And the search should have used path ".fspec/worktrees/abc-123/"

  @non-isolated
  @file-search
  Scenario: File search in non-isolated session searches project root
    Given I am in a non-isolated session
    And a file "src/config.ts" exists in the project root
    When I type "@config" in the input field
    Then the file search popup should appear
    And the popup should show "src/config.ts" in the results
    And the search should have used the project root path

  @no-session
  @file-search
  Scenario: File search before session creation uses project root fallback
    Given I have not sent any messages yet
    And no session has been created
    And a file "README.md" exists in the project root
    When I type "@README" in the input field
    Then the file search popup should appear
    And the popup should show "README.md" in the results
    And the search should have used the project root path

  @isolated
  @file-search
  @deletion
  Scenario: File search reflects worktree state after AI deletes file
    Given I am in an isolated session with worktree at ".fspec/worktrees/abc-123/"
    And a file "src/old-file.txt" exists in the main project
    And the AI has deleted "src/old-file.txt" from the worktree
    When I type "@old-file" in the input field
    Then the file search popup should appear
    And the popup should NOT show "src/old-file.txt" in the results
