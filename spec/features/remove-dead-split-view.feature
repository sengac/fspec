@done
@TUI-080
Feature: Remove dead split view — subordinates use their own session view now
  """
  Pure code deletion — remove SplitSessionView.tsx, correlationMapping.ts, all split-view state and rendering from AgentView.tsx, and related dead test files. Shared utils keep their functionality but get stale comments cleaned.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SplitSessionView.tsx must be deleted entirely — it was the old supervisor split-pane renderer
  #   2. All split-view state in AgentView.tsx must be removed: _subordinateSessionId, subordinateSessionName, subordinateConversation, isSplitViewSelectMode, splitViewSelectedIndex, and the supervisor detection useEffect (~line 1413)
  #   3. The SplitSessionView render block (~line 5198-5260) and the half-width calculation (~line 4786) must be removed from AgentView
  #   4. correlationMapping.ts can be deleted — only imported by SplitSessionView and its test
  #   5. Test files to delete: watcher-split-view.test.tsx, cross-pane-correlation.test.tsx, discuss-selected.test.tsx, SplitSessionView.workUnitLogic.test.ts
  #   6. Comment-only references to SplitSessionView in shared utils (turnSelection.ts, sessionHeaderUtils.ts, ConversationInputArea.tsx, SelectionSeparatorBar.tsx, useTurnSelection.ts, conversation.ts) must be updated to remove stale mentions — the shared utils themselves stay
  #   7. The split-view select mode keyboard handler (~line 4620) and escape handler (~line 4651) must be removed
  #   8. Tests referencing SplitSessionView in watch-024-supervisor-terminology-refactoring.test.ts must be removed or updated
  #   9. npm run build succeeds with zero errors and npm test passes with all remaining tests green
  #
  # EXAMPLES:
  #   1. After removal, grep -r SplitSessionView src/ returns zero hits, and grep -r isSplitView src/ returns zero hits
  #   2. After removal, correlationMapping.ts no longer exists in src/tui/utils/
  #   3. npm run build produces zero TypeScript errors and npm test shows no test failures
  #
  # ========================================
  Background: User Story
    As a developer
    I want to remove dead split view code
    So that the codebase is cleaner and AgentView.tsx is smaller with no orphaned WATCH-era code

  Scenario: SplitSessionView component and correlationMapping utility are fully removed
    Given the codebase contains SplitSessionView.tsx and correlationMapping.ts
    When I remove the dead split view code
    Then SplitSessionView.tsx no longer exists
    Then correlationMapping.ts no longer exists
    Then grep -r 'SplitSessionView' src/ returns zero hits
    Then grep -r 'isSplitView' src/ returns zero hits

  Scenario: Dead split-view test files are removed
    Given the test directory contains watcher-split-view.test.tsx, cross-pane-correlation.test.tsx, discuss-selected.test.tsx, and SplitSessionView.workUnitLogic.test.ts
    When I remove the dead split view code
    Then none of those test files exist in the repository

  Scenario: Build and tests pass after removal
    Given the dead split view code has been removed
    When I run npm run build
    Then there are zero TypeScript compilation errors
    When I run npm test
    Then all remaining tests pass with no failures
