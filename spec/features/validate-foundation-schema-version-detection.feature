@done
@phase1
@cli
@validation
@bug
@BUG-014
Feature: validate-foundation-schema uses wrong schema for generic foundations
  """
  Uses validateGenericFoundation for v2.0.0+ and validateFoundationSchema for v1.x. Version detection reads 'version' field from foundation.json. Both validators return same interface: {valid, errors}.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command must detect schema version from foundation.json 'version' field
  #   2. Version 2.0.0 and above should use generic-foundation-validator
  #   3. Version 1.x should use fspec-specific validator for backward compatibility
  #
  # EXAMPLES:
  #   1. Foundation with version: 2.0.0 validates using generic schema successfully
  #   2. Foundation with version: 1.0.0 validates using fspec-specific schema
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to validate foundation.json files
    So that I get correct validation regardless of schema version

  Scenario: Validate foundation with version 2.0.0 using generic schema
    Given I have a foundation.json file with version field set to 2.0.0
    When I run 'fspec validate-foundation-schema'
    Then the command should use generic-foundation-validator
    And the validation should pass
