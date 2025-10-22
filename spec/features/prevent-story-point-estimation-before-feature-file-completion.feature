@cli
@validation
@work-unit-management
@bug-fix
@BUG-013
Feature: Prevent story point estimation before feature file completion
  """
  Check work unit type: if type='task', skip validation (tasks don't require feature files)
  For story/bug types: find linked feature file via work unit ID tag (e.g., @AUTH-001)
  If no feature file found OR file has prefill placeholders, block estimation and emit system-reminder
  System-reminder should explain: ACDD requires feature file completion before estimation, suggest completing specifying phase first
  Reuse existing prefill detection logic from temporal validation (similar pattern)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Story and Bug work units CANNOT be estimated until feature file exists and is complete (specifying phase finished)
  #   2. Task work units CAN be estimated at any stage (tasks don't require feature files)
  #   3. When AI attempts invalid estimation, emit system-reminder explaining ACDD estimation rules
  #   4. Estimation requires completed feature file to understand complexity from acceptance criteria
  #
  # EXAMPLES:
  #   1. Story work unit AUTH-001 in 'backlog' state with no feature file → estimation BLOCKED with system-reminder
  #   2. Story work unit AUTH-001 in 'testing' state with completed feature file → estimation ALLOWED
  #   3. Task work unit TASK-001 in 'backlog' state with no feature file → estimation ALLOWED (tasks exempt)
  #   4. Bug work unit BUG-001 in 'specifying' state but feature file has placeholders → estimation BLOCKED until prefill removed
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to be prevented from estimating story points before feature file is complete
    So that I follow ACDD properly and estimate based on actual acceptance criteria

  Scenario: Block estimation for story work unit without feature file
    Given I have a story work unit "AUTH-001" in "backlog" state
    And the work unit has no linked feature file
    When I run "fspec update-work-unit-estimate AUTH-001 5"
    Then the command should exit with code 1
    And the output should contain a system-reminder explaining ACDD estimation rules
    And the system-reminder should suggest completing the specifying phase first
    And the system-reminder should explain that estimation requires completed acceptance criteria

  Scenario: Allow estimation for story work unit with completed feature file
    Given I have a story work unit "AUTH-001" in "testing" state
    And the work unit has a linked feature file with complete scenarios
    And the feature file has no prefill placeholders
    When I run "fspec update-work-unit-estimate AUTH-001 5"
    Then the command should exit with code 0
    And the work unit estimate should be updated to 5

  Scenario: Allow estimation for task work unit at any stage
    Given I have a task work unit "TASK-001" in "backlog" state
    And the work unit has no linked feature file
    When I run "fspec update-work-unit-estimate TASK-001 3"
    Then the command should exit with code 0
    And the work unit estimate should be updated to 3

  Scenario: Block estimation for bug work unit with incomplete feature file
    Given I have a bug work unit "BUG-001" in "specifying" state
    And the work unit has a linked feature file with prefill placeholders
    When I run "fspec update-work-unit-estimate BUG-001 2"
    Then the command should exit with code 1
    And the output should contain a system-reminder about incomplete feature file
    And the system-reminder should mention prefill placeholders must be removed first
