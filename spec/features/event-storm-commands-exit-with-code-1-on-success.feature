@feature-management
@critical
@cli
@event-storm
@bug-fix
@BUG-086
Feature: Event Storm commands exit with code 1 on success
  """
  Commands affected: add-domain-event, add-command, add-policy, add-hotspot. Issue: Commands exit with code 1 even on successful completion. Root cause: Missing or incorrect process.exit(0) calls in command handlers. Fix: Ensure all Event Storm command handlers explicitly return exit code 0 on success and exit code 1 only on actual errors (validation failures, work unit not found, etc.).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Event Storm commands must return exit code 0 on success
  #   2. Event Storm commands must return exit code 1 only on actual errors
  #   3. Command chaining with && must work correctly when all commands succeed
  #
  # EXAMPLES:
  #   1. Running 'fspec add-domain-event UI-001 TestEvent' successfully adds event and returns exit code 0
  #   2. Running 'fspec add-command UI-001 TestCommand' successfully adds command and returns exit code 0
  #   3. Running 'fspec add-policy UI-001 "Send email" --when UserRegistered --then SendEmail' successfully adds policy and returns exit code 0
  #   4. Running 'fspec add-hotspot UI-001 "Email timeout" --concern "Unclear timeout duration"' successfully adds hotspot and returns exit code 0
  #   5. Chaining commands: 'fspec add-domain-event UI-001 Event1 && fspec add-domain-event UI-001 Event2' should execute both commands when both succeed
  #   6. Running 'fspec add-domain-event NONEXISTENT-001 Event' with invalid work unit should return exit code 1
  #
  # ========================================
  Background: User Story
    As a AI agent or developer chaining fspec Event Storm commands
    I want to receive correct exit codes from Event Storm commands
    So that command chaining with && works correctly and CI/CD pipelines don't fail

  Scenario: add-domain-event returns exit code 0 on success
    Given I have a work unit "UI-001" in specifying state
    When I run "fspec add-domain-event UI-001 TestEvent"
    Then the command should exit with code 0
    And the domain event "TestEvent" should be added to UI-001

  Scenario: add-command returns exit code 0 on success
    Given I have a work unit "UI-001" in specifying state
    When I run "fspec add-command UI-001 TestCommand"
    Then the command should exit with code 0
    And the command "TestCommand" should be added to UI-001

  Scenario: add-policy returns exit code 0 on success
    Given I have a work unit "UI-001" in specifying state
    When I run "fspec add-policy UI-001 'Send email' --when UserRegistered --then SendEmail"
    Then the command should exit with code 0
    And the policy "Send email" should be added to UI-001

  Scenario: add-hotspot returns exit code 0 on success
    Given I have a work unit "UI-001" in specifying state
    When I run "fspec add-hotspot UI-001 'Email timeout' --concern 'Unclear timeout duration'"
    Then the command should exit with code 0
    And the hotspot "Email timeout" should be added to UI-001

  Scenario: Command chaining with && executes all commands on success
    Given I have a work unit "UI-001" in specifying state
    When I run "fspec add-domain-event UI-001 Event1 && fspec add-domain-event UI-001 Event2"
    Then both commands should execute
    And the domain event "Event1" should be added to UI-001
    And the domain event "Event2" should be added to UI-001

  Scenario: Event Storm command returns exit code 1 on error
    Given I have no work unit "NONEXISTENT-001"
    When I run "fspec add-domain-event NONEXISTENT-001 Event"
    Then the command should exit with code 1
    And an error message should be displayed
