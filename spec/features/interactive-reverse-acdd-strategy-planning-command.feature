@workflow
@done
@strategy-planning
@session-based
@critical
@reverse-acdd
@cli
@REV-001
Feature: Interactive reverse ACDD strategy planning command
  """
  Session state stored in OS temp directory (tmpdir) with project-specific hash for isolation - ephemeral, OS-managed cleanup
  State machine phases: analyzing → gap-detection → strategy-planning → executing → complete (similar to discover-foundation feedback loop)
  System-reminders emitted at phase transitions and for step-by-step guidance (wrapped in <system-reminder> tags)
  Reuses existing analysis utilities: file scanners (Glob), Gherkin parser, test file pattern matching
  NO execution logic in command - all work done by AI using existing fspec commands (create-feature, link-coverage, etc.)
  Integration point 1: .claude/commands/fspec.md must mention 'fspec reverse' and deprecate /rspec command
  Integration point 2: spec/CLAUDE.md section 'Reverse ACDD for Existing Codebases' must be updated with new workflow
  Strategy templates: A=Spec Gap Filling, B=Test Gap Filling, C=Coverage Mapping, D=Full Reverse ACDD (each has predefined guidance steps)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command must be interactive like discover-foundation (session-based with system-reminders)
  #   2. Command analyzes project state but does NOT execute reverse ACDD itself - it only guides
  #   3. Must detect gaps: missing features, missing tests, missing coverage mappings, missing work units
  #   4. Must offer multiple strategies based on gap analysis (Spec Gap Filling, Test Gap Filling, Coverage Mapping, Full Reverse ACDD)
  #   5. Must emit step-by-step guidance via system-reminders (NOT execute the steps)
  #   6. Must persist session state in OS temp directory (tmpdir) with project-specific hash - ephemeral, OS-managed cleanup
  #   7. Must support CLI flags: --continue, --strategy=X, --status, --reset, --complete
  #   8. Must replace /rspec command in .claude/commands/fspec.md and deprecate it
  #   9. Must update spec/CLAUDE.md section on Reverse ACDD with new workflow
  #   10. Session file must be deleted on completion (--complete) or reset (--reset)
  #   11. State machine phases: analyzing → gap-detection → strategy-planning → executing → complete
  #   12. Show summary with counts, paginate detailed gap list, suggest --strategy to narrow scope
  #   13. Yes - include story point estimates in strategy suggestions (helps AI prioritize)
  #   14. Yes - support --dry-run for preview analysis without creating session file
  #
  # EXAMPLES:
  #   1. User runs 'fspec reverse' → command analyzes project → finds 3 test files without features → suggests Strategy A (Spec Gap Filling) via system-reminder
  #   2. AI runs 'fspec reverse --strategy=A' → emits system-reminder with step 1 guidance → AI reads test file, creates feature, links coverage → AI runs 'fspec reverse --continue' → emits step 2 guidance
  #   3. User runs 'fspec reverse --status' → shows current phase (executing), detected gaps (3 test files without features), chosen strategy (A), current step (2 of 3)
  #   4. User runs 'fspec reverse --reset' → deletes session file → outputs 'Session reset' → user can start fresh with 'fspec reverse'
  #   5. Project has features but no tests → analysis detects 2 feature files without tests → suggests Strategy B (Test Gap Filling) → guides AI through creating test skeletons with --skip-validation
  #   6. Project has both features and tests but unmapped → analysis detects 5 scenarios without coverage links → suggests Strategy C (Coverage Mapping) → guides AI through linking existing tests to scenarios
  #   7. Project has raw implementation code only (no features, no tests) → analysis detects 10 implementation files → suggests Strategy D (Full Reverse ACDD) → guides through creating features, tests, and work units from scratch
  #   8. AI completes all steps → runs 'fspec reverse --complete' → command validates work done → deletes session file → emits system-reminder with completion summary
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the session file be human-readable JSON or optimized binary format?
  #   A: true
  #
  #   Q: Should we support parallel strategies (e.g., Strategy A + Strategy C simultaneously)?
  #   A: true
  #
  #   Q: How should we handle very large projects (100+ gaps detected)? Paginate output?
  #   A: true
  #
  #   Q: Should --strategy=custom trigger an interactive Q&A session with the user?
  #   A: true
  #
  #   Q: Should we track time/effort estimates for each strategy in the guidance output?
  #   A: true
  #
  #   Q: Should the command support --dry-run mode to preview analysis without creating session?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Human-readable JSON - easier to debug, inspect, and aligns with foundation.json pattern
  #   2. No parallel strategies in MVP - keep it simple. AI chooses one strategy, completes it, then can run reverse again for other gaps
  #   3. Defer --strategy=custom to Phase 2 - start with four predefined strategies (A, B, C, D)
  #
  # ========================================
  Background: User Story
    As a AI agent (Claude) working on legacy codebase
    I want to plan and execute reverse ACDD strategy to fill specification/test gaps
    So that I can intelligently choose the right approach based on project state rather than following rigid instructions

  Scenario: Initial analysis detects test files without features and suggests Strategy A
    Given I have a project with 3 test files in "src/__tests__/"
    And those test files have no corresponding feature files
    And no reverse session exists
    When I run "fspec reverse"
    Then the command should analyze the project structure
    And the command should create a session file
    And the session should be in "gap-detection" phase
    And the output should show "3 test files without features"
    And the output should suggest "Strategy A: Spec Gap Filling"
    And the output should emit a system-reminder with strategy guidance
    And the system-reminder should tell me to run "fspec reverse --strategy=A" to choose the strategy

  Scenario: Choose Strategy A and receive step-by-step guidance
    Given I have a reverse session in "strategy-planning" phase
    And the session detected 3 test files without features
    When I run "fspec reverse --strategy=A"
    Then the session should transition to "executing" phase
    And the session should set currentStep to 1
    And the command should emit a system-reminder with step 1 guidance
    And the guidance should instruct me to read the first test file
    And the guidance should instruct me to create a feature file
    And the guidance should instruct me to run "fspec link-coverage" with --skip-validation
    And the system-reminder should tell me to run "fspec reverse --continue" after completing step 1

  Scenario: Continue to next step in strategy execution
    Given I have a reverse session in "executing" phase
    And the session is on step 1 of 3
    And I have completed the work for step 1
    When I run "fspec reverse --continue"
    Then the session should increment currentStep to 2
    And the command should emit a system-reminder with step 2 guidance
    And the guidance should reference the next test file to process
    And the system-reminder should tell me to run "fspec reverse --continue" after completing step 2

  Scenario: Check session status during execution
    Given I have a reverse session in "executing" phase
    And the chosen strategy is "A"
    And the session detected "3 test files without features"
    And the current step is 2 of 3
    When I run "fspec reverse --status"
    Then the output should show "Phase: executing"
    And the output should show "Strategy: A (Spec Gap Filling)"
    And the output should show "Gaps detected: 3 test files without features"
    And the output should show "Progress: Step 2 of 3"
    And the output should list the 3 gaps with completion status

  Scenario: Reset session and start fresh
    Given I have a reverse session in "executing" phase
    And the session file exists
    When I run "fspec reverse --reset"
    Then the session file should be deleted
    And the output should show "Session reset"
    And I should be able to run "fspec reverse" to start a new session

  Scenario: Prevent starting new session when one already exists
    Given I have a reverse session in "executing" phase
    And the session is on step 2 of 3
    And the session file exists
    When I run "fspec reverse"
    Then the command should detect the existing session
    And the command should exit with error code 1
    And the output should show "Existing reverse session detected"
    And the output should show the current session phase "executing"
    And the output should show the current strategy and progress
    And the output should suggest running "fspec reverse --continue"
    And the output should suggest running "fspec reverse --status"
    And the output should suggest running "fspec reverse --reset"
    And the output should suggest running "fspec reverse --complete"
    And the command should NOT overwrite the existing session file
    And the command should emit a system-reminder about the conflict

  Scenario: Detect feature files without tests and suggest Strategy B
    Given I have a project with 2 feature files in "spec/features/"
    And those feature files have no corresponding test files
    And no reverse session exists
    When I run "fspec reverse"
    Then the command should analyze the project structure
    And the session should be in "gap-detection" phase
    And the output should show "2 feature files without tests"
    And the output should suggest "Strategy B: Test Gap Filling"
    And the guidance should mention creating test skeletons
    And the guidance should mention using --skip-validation flag

  Scenario: Detect unmapped coverage and suggest Strategy C
    Given I have a project with 5 feature files
    And I have corresponding test files for those features
    And the coverage files show 5 scenarios without test mappings
    And no reverse session exists
    When I run "fspec reverse"
    Then the command should analyze coverage files
    And the session should be in "gap-detection" phase
    And the output should show "5 scenarios without coverage mappings"
    And the output should suggest "Strategy C: Coverage Mapping"
    And the guidance should mention "Quick wins - no new files needed"
    And the guidance should estimate effort as "1 point total"

  Scenario: Detect raw implementation with no specs or tests and suggest Strategy D
    Given I have a project with 10 implementation files in "src/"
    And the project has no feature files
    And the project has no test files
    And no reverse session exists
    When I run "fspec reverse"
    Then the command should analyze implementation files
    And the session should be in "gap-detection" phase
    And the output should show "10 implementation files without specs or tests"
    And the output should suggest "Strategy D: Full Reverse ACDD"
    And the guidance should mention "Highest effort - analyze code, create features, tests, and work units"
    And the guidance should estimate effort range

  Scenario: Reach final step and receive completion guidance
    Given I have a reverse session in "executing" phase
    And the session is on step 2 of 3
    And I have completed the work for step 2
    When I run "fspec reverse --continue"
    Then the session should increment currentStep to 3
    And the command should emit a system-reminder with step 3 guidance
    And the guidance should reference the final test file to process
    And the system-reminder should tell me to run "fspec reverse --complete" after completing step 3
    And the system-reminder should NOT mention "fspec reverse --continue"

  Scenario: Complete all steps and finalize session
    Given I have a reverse session in "executing" phase
    And I am on the final step (step 3 of 3)
    And I have completed all gap-filling work
    When I run "fspec reverse --complete"
    Then the command should validate all work was completed
    And the command should verify gaps are filled
    And the session file should be deleted
    And the command should emit a system-reminder with completion summary
    And the system-reminder should confirm that all gaps have been filled
    And the output should show "✓ Reverse ACDD session complete"

  Scenario: Preview analysis without creating session (dry-run mode)
    Given I have a project with gaps (missing features or tests)
    And no reverse session exists
    When I run "fspec reverse --dry-run"
    Then the command should analyze the project structure
    And the command should detect gaps
    And the command should suggest strategies
    And the command should NOT create a session file
    And the output should show gap analysis results
    And the output should show "Dry-run mode - no session created"

  Scenario: Handle large projects with pagination
    Given I have a project with 150 test files without features
    And no reverse session exists
    When I run "fspec reverse"
    Then the command should analyze the project structure
    And the output should show "150 test files without features"
    And the output should paginate the detailed gap list
    And the output should show a summary with total counts
    And the guidance should suggest "Use --strategy=A to narrow scope"
