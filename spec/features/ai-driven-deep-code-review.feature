@high
@cli
@code-quality
@review
@ai-driven
@REV-004
Feature: AI-Driven Deep Code Review
  """
  Review output must include system-reminder with AI instructions for deep analysis. Pattern similar to fspec reverse: build systemReminder string, wrap with wrapSystemReminder(), include in result. AI instructions: 1) Read all implementation files from coverage data 2) Analyze code deeply for bugs/patterns 3) Check FOUNDATION.md alignment 4) Report findings conversationally. Preserve existing static analysis (lines 188-229 check for 'any', require, file extensions). Add new section after static checks for AI-driven deep analysis guidance.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Review must use <system-reminder> tags for conversational AI-driven guidance (like fspec reverse)
  #   2. Review must instruct AI to read all implementation files linked to work unit coverage
  #   3. Review must instruct AI to analyze code for bugs, edge cases, race conditions, and logic errors
  #   4. Review must check code alignment with FOUNDATION.md goals and architectural principles
  #   5. Review must detect anti-patterns (duplicated code, God objects, tight coupling, etc.)
  #   6. Review must suggest refactoring opportunities for code sharing and DRY principle
  #   7. Review must preserve existing static checks (ACDD compliance, coverage, coding standards)
  #   8. Review system-reminder must be agent-aware (use getAgentConfig and formatAgentOutput)
  #
  # EXAMPLES:
  #   1. AI runs 'fspec review AUTH-001' and receives system-reminder instructing it to read src/auth/login.ts, analyze for bugs, check against FOUNDATION.md, and report findings
  #   2. Review detects duplicated validation logic across 3 files and suggests creating shared validator utility
  #   3. Review finds race condition in async file operations and suggests using file locking pattern from FOUNDATION.md
  #   4. Review detects anti-pattern: 500-line God function that should be refactored into smaller focused functions
  #   5. Review checks FOUNDATION.md and finds code violates 'keep files under 300 lines' principle with file at 450 lines
  #   6. Review preserves existing static checks: still reports 'any' type usage, missing tests, ACDD violations
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec for code quality
    I want to get deep analysis and actionable guidance on code quality issues
    So that I can fix bugs, refactor code, and align with project architecture before marking work as done

  Scenario: Review provides system-reminder with AI instructions for deep analysis
    Given I have a work unit "AUTH-001" with linked implementation files in coverage data
    And the implementation file is "src/auth/login.ts"
    When I run "fspec review AUTH-001"
    Then the output should contain a <system-reminder> tag
    And the system-reminder should instruct the AI to read "src/auth/login.ts"
    And the system-reminder should instruct the AI to analyze code for bugs and edge cases
    And the system-reminder should instruct the AI to check FOUNDATION.md alignment
    And the system-reminder should instruct the AI to report findings conversationally

  Scenario: Review detects duplicated code and suggests shared utility
    Given I have a work unit "VAL-001" with 3 implementation files
    And all 3 files contain similar validation logic
    When I run "fspec review VAL-001"
    Then the output should contain a <system-reminder> tag
    And the system-reminder should instruct the AI to detect duplicated validation logic
    And the system-reminder should suggest creating a shared validator utility
    And the system-reminder should list the files containing duplicated code

  Scenario: Review finds race condition and suggests FOUNDATION.md pattern
    Given I have a work unit "FILE-001" with async file operations
    And the code has potential race conditions
    And FOUNDATION.md defines file locking patterns
    When I run "fspec review FILE-001"
    Then the output should contain a <system-reminder> tag
    And the system-reminder should instruct the AI to analyze async operations
    And the system-reminder should suggest checking FOUNDATION.md for file locking pattern
    And the system-reminder should guide the AI to detect race conditions

  Scenario: Review detects God function anti-pattern
    Given I have a work unit "PROC-001" with a 500-line function
    When I run "fspec review PROC-001"
    Then the output should contain a <system-reminder> tag
    And the system-reminder should instruct the AI to detect large functions
    And the system-reminder should suggest refactoring into smaller focused functions
    And the system-reminder should mention the God object anti-pattern

  Scenario: Review checks FOUNDATION.md file size principle
    Given I have a work unit "BIG-001" with a file at 450 lines
    And FOUNDATION.md states "keep files under 300 lines"
    When I run "fspec review BIG-001"
    Then the output should contain a <system-reminder> tag
    And the system-reminder should instruct the AI to check file sizes against FOUNDATION.md
    And the system-reminder should report the 450-line file violates the 300-line principle
    And the system-reminder should suggest refactoring to meet the standard

  Scenario: Review preserves existing static checks
    Given I have a work unit "TEST-001" with code using "any" type
    And the work unit has incomplete test coverage
    And the work unit has ACDD compliance violations
    When I run "fspec review TEST-001"
    Then the output should contain critical issue for "any" type usage
    And the output should contain ACDD compliance violations
    And the output should contain incomplete test coverage warnings
    And the output should also contain <system-reminder> for AI-driven deep analysis
