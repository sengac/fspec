@cli
@project-management
@work-units
@crud
Feature: Work Unit Management
  """
  Architecture notes:
  - Work units are tag-based entities stored in spec/work-units.json
  - Each work unit has a unique ID with prefix (e.g., AUTH-001, DASH-002)
  - IDs auto-increment based on existing work units with the same prefix
  - Work units can have parent/child relationships for task decomposition
  - Work units must have at minimum: id, title, status
  - Status defaults to 'backlog' when created
  - Prefixes are configured in spec/prefixes.json
  - JSON Schema validation ensures data integrity
  - Work units link to feature files via tags (@WORK-001) in feature files
  - Feature-level tags apply to all scenarios, scenario-level tags override
  - fspec show-work-unit scans feature files to display linked scenarios

  Critical implementation requirements:
  - MUST validate work unit ID format: [A-Z]{2,6}-\d+
  - MUST prevent duplicate work unit IDs
  - MUST enforce parent/child consistency (parent can't be done until children done)
  - MUST validate status is one of: backlog, specifying, testing, implementing, validating, done, blocked
  - MUST update timestamps (createdAt, updatedAt) automatically
  - MUST maintain states index for fast queries
  - DELETE operations must check for dependencies and children
  - show-work-unit MUST scan all feature files for work unit tags
  - MUST support hybrid tag inheritance (feature-level + scenario-level)
  - MUST display linked scenarios with file path and line numbers

  Data model:
  - spec/work-units.json: Main work unit data with states index
  - spec/prefixes.json: Prefix definitions and configuration
  - spec/epics.json: Epic definitions
  - Work unit linking: Tags in feature files (@WORK-001) - NO storage in JSON

  References:
  - Project Management Design: project-management.md
  - ACDD Workflow: spec/CLAUDE.md
  - Work Unit Linking: Tags in feature files (scan-based, not stored)
  """

  Background: User Story
    As an AI agent practicing ACDD
    I want to create and manage work units through CLI commands
    So that I can organize, track, and complete work systematically

  @critical
  @happy-path
  Scenario: Create work unit with auto-incrementing ID
    Given I have a project with spec directory
    And the prefix "AUTH" is registered in spec/prefixes.json
    And no work units exist with prefix "AUTH"
    When I run "fspec create-work-unit AUTH 'Implement OAuth login'"
    Then the command should succeed
    And a work unit "AUTH-001" should be created in spec/work-units.json
    And the work unit should have title "Implement OAuth login"
    And the work unit should have status "backlog"
    And the work unit should have createdAt timestamp
    And the work unit should have updatedAt timestamp
    And the states.backlog array should contain "AUTH-001"

  @happy-path
  Scenario: Create second work unit with incremented ID
    Given I have a project with spec directory
    And the prefix "AUTH" is registered
    And a work unit "AUTH-001" exists
    When I run "fspec create-work-unit AUTH 'Add password reset'"
    Then the command should succeed
    And a work unit "AUTH-002" should be created
    And the work unit should have status "backlog"
    And the states.backlog array should contain "AUTH-002"

  @happy-path
  Scenario: Create work unit with epic assignment
    Given I have a project with spec directory
    And the prefix "AUTH" is registered
    And an epic "epic-user-management" exists
    When I run "fspec create-work-unit AUTH 'OAuth integration' --epic=epic-user-management"
    Then the command should succeed
    And the work unit "AUTH-001" should have epic "epic-user-management"
    And the epic should reference work unit "AUTH-001"

  @happy-path
  Scenario: Create work unit with description
    Given I have a project with spec directory
    And the prefix "AUTH" is registered
    When I run "fspec create-work-unit AUTH 'OAuth login' --description='Add OAuth 2.0 with Google and GitHub'"
    Then the command should succeed
    And the work unit should have description "Add OAuth 2.0 with Google and GitHub"

  @happy-path
  Scenario: Create child work unit with parent
    Given I have a project with spec directory
    And the prefix "AUTH" is registered
    And a work unit "AUTH-001" exists with title "OAuth integration"
    When I run "fspec create-work-unit AUTH 'Google provider' --parent=AUTH-001"
    Then the command should succeed
    And the work unit "AUTH-002" should be created
    And the work unit "AUTH-002" should have parent "AUTH-001"
    And the work unit "AUTH-001" children array should contain "AUTH-002"

  @validation
  @error-handling
  Scenario: Attempt to create work unit with unregistered prefix
    Given I have a project with spec directory
    And the prefix "INVALID" is not registered
    When I run "fspec create-work-unit INVALID 'Some work'"
    Then the command should fail
    And the error should contain "Prefix 'INVALID' is not registered"
    And the error should suggest "Run 'fspec create-prefix INVALID' first"

  @validation
  @error-handling
  Scenario: Attempt to create work unit with missing title
    Given I have a project with spec directory
    And the prefix "AUTH" is registered
    When I run "fspec create-work-unit AUTH"
    Then the command should fail
    And the error should contain "Title is required"

  @validation
  @error-handling
  Scenario: Attempt to create child with non-existent parent
    Given I have a project with spec directory
    And the prefix "AUTH" is registered
    And no work unit "AUTH-999" exists
    When I run "fspec create-work-unit AUTH 'Child work' --parent=AUTH-999"
    Then the command should fail
    And the error should contain "Parent work unit 'AUTH-999' does not exist"

  @update
  @happy-path
  Scenario: Update work unit title
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with title "Old title"
    When I run "fspec update-work-unit AUTH-001 --title='New title'"
    Then the command should succeed
    And the work unit "AUTH-001" should have title "New title"
    And the updatedAt timestamp should be updated

  @update
  @happy-path
  Scenario: Update work unit description
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec update-work-unit AUTH-001 --description='Updated description'"
    Then the command should succeed
    And the work unit should have description "Updated description"

  @update
  @happy-path
  Scenario: Update work unit epic
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And an epic "epic-security" exists
    When I run "fspec update-work-unit AUTH-001 --epic=epic-security"
    Then the command should succeed
    And the work unit should have epic "epic-security"
    And the epic should reference work unit "AUTH-001"

  @update
  @validation
  Scenario: Attempt to update work unit with invalid epic
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And no epic "epic-nonexistent" exists
    When I run "fspec update-work-unit AUTH-001 --epic=epic-nonexistent"
    Then the command should fail
    And the error should contain "Epic 'epic-nonexistent' does not exist"

  @read
  @happy-path
  Scenario: Show single work unit details
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with:
      | field       | value                 |
      | title       | OAuth integration     |
      | description | Add OAuth 2.0 support |
      | status      | implementing          |
      | estimate    | 5                     |
    When I run "fspec show-work-unit AUTH-001"
    Then the command should succeed
    And the output should display work unit details
    And the output should contain "AUTH-001"
    And the output should contain "OAuth integration"
    And the output should contain "implementing"

  @read
  @happy-path
  Scenario: Show work unit with JSON output
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec show-work-unit AUTH-001 --output=json"
    Then the command should succeed
    And the output should be valid JSON
    And the JSON should have field "id" with value "AUTH-001"
    And the JSON should have field "title"
    And the JSON should have field "status"

  @read
  @work-unit-linking
  Scenario: Show work unit with linked feature files and scenarios
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists with title "OAuth Login Implementation"
    And a feature file "oauth-login.feature" is tagged with "@AUTH-001"
    And the feature has 3 scenarios at lines 15, 25, and 35
    When I run "fspec show-work-unit AUTH-001"
    Then the command should succeed
    And the output should show "Linked Features & Scenarios:"
    And the output should show "oauth-login.feature:15 - Login with Google"
    And the output should show "oauth-login.feature:25 - Login with GitHub"
    And the output should show "oauth-login.feature:35 - Handle OAuth errors"
    And the output should show "Total: 3 scenarios"

  @read
  @work-unit-linking
  Scenario: Show work unit with scenario-level tags
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And a work unit "AUTH-002" exists
    And a feature file "oauth.feature" has feature-level tag "@AUTH-001"
    And the feature has a scenario "Login" inheriting @AUTH-001
    And the feature has a scenario "Refresh tokens" with tag "@AUTH-002"
    When I run "fspec show-work-unit AUTH-001"
    Then the command should succeed
    And the output should show scenario "Login" under AUTH-001
    And the output should not show scenario "Refresh tokens"
    When I run "fspec show-work-unit AUTH-002"
    Then the output should show only scenario "Refresh tokens"

  @read
  @work-unit-linking
  Scenario: Show work unit with no linked features
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And no feature files are tagged with "@AUTH-001"
    When I run "fspec show-work-unit AUTH-001"
    Then the command should succeed
    And the output should show "Linked Features & Scenarios: None"

  @read
  @work-unit-linking
  Scenario: Show work unit with JSON output including linked features
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And a feature file "oauth.feature" is tagged with "@AUTH-001"
    And the feature has 2 scenarios
    When I run "fspec show-work-unit AUTH-001 --output=json"
    Then the command should succeed
    And the JSON should have "linkedFeatures" array
    And the linkedFeatures array should contain "oauth.feature"
    And the JSON should have "linkedScenarios" array with 2 items
    And each scenario should have "file", "line", and "name" fields

  @read
  @work-unit-linking
  Scenario: Show work unit with multiple feature files
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And feature file "oauth-login.feature" is tagged with "@AUTH-001"
    And feature file "oauth-refresh.feature" is tagged with "@AUTH-001"
    When I run "fspec show-work-unit AUTH-001"
    Then the command should succeed
    And the output should show both feature files
    And scenarios should be grouped by feature file

  @read
  @happy-path
  Scenario: List all work units
    Given I have a project with spec directory
    And work units exist:
      | id       | title          | status       |
      | AUTH-001 | OAuth login    | done         |
      | AUTH-002 | Password reset | implementing |
      | DASH-001 | User dashboard | backlog      |
    When I run "fspec list-work-units"
    Then the command should succeed
    And the output should contain "AUTH-001"
    And the output should contain "AUTH-002"
    And the output should contain "DASH-001"

  @read
  @filtering
  Scenario: List work units filtered by status
    Given I have a project with spec directory
    And work units exist:
      | id       | status       |
      | AUTH-001 | backlog      |
      | AUTH-002 | implementing |
      | DASH-001 | backlog      |
    When I run "fspec list-work-units --status=backlog"
    Then the command should succeed
    And the output should contain "AUTH-001"
    And the output should contain "DASH-001"
    And the output should not contain "AUTH-002"

  @read
  @filtering
  Scenario: List work units filtered by prefix
    Given I have a project with spec directory
    And work units exist:
      | id       | title     |
      | AUTH-001 | Auth work |
      | AUTH-002 | More auth |
      | DASH-001 | Dashboard |
    When I run "fspec list-work-units --prefix=AUTH"
    Then the command should succeed
    And the output should contain "AUTH-001"
    And the output should contain "AUTH-002"
    And the output should not contain "DASH-001"

  @read
  @filtering
  Scenario: List work units filtered by epic
    Given I have a project with spec directory
    And an epic "epic-user-management" exists
    And work units exist:
      | id       | epic                 |
      | AUTH-001 | epic-user-management |
      | AUTH-002 | epic-user-management |
      | SEC-001  | epic-security        |
    When I run "fspec list-work-units --epic=epic-user-management"
    Then the command should succeed
    And the output should contain "AUTH-001"
    And the output should contain "AUTH-002"
    And the output should not contain "SEC-001"

  @delete
  @happy-path
  Scenario: Delete work unit with no dependencies
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And the work unit has no children
    And the work unit is not blocking other work
    When I run "fspec delete-work-unit AUTH-001"
    Then the command should prompt for confirmation
    When I confirm the deletion
    Then the command should succeed
    And the work unit "AUTH-001" should not exist in spec/work-units.json
    And the work unit should be removed from states index

  @delete
  @happy-path
  Scenario: Force delete work unit without confirmation
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec delete-work-unit AUTH-001 --force"
    Then the command should succeed without prompting
    And the work unit "AUTH-001" should not exist

  @delete
  @validation
  Scenario: Attempt to delete work unit with children
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And a work unit "AUTH-002" exists with parent "AUTH-001"
    When I run "fspec delete-work-unit AUTH-001 --force"
    Then the command should fail
    And the error should contain "Cannot delete work unit with children"
    And the error should list child "AUTH-002"
    And the error should suggest "Delete children first or remove parent relationship"

  @delete
  @validation
  Scenario: Attempt to delete work unit that blocks other work
    Given I have a project with spec directory
    And a work unit "API-001" exists
    And a work unit "AUTH-001" exists with blockedBy "API-001"
    When I run "fspec delete-work-unit API-001 --force"
    Then the command should fail
    And the error should contain "Cannot delete work unit that blocks other work"
    And the error should list blocked work unit "AUTH-001"
    And the error should suggest "Remove blocking relationships first"

  @parent-child
  @happy-path
  Scenario: Create nested work unit hierarchy
    Given I have a project with spec directory
    And the prefix "AUTH" is registered
    When I run "fspec create-work-unit AUTH 'OAuth 2.0 implementation'"
    And I run "fspec create-work-unit AUTH 'Google provider' --parent=AUTH-001"
    And I run "fspec create-work-unit AUTH 'GitHub provider' --parent=AUTH-001"
    And I run "fspec create-work-unit AUTH 'Token storage' --parent=AUTH-001"
    Then work unit "AUTH-001" should have 3 children
    And the children should be "AUTH-002", "AUTH-003", "AUTH-004"
    And each child should have parent "AUTH-001"

  @parent-child
  @validation
  Scenario: Attempt to create circular parent relationship
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And a work unit "AUTH-002" exists with parent "AUTH-001"
    When I run "fspec update-work-unit AUTH-001 --parent=AUTH-002"
    Then the command should fail
    And the error should contain "Circular parent relationship detected"

  @parent-child
  @validation
  Scenario: Attempt to exceed maximum nesting depth
    Given I have a project with spec directory
    And work units exist with nesting:
      | id       | parent   |
      | AUTH-001 | null     |
      | AUTH-002 | AUTH-001 |
      | AUTH-003 | AUTH-002 |
    When I run "fspec create-work-unit AUTH 'Too deep' --parent=AUTH-003"
    Then the command should fail
    And the error should contain "Maximum nesting depth (3) exceeded"

  @validation
  @json-schema
  Scenario: Validate work unit data structure
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    When I run "fspec validate-work-units"
    Then the command should succeed
    And the validation should check JSON schema compliance
    And the validation should check parent/child consistency
    And the validation should check work unit IDs are unique

  @validation
  @error-handling
  Scenario: Attempt to perform operations on non-existent work unit
    Given I have a project with spec directory
    And no work unit "AUTH-999" exists
    When I run "fspec show-work-unit AUTH-999"
    Then the command should fail
    And the error should contain "Work unit 'AUTH-999' does not exist"

  @auto-create
  @file-initialization
  @critical
  Scenario: Auto-create work-units.json when missing
    Given I have a project with spec directory
    And the file "spec/work-units.json" does not exist
    And the prefix "HOOK" is registered in spec/prefixes.json
    When I run "fspec create-work-unit HOOK 'Hook Handler'"
    Then the command should succeed
    And the file "spec/work-units.json" should be created with initial structure
    And the structure should include meta section with version and lastUpdated
    And the structure should include states: backlog, specifying, testing, implementing, validating, done, blocked
    And the structure should include workUnits object
    And the work unit "HOOK-001" should be created successfully
    And "HOOK-001" should be in the backlog state array

  @auto-create
  @file-initialization
  Scenario: Auto-create prefixes.json when reading work units
    Given I have a project with spec directory
    And the file "spec/prefixes.json" does not exist
    And the file "spec/work-units.json" exists with work unit data
    When I run "fspec list-work-units"
    Then the command should succeed
    And the file "spec/prefixes.json" should be created with empty structure
    And the command should list all work units

  @auto-create
  @file-initialization
  Scenario: Auto-create epics.json when needed for work unit operations
    Given I have a project with spec directory
    And the file "spec/epics.json" does not exist
    And the prefix "AUTH" is registered
    And spec/work-units.json exists
    When I run "fspec create-work-unit AUTH 'Login feature' --epic=epic-auth"
    Then the command should fail
    And the error should contain "Epic 'epic-auth' does not exist"
    And the file "spec/epics.json" should be created with empty structure

  @work-unit-linking
  @e2e
  @critical
  Scenario: Complete end-to-end work unit to feature linking workflow
    Given I have a project with spec directory
    And the prefix "AUTH" is registered in spec/prefixes.json
    And a feature file "oauth-login.feature" exists without work unit tags
    When I run "fspec create-work-unit AUTH 'OAuth Login Implementation'"
    Then the command should succeed
    And a work unit "AUTH-001" should be created with title "OAuth Login Implementation"
    When I run "fspec add-tag-to-feature oauth-login.feature @AUTH-001"
    Then the command should succeed
    And the feature file should contain tag "@AUTH-001"
    When I run "fspec show-work-unit AUTH-001"
    Then the command should succeed
    And the output should show linked feature "oauth-login.feature"
    And the output should list all scenarios from the feature
    When I run "fspec show-feature oauth-login.feature"
    Then the command should succeed
    And the output should show work unit "AUTH-001" linked to this feature
    When I run "fspec validate-tags oauth-login.feature"
    Then the command should succeed
    And work unit tag "@AUTH-001" should be validated against spec/work-units.json

  @work-unit-linking
  @e2e
  @critical
  Scenario: Remove work unit tag from feature and verify unlinking
    Given I have a project with spec directory
    And the prefix "AUTH" is registered in spec/prefixes.json
    And a work unit "AUTH-001" exists with title "OAuth Login Implementation"
    And a feature file "oauth-login.feature" is tagged with "@AUTH-001"
    When I run "fspec show-work-unit AUTH-001"
    Then the command should succeed
    And the output should show linked feature "oauth-login.feature"
    When I run "fspec remove-tag-from-feature oauth-login.feature @AUTH-001"
    Then the command should succeed
    And the feature file should not contain tag "@AUTH-001"
    When I run "fspec show-work-unit AUTH-001"
    Then the command should succeed
    And the output should show "Linked Features & Scenarios: None"
    When I run "fspec show-feature oauth-login.feature"
    Then the command should succeed
    And the output should not show work unit "AUTH-001"
