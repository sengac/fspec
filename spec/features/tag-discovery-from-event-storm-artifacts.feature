@event-storming
@tag-management
@cli
@high
@EXMAP-008
Feature: Tag discovery from Event Storm artifacts

  """
  Architecture notes:
  - Command analyzes work unit's eventStorm.items array to extract bounded contexts, aggregates, events, and external systems
  - Tag suggestions use pattern matching and clustering algorithms to identify related domain concepts
  - Output format includes category (component/feature-group/technical), tag name (kebab-case), source artifact reference, and confidence score (high/medium/low)
  - Tag name normalization converts CamelCase/PascalCase/snake_case to kebab-case using regex transformation
  - Dependencies: work-units.json eventStorm section (created by add-domain-event, add-aggregate, add-bounded-context commands)
  - No external AI/ML services required - uses deterministic pattern matching and string similarity algorithms
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command suggest-tags-from-events analyzes work unit's eventStorm section to suggest component, feature group, and technical tags
  #   2. Bounded contexts map to @component tags (e.g., bounded context 'User Management' suggests @user-management component tag)
  #   3. Domain event clusters map to @feature-group tags (e.g., events 'UserRegistered', 'UserLoggedIn' suggest @authentication feature group)
  #   4. Aggregates map to @component tags (e.g., 'User' aggregate suggests @user component tag)
  #   5. External systems suggest @integration or @external-service technical tags based on integration type
  #   6. Command outputs suggested tags in structured format with category, tag name, source artifact, and confidence score
  #   7. Command fails with error if work unit has no Event Storm artifacts or eventStorm section is empty
  #   8. Tag names are normalized to kebab-case format (lowercase with hyphens)
  #
  # EXAMPLES:
  #   1. Work unit has bounded context 'User Management' → suggests @component:@user-management with confidence:high
  #   2. Work unit has events 'UserRegistered', 'UserLoggedIn', 'PasswordReset' → suggests @feature-group:@authentication
  #   3. Work unit has aggregate 'User' with events 'UserRegistered', 'ProfileUpdated' → suggests @component:@user
  #   4. Work unit has external system 'OAuth2Provider' with integrationType:REST_API → suggests @technical:@oauth, @technical:@rest-api
  #   5. Work unit has no Event Storm artifacts → command fails with error 'No Event Storm artifacts found for WORK-001'
  #   6. Multiple related events like 'CheckpointCreated', 'CheckpointRestored', 'CheckpointDeleted' → suggests @feature-group:@checkpoint-management
  #
  # ========================================

  Background: User Story
    As a AI agent or developer using Event Storming for domain discovery
    I want to automatically suggest relevant tags based on Event Storm artifacts
    So that I can quickly create appropriate component, feature group, and technical tags aligned with the domain model without manual analysis

  Scenario: Suggest component tag from bounded context
    Given I have a work unit TEST-001 with Event Storm data
    And the work unit has a bounded context "User Management"
    When I run fspec suggest-tags-from-events TEST-001
    Then the command should succeed
    And the output should suggest a component tag "@user-management"
    And the suggestion should have confidence "high"
    And the suggestion should reference source "bounded_context: User Management"

  Scenario: Suggest feature group tag from domain event cluster
    Given I have a work unit TEST-002 with Event Storm data
    And the work unit has domain events "UserRegistered", "UserLoggedIn", "PasswordReset"
    When I run fspec suggest-tags-from-events TEST-002
    Then the command should succeed
    And the output should suggest a feature group tag "@authentication"
    And the suggestion should reference events "UserRegistered, UserLoggedIn, PasswordReset"

  Scenario: Suggest component tag from aggregate
    Given I have a work unit TEST-003 with Event Storm data
    And the work unit has an aggregate "User"
    And the aggregate has domain events "UserRegistered", "ProfileUpdated"
    When I run fspec suggest-tags-from-events TEST-003
    Then the command should succeed
    And the output should suggest a component tag "@user"
    And the suggestion should reference source "aggregate: User"

  Scenario: Suggest technical tags from external system
    Given I have a work unit TEST-004 with Event Storm data
    And the work unit has an external system "OAuth2Provider" with integrationType "REST_API"
    When I run fspec suggest-tags-from-events TEST-004
    Then the command should succeed
    And the output should suggest technical tag "@oauth"
    And the output should suggest technical tag "@rest-api"
    And both suggestions should reference source "external_system: OAuth2Provider"

  Scenario: Fail when work unit has no Event Storm artifacts
    Given I have a work unit TEST-005 with no Event Storm data
    When I run fspec suggest-tags-from-events TEST-005
    Then the command should fail with exit code 1
    And the error message should contain "No Event Storm artifacts found for TEST-005"

  Scenario: Suggest feature group tag from multiple related events
    Given I have a work unit TEST-006 with Event Storm data
    And the work unit has domain events "CheckpointCreated", "CheckpointRestored", "CheckpointDeleted"
    When I run fspec suggest-tags-from-events TEST-006
    Then the command should succeed
    And the output should suggest a feature group tag "@checkpoint-management"
    And the tag name should be in kebab-case format
