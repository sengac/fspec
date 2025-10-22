@cli
@foundation-management
@modification
@medium
@unit-test
Feature: Delete Architecture Diagram from Foundation
  """
  Architecture notes:
  - Removes Mermaid diagrams from foundation.json
  - Regenerates FOUNDATION.md after deletion
  - Validates diagram exists before deletion
  - Supports section and title-based lookup
  - Maintains foundation.json structure integrity
  - Provides clear error messages for non-existent diagrams

  Critical implementation requirements:
  - MUST load foundation.json
  - MUST find diagram by section and title
  - MUST remove diagram from architectureDiagrams array
  - MUST write updated foundation.json
  - MUST regenerate FOUNDATION.md from JSON
  - MUST validate foundation.json against schema
  - Exit code 0 for success, 1 for errors

  Workflow:
  1. Load spec/foundation.json
  2. Parse Foundation data structure
  3. Find diagram matching section and title
  4. Remove from architectureDiagrams array
  5. Write updated foundation.json
  6. Regenerate FOUNDATION.md from foundation.json
  7. Display success message

  References:
  - Foundation schema: src/schemas/foundation.schema.json
  - Foundation type: src/types/foundation.ts
  - MD generator: src/utils/generate-foundation-md.ts
  """

  Background: User Story
    As a developer maintaining architecture documentation
    I want to delete outdated diagrams from foundation.json
    So that the architecture documentation stays current and accurate

  Scenario: Delete diagram by section and title
    Given I have a foundation.json with a diagram titled "System Context" in section "Architecture Diagrams"
    When I run `fspec delete-diagram "Architecture Diagrams" "System Context"`
    Then the command should exit with code 0
    And the diagram should be removed from foundation.json
    And FOUNDATION.md should be regenerated without the diagram
    And the output should show "✓ Deleted diagram 'System Context' from section 'Architecture Diagrams'"

  Scenario: Delete one of multiple diagrams
    Given I have a foundation.json with 3 diagrams in "Architecture Diagrams" section
    When I run `fspec delete-diagram "Architecture Diagrams" "Diagram 2"`
    Then the command should exit with code 0
    And only the specified diagram should be removed
    And the other 2 diagrams should remain in foundation.json
    And FOUNDATION.md should contain the remaining diagrams

  Scenario: Handle non-existent diagram
    Given I have a foundation.json
    When I run `fspec delete-diagram "Architecture Diagrams" "Non-Existent Diagram"`
    Then the command should exit with code 1
    And the output should show "Error: Diagram 'Non-Existent Diagram' not found in section 'Architecture Diagrams'"
    And foundation.json should remain unchanged

  Scenario: Handle non-existent section
    Given I have a foundation.json
    When I run `fspec delete-diagram "Invalid Section" "Some Diagram"`
    Then the command should exit with code 1
    And the output should show "Error: No diagrams found in section 'Invalid Section'"
    And foundation.json should remain unchanged

  Scenario: Delete last diagram in section
    Given I have a foundation.json with only 1 diagram in "Architecture Diagrams"
    When I run `fspec delete-diagram "Architecture Diagrams" "Last Diagram"`
    Then the command should exit with code 0
    And the architectureDiagrams array should be empty
    And FOUNDATION.md should not contain any diagrams in that section

  Scenario: Preserve other foundation.json sections
    Given I have a foundation.json with project info and diagrams
    When I run `fspec delete-diagram "Architecture Diagrams" "Test Diagram"`
    Then the command should exit with code 0
    And the project section should remain unchanged
    And the whatWeAreBuilding section should remain unchanged
    And only the specified diagram should be removed

  Scenario: JSON-backed workflow - delete from foundation.json
    Given I have foundation.json with multiple architecture diagrams
    When I run `fspec delete-diagram "Architecture Diagrams" "Outdated Diagram"`
    Then the command should load foundation.json
    And remove the specified diagram from the architectureDiagrams array
    And write the updated foundation.json
    And regenerate FOUNDATION.md from the updated JSON
    And the command should exit with code 0
