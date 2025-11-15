@file-ops
@medium
@ddd
@work-unit-management
@data-model
@EXMAP-005
Feature: Event Storm data model for work units
  """
  JSON Schema validation ensures data integrity, version field supports migrations, relationships tracked via relatedTo arrays and specific fields (triggersEvent, when/then)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Event Storm data model uses stable IDs with soft-delete pattern (id, text, deleted, createdAt, deletedAt) consistent with existing rules/examples/questions pattern
  #   2. WorkUnit.eventStorm section stores: level (process_modeling or software_design), sessionDate, items array (events, commands, aggregates, policies, hotspots, external systems, bounded contexts), nextItemId counter
  #   3. Event Storm items have type field: 'event', 'command', 'aggregate', 'policy', 'hotspot', 'external_system', 'bounded_context'
  #   4. Items support relationships via relatedTo array containing IDs of related items (e.g., command triggersEvent, policy when/then)
  #   5. Suggested tags generated from Event Storm stored in suggestedTags object with componentTags, featureGroupTags, technicalTags arrays and reasoning field
  #   6. Data model must be versioned and support JSON Schema validation
  #   7. Yes, add timestamp field for timeline visualization. Makes Event Storm temporal and enables Mermaid timeline diagrams.
  #   8. Yes, add color field matching Event Storming convention (orange/blue/yellow/pink/purple/green/red) for visualization and adherence to Event Storming standard.
  #   9. Use discriminated union with type field for type safety. Each item type has specific required fields (e.g., commands have triggersEvent, policies have when/then).
  #
  # EXAMPLES:
  #   1. WorkUnit AUTH-001 has eventStorm section with level='process_modeling', 5 domain events (UserRegistered, EmailVerified, UserAuthenticated, SessionCreated, PasswordResetRequested), each with id, text, type='event', deleted=false
  #   2. Event with id=0 'UserAuthenticated' has relatedTo=[1] linking to command id=1 'AuthenticateUser' which has triggersEvent=0
  #   3. suggestedTags object contains componentTags=['@auth', '@session-management'], featureGroupTags=['@authentication', '@user-management'], technicalTags=['@oauth2-integration'], reasoning='Derived from Authentication bounded context, User aggregate, and OAuth2Provider external system'
  #   4. Soft-delete: item id=3 marked deleted=true with deletedAt timestamp, still in items array, ID not reused, can be restored via restore commands
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should timestamp field be added to Event Storm items for timeline visualization (like in the attachment's example with timestamp: 5000, 15000)?
  #   A: true
  #
  #   Q: Should the data model include color field (orange, blue, yellow, etc.) to match physical Event Storming sticky note colors for better visualization?
  #   A: true
  #
  #   Q: For TypeScript types, should we create EventStormItem interface with discriminated union based on type field, or simpler base interface with optional fields?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent performing Event Storming during specifying phase
    I want to store Event Storm artifacts in work-units.json
    So that I have persistent structured data for tag discovery and Example Mapping transformation

  Scenario: Store Process Modeling Event Storm with multiple domain events
    Given I have a work unit "AUTH-001" in specifying status
    When I add Event Storm section with level "process_modeling"
    And I add 5 domain events: "UserRegistered", "EmailVerified", "UserAuthenticated", "SessionCreated", "PasswordResetRequested"
    Then each event should have stable ID (0, 1, 2, 3, 4)
    And each event should have type="event"
    And each event should have deleted=false
    And each event should have createdAt timestamp
    And each event should have color="orange" (Event Storming convention)
    And each event should have timestamp field for timeline visualization
    And eventStorm.nextItemId should be 5

  Scenario: Create relationships between commands and events
    Given I have Event Storm with event id=0 "UserAuthenticated"
    And I have command id=1 "AuthenticateUser"
    When I link command to event using triggersEvent field
    Then event id=0 should have relatedTo=[1]
    And command id=1 should have triggersEvent=0
    And command id=1 should have relatedTo=[0]
    And relationship is bidirectional for traceability

  Scenario: Generate suggested tags from Event Storm artifacts
    Given I have Event Storm with bounded context "Authentication"
    And I have aggregates "User", "Session"
    And I have external system "OAuth2Provider"
    When tag suggestion algorithm analyzes Event Storm data
    Then suggestedTags.componentTags should contain "@auth", "@session-management"
    And suggestedTags.featureGroupTags should contain "@authentication", "@user-management"
    And suggestedTags.technicalTags should contain "@oauth2-integration"
    And suggestedTags.reasoning should explain derivation from bounded context and aggregates

  Scenario: Soft-delete Event Storm item with stable ID preservation
    Given I have Event Storm item with id=3 "PolicyItem"
    When I soft-delete item id=3
    Then item id=3 should have deleted=true
    And item id=3 should have deletedAt timestamp
    And item id=3 should remain in items array
    And ID 3 should never be reused for new items
    And item id=3 can be restored using restore commands
    And nextItemId counter should not decrement
