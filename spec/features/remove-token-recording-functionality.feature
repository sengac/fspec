@done
@code-quality
@critical
@cli
@CLEAN-003
Feature: Remove token recording functionality
  """
  Removal rationale: Token tracking via AI self-reporting is unreliable (30-40% accuracy). AI agents don't have access to actual token counts and would be guessing. Automatic extraction from Claude Code logs is not feasible without reverse engineering. API-level tracking only works if we control the API calls, which we don't in Claude Code. The feature adds maintenance burden without providing reliable data.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. MUST remove src/commands/record-tokens.ts and src/commands/record-tokens-help.ts
  #   2. MUST remove actualTokens field from WorkUnit interface in src/types/index.ts
  #   3. MUST remove record-tokens import and registration from src/index.ts
  #   4. MUST remove record-tokens examples from src/help.ts
  #   5. MUST remove or update tests that reference actualTokens or recordTokens
  #   6. MUST remove spec/features/ai-token-usage-tracking.feature and its .coverage file
  #   7. MUST preserve record-iteration and query-metrics commands (they're separate from token tracking)
  #   8. MUST update any documentation that mentions token recording
  #
  # EXAMPLES:
  #   1. Delete src/commands/record-tokens.ts file (102 lines)
  #   2. Delete src/commands/record-tokens-help.ts file (36 lines)
  #   3. Remove 'actualTokens?: number' from WorkUnit interface
  #   4. Remove line 109 from src/index.ts: import { registerRecordTokensCommand } from './commands/record-tokens'
  #   5. Remove lines 1079-1081 from src/help.ts showing record-tokens examples
  #   6. Update work-unit-estimation-and-metrics.test.ts to remove token recording scenario (lines 138-173)
  #   7. Delete spec/features/ai-token-usage-tracking.feature (55 lines)
  #   8. Delete spec/features/ai-token-usage-tracking.feature.coverage
  #
  # ========================================
  Background: User Story
    As a developer maintaining fspec
    I want to remove all token recording functionality
    So that the codebase is cleaner and doesn't contain unreliable features

  Scenario: Remove record-tokens command files
    Given the file "src/commands/record-tokens.ts" exists
    And the file "src/commands/record-tokens-help.ts" exists
    When I remove the record-tokens command implementation
    Then the file "src/commands/record-tokens.ts" should not exist
    And the file "src/commands/record-tokens-help.ts" should not exist

  Scenario: Remove actualTokens field from WorkUnit interface
    Given the file "src/types/index.ts" contains the WorkUnit interface
    And the WorkUnit interface has an "actualTokens?: number" field
    When I remove the actualTokens field from the interface
    Then the file "src/types/index.ts" should not contain "actualTokens"
    And the WorkUnit interface should remain valid TypeScript

  Scenario: Remove record-tokens CLI registration
    Given the file "src/index.ts" imports registerRecordTokensCommand
    And the file "src/index.ts" registers the record-tokens command
    When I remove the record-tokens import and registration
    Then the file "src/index.ts" should not import "registerRecordTokensCommand"
    And the file "src/index.ts" should not reference "record-tokens"

  Scenario: Remove record-tokens from help documentation
    Given the file "src/help.ts" contains record-tokens examples
    When I remove the record-tokens help examples
    Then the file "src/help.ts" should not contain "record-tokens"
    And the help system should remain functional

  Scenario: Update tests to remove token recording references
    Given the file "src/commands/__tests__/work-unit-estimation-and-metrics.test.ts" exists
    And the test file contains a scenario "Record tokens consumed during implementation"
    When I remove the token recording test scenario
    Then the test file should not contain "recordTokens" function calls
    And the test file should not contain "actualTokens" assertions
    And all remaining tests should still pass

  Scenario: Remove ai-token-usage-tracking feature files
    Given the file "spec/features/ai-token-usage-tracking.feature" exists
    And the file "spec/features/ai-token-usage-tracking.feature.coverage" exists
    When I remove the token tracking feature files
    Then the file "spec/features/ai-token-usage-tracking.feature" should not exist
    And the file "spec/features/ai-token-usage-tracking.feature.coverage" should not exist

  Scenario: Preserve record-iteration and query-metrics commands
    Given the file "src/commands/record-iteration.ts" exists
    And the file "src/commands/query-metrics.ts" exists
    When I complete the token recording removal
    Then the file "src/commands/record-iteration.ts" should still exist
    And the file "src/commands/query-metrics.ts" should still exist
    And both commands should remain functional

  Scenario: Verify no references to token recording remain
    Given all token recording code has been removed
    When I search the codebase for "recordTokens" and "actualTokens"
    Then no source files should contain "recordTokens" function references
    And no source files should contain "actualTokens" field references
    And documentation should not mention token recording
    And the build should succeed without errors
