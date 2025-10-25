@done
@acdd
@code-review
@slash-commands
@quality-assurance
@cli
@high
@CLI-011
Feature: Slash command for critical story review with ULTRATHINK
  """
  Slash command implementation stored in .claude/commands/review.md
  - Uses ULTRATHINK directive for deep critical analysis
  - Reads work unit metadata via 'fspec show-work-unit' command
  - Discovers linked feature files via work unit tags (e.g., @CLI-011)
  - Reads coverage files to find test and implementation files
  - Uses fspec query commands (get-scenarios, show-coverage, query-work-units) to compare against similar completed stories
  - Validates against CLAUDE.md coding standards and FOUNDATION.md project requirements
  - Checks ACDD workflow compliance via state history timestamps and file modification times
  - Structured output format: Issues Found, Recommendations, Refactoring Opportunities, ACDD Compliance
  - Each issue includes: problem description, suggested fix, and actionable next steps
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command must prompt user to specify work unit ID if none provided as argument
  #   2. Must use ULTRATHINK mode for deep critical analysis of the story
  #   3. Must analyze all aspects: acceptance criteria (feature file), tests, implementation code, and coverage mappings
  #   4. Must identify: logical flaws, bugs introduced/created, anti-patterns, and refactoring opportunities
  #   5. Must check if existing architecture/utilities could be reused instead of new code
  #   6. Should read work unit metadata, linked feature file(s), test files from coverage, and implementation files from coverage
  #   7. Review output should be structured: Issues Found, Recommendations, Refactoring Opportunities, ACDD Compliance
  #   8. Yes, review should compare against similar completed stories using fspec query commands (get-scenarios, show-coverage, query-work-units) to identify inconsistencies in approach, naming conventions, and architectural patterns. If existing commands don't provide sufficient search capability, create a new story for enhanced search/comparison functionality as a dependency.
  #   9. Yes, review must validate consistency with CLAUDE.md coding standards (no 'any' types, ES modules, proper imports, etc.) and FOUNDATION.md project requirements (alignment with project goals, personas, capabilities)
  #   10. Yes, review must check if ACDD workflow was followed properly: verify temporal ordering (feature files created during specifying, tests before implementation), check for Example Mapping data (rules, examples, answered questions), validate state history timestamps align with file modification times
  #   11. Yes, review output must include specific actionable next steps for each issue: show the problem, suggest the fix, and provide concrete commands or actions to resolve it (e.g., 'Issue: Using any type in file.ts:42 → Fix: Replace with proper interface. Action: Edit file.ts line 42 to use UserInterface type')
  #   12. Yes, support reviewing multiple work units at once. Allow '/review CLI-001 CLI-002 CLI-003' syntax to review multiple stories in one go. Output should be structured per work unit with clear separators between reviews.
  #
  # EXAMPLES:
  #   1. User runs '/review CLI-011', command loads work unit, reads feature file, analyzes tests and implementation, outputs structured review with identified issues
  #   2. User runs '/review' without arguments, command asks 'Which work unit would you like me to review? Please provide the work unit ID (e.g., CLI-011)'
  #   3. Review finds test that doesn't actually test the scenario it claims to test, suggests rewriting test to properly validate acceptance criteria
  #   4. Review finds implementation using manual file operations when existing utility function in src/utils/ should be used instead, suggests refactoring
  #   5. Review finds feature file scenario that doesn't have corresponding test coverage, flags as gap in coverage mapping
  #   6. Review detects anti-pattern where code violates CLAUDE.md coding standards (using 'any' type instead of proper types), suggests specific fix
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the review compare against similar completed stories to identify inconsistencies or patterns?
  #   A: true
  #
  #   Q: Should it validate consistency with CLAUDE.md coding standards and FOUNDATION.md project requirements?
  #   A: true
  #
  #   Q: Should it check if the ACDD workflow was followed properly (temporal ordering, Example Mapping before specs, tests before implementation)?
  #   A: true
  #
  #   Q: Should the review output include specific actionable next steps, or just identify issues for the developer to address?
  #   A: true
  #
  #   Q: Should it support reviewing multiple work units at once, or always single work unit reviews?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using Claude Code with fspec
    I want to critically review a story's acceptance criteria, tests, implementation, and coverage
    So that I can identify logical flaws, bugs, anti-patterns, and refactoring opportunities before they become problems

  Scenario: Review single work unit with full analysis
    Given I have a completed work unit CLI-011 with feature file, tests, and implementation
    When I run '/review CLI-011'
    Then the command should load work unit metadata
    And read the linked feature file
    And analyze test files from coverage mappings
    And analyze implementation files from coverage mappings
    And output structured review with sections: Issues Found, Recommendations, Refactoring Opportunities, ACDD Compliance

  Scenario: Prompt for work unit ID when none provided
    Given I am using the /review command
    When I run '/review' without any arguments
    Then I should see the message 'Which work unit would you like me to review? Please provide the work unit ID (e.g., CLI-011)'

  Scenario: Detect test that doesn't validate its scenario
    Given I have a work unit with a test that claims to test 'Login with valid credentials'
    And the test only checks that a function exists, not that login actually works
    When I run '/review AUTH-001'
    Then the review should identify 'Issue: Test does not validate acceptance criteria'
    And suggest 'Fix: Rewrite test to verify actual login behavior with credentials and session creation'
    And provide actionable next steps

  Scenario: Detect manual file operations that should use existing utilities
    Given I have implementation code using manual fs.readFile and fs.writeFile
    And an existing utility function in src/utils/config.ts already handles this pattern
    When I run '/review CONFIG-002'
    Then the review should identify 'Issue: Manual file operations instead of existing utility'
    And suggest 'Fix: Use loadConfig() from src/utils/config.ts instead of manual fs operations'
    And provide specific refactoring steps

  Scenario: Detect coverage gaps in feature file scenarios
    Given I have a feature file with 5 scenarios
    And coverage file shows only 3 scenarios have test mappings
    When I run '/review TEST-001'
    Then the review should identify 'Issue: 2 scenarios lack test coverage'
    And list the uncovered scenario names
    And suggest 'Fix: Add test coverage for missing scenarios'

  Scenario: Detect CLAUDE.md coding standard violations
    Given I have implementation code using 'any' type in file.ts:42
    And CLAUDE.md mandates no 'any' types
    When I run '/review FEAT-001'
    Then the review should identify 'Issue: Using any type in file.ts:42 (violates CLAUDE.md)'
    And suggest 'Fix: Replace with proper interface type'
    And provide 'Action: Edit file.ts line 42 to use UserInterface type'

  Scenario: Review multiple work units at once
    Given I have three completed work units: CLI-001, CLI-002, CLI-003
    When I run '/review CLI-001 CLI-002 CLI-003'
    Then the command should review all three work units
    And output should be structured per work unit with clear separators
    And each work unit review should include: Issues Found, Recommendations, Refactoring Opportunities, ACDD Compliance
