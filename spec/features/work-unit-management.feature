@cli
@project-management
@work-units
@crud
Feature: Work Unit Management
  """
  Architecture notes:
  - Work units are stored in spec/work-units.json; a work unit links to feature files only through work-unit ID tags (@AUTH-001) in those files, and nothing about the link is stored in JSON
  - A feature-level work-unit tag links every scenario of the feature; a scenario-level work-unit tag links only that scenario
  - show-work-unit scans spec/features/ and prints a Linked Features section: each linked feature file, then its linked scenarios as file:line - name, grouped by feature file; with no linked feature the section is left out
  - show-feature ends with the work units the feature links to (feature-level or scenario-level), or "Work Units: None"
  - validate-tags checks a feature-level work-unit tag against spec/work-units.json
  - Creating, updating, showing, listing, deleting and validating work units are specified by each command's own feature: create-story-rust-port, update-work-unit-rust-port, show-work-unit-rust-port, list-work-units-rust-port, delete-work-unit-rust-port, validate-work-units-rust-port
  - Tests: rust/fspec-core/tests/work_unit_management_feature_linking.rs (dispatcher tests on temporary projects)
  """

  Background: User Story
    As an AI agent practicing ACDD
    I want to create and manage work units through CLI commands
    So that I can organize, track, and complete work systematically

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
  Scenario: Show work unit with multiple feature files
    Given I have a project with spec directory
    And a work unit "AUTH-001" exists
    And feature file "oauth-login.feature" is tagged with "@AUTH-001"
    And feature file "oauth-refresh.feature" is tagged with "@AUTH-001"
    When I run "fspec show-work-unit AUTH-001"
    Then the command should succeed
    And the output should show both feature files
    And scenarios should be grouped by feature file

  @work-unit-linking
  @e2e
  @critical
  Scenario: Complete end-to-end work unit to feature linking workflow
    Given I have a project with spec directory
    And the prefix "AUTH" is registered in spec/prefixes.json
    And a feature file "oauth-login.feature" exists without work unit tags
    When I run "fspec create-story AUTH 'OAuth Login Implementation'"
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
    And the output should have no Linked Features section
    When I run "fspec show-feature oauth-login.feature"
    Then the command should succeed
    And the output should not show work unit "AUTH-001"
