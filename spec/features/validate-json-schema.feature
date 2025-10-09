@phase7
@validator
@validation
@json-schema
@internal
@critical
@unit-test
Feature: Internal JSON Schema Validation
  """
  Architecture notes:
  - INTERNAL UTILITY - NOT a user-facing CLI command
  - Uses Ajv (Another JSON Validator) for JSON Schema validation
  - Schemas are stored in src/schemas/ (bundled with the application)
  - Validates foundation.json against src/schemas/foundation.schema.json
  - Validates tags.json against src/schemas/tags.schema.json
  - Called automatically by:
  * generate-foundation command (before generating FOUNDATION.md)
  * generate-tags command (before generating TAGS.md)
  * Any command that modifies foundation.json or tags.json
  - Provides detailed validation errors with JSON paths for debugging
  - Throws errors if validation fails (prevents corrupt JSON files)

  Critical implementation requirements:
  - MUST use Ajv v8+ with full JSON Schema Draft 7 support
  - MUST use ajv-formats for uri, date-time validation
  - MUST validate $schema reference matches actual schema
  - MUST provide JSON path for each validation error
  - MUST throw descriptive errors when validation fails
  - MUST be called BEFORE writing any JSON file
  - MUST be called BEFORE generating any MD file from JSON

  References:
  - JSON Schema: https://json-schema.org/
  - Ajv: https://ajv.js.org/
  """

  Background: User Story
    As an internal validation system
    I want to validate JSON files against schemas automatically
    So that foundation.json and tags.json are always structurally correct

  Scenario: Validate valid foundation.json
    Given I have a foundation.json object with valid structure
    When the validation utility validates it against foundation.schema.json
    Then the validation should pass
    And no errors should be returned

  Scenario: Detect missing required field in foundation.json
    Given I have a foundation.json object missing required field "project.name"
    When the validation utility validates it against foundation.schema.json
    Then the validation should fail
    And the error should indicate "/project" path
    And the error should mention "must have required property 'name'"

  Scenario: Validate valid tags.json
    Given I have a tags.json object with valid structure
    When the validation utility validates it against tags.schema.json
    Then the validation should pass
    And no errors should be returned

  Scenario: Detect invalid tag name format
    Given I have a tags.json object
    And it contains a tag "phase1" without @ prefix
    When the validation utility validates it against tags.schema.json
    Then the validation should fail
    And the error should indicate the tag name field path
    And the error should mention pattern requirement "^@[a-z0-9-]+$"

  Scenario: Validate from file path
    Given I have a file "spec/foundation.json" with valid JSON
    When the validation utility reads and validates the file
    Then the validation should pass
    And the JSON data should be returned

  Scenario: Handle malformed JSON file
    Given I have a file "spec/foundation.json" with invalid JSON syntax
    When the validation utility tries to read the file
    Then it should throw a SyntaxError
    And the error message should indicate JSON parsing failure

  Scenario: Format validation errors for display
    Given I have multiple validation errors from Ajv
    When I format the errors for display
    Then each error should show the JSON path
    And each error should show the validation rule violated
    And each error should be human-readable
