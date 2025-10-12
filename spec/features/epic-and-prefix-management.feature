@phase7
@cli
@project-management
@epics
@prefixes
Feature: Epic and Prefix Management
  """
  Architecture notes:
  - Epics group related work units for high-level tracking
  - Prefixes determine work unit ID format (AUTH-001, DASH-002)
  - Epics stored in spec/epics.json
  - Prefixes stored in spec/prefixes.json
  - Work units reference epic by ID
  - Epic tracks total estimate and completion percentage

  Critical implementation requirements:
  - MUST validate epic ID format (lowercase-with-hyphens)
  - MUST validate prefix format (2-6 uppercase letters)
  - MUST prevent duplicate epic IDs
  - MUST prevent duplicate prefix values
  - MUST track work units per epic
  - MUST calculate epic progress from work unit states
  - MUST support epic hierarchies (optional)

  Data model:
  - epics.json: {id, title, description, workUnits[], totalEstimate, completedEstimate}
  - prefixes.json: {prefix, description, nextId, epic?}
  - work-units.json: Each work unit has optional epic field

  References:
  - Project Management Design: project-management.md (section 12: Epics)
  """

  Background: User Story
    As an AI agent organizing work
    I want to group work units into epics and manage ID prefixes
    So that I can track high-level progress and maintain consistent naming

  @critical
  @happy-path
  Scenario: Create epic
    Given I have a project with spec directory
    When I run "fspec create-epic epic-user-management 'User Management' --description='All user-related features'"
    Then the command should succeed
    And an epic "epic-user-management" should be created in spec/epics.json
    And the epic should have title "User Management"

  @happy-path
  Scenario: Create prefix for work unit IDs
    Given I have a project with spec directory
    When I run "fspec create-prefix AUTH 'Authentication work units'"
    Then the command should succeed
    And a prefix "AUTH" should be created in spec/prefixes.json
    And the prefix should have nextId of 1

  @happy-path
  Scenario: Link prefix to epic
    Given I have a project with spec directory
    And an epic "epic-auth" exists
    And a prefix "AUTH" exists
    When I run "fspec update-prefix AUTH --epic=epic-auth"
    Then the command should succeed
    And the prefix should be linked to epic "epic-auth"

  @query
  @progress
  Scenario: Show epic progress
    Given I have a project with spec directory
    And an epic "epic-auth" exists
    And work units in epic:
      | id       | status       | estimate |
      | AUTH-001 | done         | 8        |
      | AUTH-002 | done         | 5        |
      | AUTH-003 | implementing | 3        |
    When I run "fspec show-epic epic-auth"
    Then the output should show total estimate: 16 points
    And the output should show completed: 13 points
    And the output should show progress: 81%

  @validation
  @error-handling
  Scenario: Reject invalid epic ID format
    Given I have a project with spec directory
    When I run "fspec create-epic InvalidEpic 'Title'"
    Then the command should fail
    And the error should contain "Epic ID must be lowercase with hyphens"

  @validation
  @error-handling
  Scenario: Reject invalid prefix format
    Given I have a project with spec directory
    When I run "fspec create-prefix auth 'Description'"
    Then the command should fail
    And the error should contain "Prefix must be 2-6 uppercase letters"

  @list
  @query
  Scenario: List all epics with progress
    Given I have a project with spec directory
    And epic "epic-auth" exists with title "Authentication"
    And epic "epic-dashboard" exists with title "Dashboard"
    And work units exist in epics:
      | id       | epic           | status       |
      | AUTH-001 | epic-auth      | done         |
      | AUTH-002 | epic-auth      | implementing |
      | DASH-001 | epic-dashboard | done         |
    When I run "fspec list-epics"
    Then the output should list 2 epics
    And the output should show epic "epic-auth" with completion 1/2 (50%)
    And the output should show epic "epic-dashboard" with completion 1/1 (100%)

  @delete
  @cascade
  Scenario: Delete epic and unlink work units
    Given I have a project with spec directory
    And an epic "epic-auth" exists
    And work units "AUTH-001", "AUTH-002" are in epic "epic-auth"
    When I run "fspec delete-epic epic-auth --force"
    Then the command should succeed
    And the epic should not exist
    And work units should have epic field cleared

  @auto-create
  @file-initialization
  @critical
  Scenario: Auto-create prefixes.json when creating first prefix
    Given I have a project with spec directory
    And the file "spec/prefixes.json" does not exist
    When I run "fspec create-prefix HOOK 'Hooks System' --description='Claude Code hook implementations'"
    Then the command should succeed
    And the file "spec/prefixes.json" should be created
    And the prefix "HOOK" should exist in prefixes.json
    And the prefix should have description "Claude Code hook implementations"

  @auto-create
  @file-initialization
  @critical
  Scenario: Auto-create epics.json when creating first epic
    Given I have a project with spec directory
    And the file "spec/epics.json" does not exist
    When I run "fspec create-epic epic-user-management 'User Management'"
    Then the command should succeed
    And the file "spec/epics.json" should be created
    And the epic "epic-user-management" should exist in epics.json

  @graceful-degradation
  @error-handling
  Scenario: List epics when epics.json does not exist
    Given I have a project with spec directory
    And the file "spec/epics.json" does not exist
    When I run "fspec list-epics"
    Then the command should succeed
    And the output should indicate no epics found
    And the file "spec/epics.json" should NOT be created

  @graceful-degradation
  @error-handling
  Scenario: List prefixes when prefixes.json does not exist
    Given I have a project with spec directory
    And the file "spec/prefixes.json" does not exist
    When I run "fspec list-prefixes"
    Then the command should succeed
    And the output should indicate no prefixes found
    And the file "spec/prefixes.json" should NOT be created
