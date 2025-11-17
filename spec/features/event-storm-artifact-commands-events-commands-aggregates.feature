@event-storm
@medium
@ddd
@discovery-workflow
@cli
@EXMAP-006
Feature: Event Storm artifact commands (events, commands, aggregates)
  """
  Uses loadWorkUnits/saveWorkUnits utilities from src/utils/file-ops.ts for atomic writes with file locking
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Commands: add-domain-event, add-command, add-aggregate take work-unit-id and text as required arguments
  #   2. Each command creates item with auto-incremented ID, type field, color (orange/blue/yellow), deleted=false, createdAt timestamp
  #   3. Commands initialize eventStorm section if not present with level='process_modeling', items=[], nextItemId=0
  #   4. add-command supports --actor flag for who executes command, add-aggregate supports --responsibilities flag (comma-separated list)
  #   5. All commands support --timestamp flag for timeline visualization (milliseconds), --bounded-context for domain association
  #   6. Commands must validate work unit exists and is not in done/blocked state before adding Event Storm items
  #
  # EXAMPLES:
  #   1. Run 'fspec add-domain-event AUTH-001 "UserRegistered"' creates event with id=0, type='event', color='orange', text='UserRegistered'
  #   2. Run 'fspec add-command AUTH-001 "AuthenticateUser" --actor "User"' creates command with id=1, type='command', color='blue', actor='User'
  #   3. Run 'fspec add-aggregate AUTH-001 "User" --responsibilities "Authentication,Profile management"' creates aggregate with id=2, type='aggregate', color='yellow', responsibilities=['Authentication','Profile management']
  #   4. First command on work unit initializes eventStorm section, subsequent commands append to items array and increment nextItemId
  #
  # ========================================
  Background: User Story
    As a AI agent performing Event Storming during specifying phase
    I want to add domain events, commands, and aggregates using fspec CLI
    So that I capture Event Storm sticky notes in structured format for tag discovery and traceability

  Scenario: Add domain event to work unit
    Given I have a work unit "AUTH-001" in specifying status
    When I run "fspec add-domain-event AUTH-001 \"UserRegistered\""
    Then eventStorm section should be initialized with level "process_modeling"
    And an event item should be created with id=0
    And the event should have type="event"
    And the event should have color="orange"
    And the event should have text="UserRegistered"
    And the event should have deleted=false
    And the event should have createdAt timestamp
    And nextItemId should be 1

  Scenario: Add command with actor flag
    Given I have a work unit "AUTH-001" with existing eventStorm section
    When I run "fspec add-command AUTH-001 \"AuthenticateUser\" --actor \"User\""
    Then a command item should be created with id=1
    And the command should have type="command"
    And the command should have color="blue"
    And the command should have text="AuthenticateUser"
    And the command should have actor="User"
    And nextItemId should be 2

  Scenario: Add aggregate with responsibilities flag
    Given I have a work unit "AUTH-001" with 2 Event Storm items
    When I run "fspec add-aggregate AUTH-001 \"User\" --responsibilities \"Authentication,Profile management\""
    Then an aggregate item should be created with id=2
    And the aggregate should have type="aggregate"
    And the aggregate should have color="yellow"
    And the aggregate should have text="User"
    And the aggregate should have responsibilities=["Authentication", "Profile management"]
    And nextItemId should be 3

  Scenario: Initialize eventStorm section on first command
    Given I have a work unit "AUTH-001" without eventStorm section
    When I run "fspec add-domain-event AUTH-001 \"FirstEvent\""
    Then eventStorm section should be created
    And eventStorm.level should be "process_modeling"
    And eventStorm.items should be an empty array initially
    And the new event should be appended to items
    And eventStorm.nextItemId should start at 0 and increment to 1
