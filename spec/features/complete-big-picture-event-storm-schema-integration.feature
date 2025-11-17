@validator
@validation
@foundation-management
@schema-validation
@json-schema
@EXMAP-012
Feature: Complete Big Picture Event Storm Schema Integration
  """
  Architecture notes:
  - Update src/schemas/generic-foundation.schema.json to include eventStorm property definition
  - Define eventStormItem discriminated union using oneOf with all 7 item types
  - Reuse EventStormBase interface pattern from generic-foundation.ts
  - Schema validation enforces level='big_picture' constraint (const)
  - Type-specific fields validated per item type (actor for commands, responsibilities for aggregates, etc.)
  - Integration point: validateGenericFoundationObject() in generic-foundation-validator.ts
  - Integration point: discover-foundation --finalize validates eventStorm if present in draft
  - Zero-semantics principle: schema validates structure only, no semantic inference
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. eventStorm property must be defined in generic-foundation.schema.json as optional field
  #   2. eventStorm.level must be constrained to 'big_picture' constant in schema
  #   3. eventStormItem discriminated union must validate all 7 item types (event, command, aggregate, policy, hotspot, external_system, bounded_context)
  #   4. Schema validation must enforce type-specific fields (e.g., actor for commands, responsibilities for aggregates)
  #   5. discover-foundation --finalize must validate eventStorm structure if present in draft
  #
  # EXAMPLES:
  #   1. Foundation with Event Storm containing bounded context validates successfully when level='big_picture'
  #   2. Schema validation fails when eventStorm.level='process_modeling' (invalid for foundation)
  #   3. EventStormCommand item with actor field passes validation
  #   4. EventStormBoundedContext item with null color passes validation (conceptual boundary, not sticky note)
  #   5. discover-foundation --finalize rejects draft with invalid Event Storm item type
  #
  # ========================================
  Background: User Story
    As a developer maintaining fspec foundation system
    I want to have Event Storm data validated by JSON Schema during foundation discovery and regeneration
    So that invalid Event Storm structures are caught early and foundation integrity is maintained

  Scenario: Validate foundation with Event Storm containing bounded context
    Given generic-foundation.schema.json includes eventStorm property definition
    And the schema enforces level='big_picture' constraint
    When I validate a foundation.json with eventStorm containing a bounded context
    And the eventStorm.level equals 'big_picture'
    Then the validation should pass
    And no schema errors should be reported

  Scenario: Reject foundation with invalid Event Storm level
    Given generic-foundation.schema.json includes eventStorm property definition
    And the schema enforces level='big_picture' constraint
    When I validate a foundation.json with eventStorm.level='process_modeling'
    Then the validation should fail
    And the error should indicate level must be 'big_picture'
    And the error should include the field path "eventStorm.level"

  Scenario: Validate EventStormCommand item with type-specific fields
    Given generic-foundation.schema.json includes eventStormItem discriminated union
    And the schema defines EventStormCommand with actor field
    When I validate a foundation with EventStormCommand item including actor field
    And the item type is 'command'
    And the item color is 'blue'
    Then the validation should pass
    And the actor field should be preserved in validated data

  Scenario: Validate EventStormBoundedContext with null color
    Given generic-foundation.schema.json includes eventStormItem discriminated union
    And the schema defines EventStormBoundedContext with color=null
    When I validate a foundation with EventStormBoundedContext item
    And the item type is 'bounded_context'
    And the item color is null
    Then the validation should pass
    And the null color should be accepted as valid

  Scenario: Reject draft with invalid Event Storm item type during finalization
    Given I have a foundation.json.draft with eventStorm section
    And the eventStorm contains an item with invalid type 'invalid_type'
    When I run "fspec discover-foundation --finalize"
    Then the finalization should fail
    And the error should indicate invalid item type
    And the draft file should not be deleted
    And foundation.json should not be created
