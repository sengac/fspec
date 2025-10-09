@phase7
@cli
@foundation-management
@modification
@json-backed
@high
@integration-test
Feature: Add Diagram to JSON-Backed Foundation
  """
  Architecture notes:
  - Updates existing `fspec add-diagram` command to use foundation.json
  - Validates foundation.json schema before and after modification
  - Automatically regenerates FOUNDATION.md after successful addition
  - Maintains backward compatibility with existing command syntax
  - Ensures atomic operation: validate → modify → regenerate

  Critical implementation requirements:
  - MUST read and parse foundation.json
  - MUST validate against foundation.schema.json
  - MUST add/update diagram in architectureDiagrams array
  - MUST validate Mermaid syntax if possible
  - MUST validate updated JSON against schema
  - MUST regenerate FOUNDATION.md automatically
  - MUST handle errors gracefully (rollback if regeneration fails)

  Workflow:
  1. Read foundation.json
  2. Validate current structure
  3. Add or update diagram in architectureDiagrams
  4. Validate modified structure
  5. Write foundation.json
  6. Regenerate FOUNDATION.md
  7. Report success

  References:
  - Original command: spec/features/add-diagram.feature
  - Mermaid: https://mermaid.js.org/
  """

  Background: User Story
    As a developer maintaining architecture documentation
    I want `fspec add-diagram` to edit foundation.json and regenerate FOUNDATION.md
    So that architecture diagrams stay consistent and properly formatted

  Scenario: Add new Mermaid diagram to Architecture Diagrams section
    Given I have a valid file "spec/foundation.json"
    When I run:
      """
      fspec add-diagram "Architecture Diagrams" "New System Flow" "graph TB\n  A[Start]\n  B[End]\n  A-->B"
      """
    Then the command should exit with code 0
    And "spec/foundation.json" should contain the new diagram:
      """json
      {
        "title": "New System Flow",
        "mermaidCode": "graph TB\n  A[Start]\n  B[End]\n  A-->B"
      }
      """
    And "spec/FOUNDATION.md" should be regenerated
    And "spec/FOUNDATION.md" should contain:
      """
      ### New System Flow

      ```mermaid
      graph TB
        A[Start]
        B[End]
        A-->B
      ```
      """
    And the output should display "✓ Added diagram 'New System Flow' to 'Architecture Diagrams'"
    And the output should display "✓ Regenerated spec/FOUNDATION.md"

  Scenario: Update existing diagram with same title
    Given I have a valid file "spec/foundation.json"
    And it contains a diagram titled "fspec System Context"
    When I run:
      """
      fspec add-diagram "Architecture Diagrams" "fspec System Context" "graph TB\n  NEW[Updated]"
      """
    Then the existing diagram should be updated (not duplicated)
    And "spec/foundation.json" should contain only one diagram with that title
    And "spec/FOUNDATION.md" should be regenerated

  Scenario: Add diagram with description
    Given I have a valid file "spec/foundation.json"
    When I run:
      """
      fspec add-diagram "Architecture Diagrams" "Data Flow" "graph LR\n  A-->B" --description="Shows data flow between components"
      """
    Then "spec/foundation.json" should contain:
      """json
      {
        "title": "Data Flow",
        "mermaidCode": "graph LR\n  A-->B",
        "description": "Shows data flow between components"
      }
      """

  Scenario: Read Mermaid code from file
    Given I have a file "diagram.mmd" containing Mermaid code
    When I run:
      """
      fspec add-diagram "Architecture Diagrams" "Complex Diagram" --file=diagram.mmd
      """
    Then the Mermaid code should be read from "diagram.mmd"
    And it should be added to "spec/foundation.json"
    And "spec/FOUNDATION.md" should be regenerated

  Scenario: Validate Mermaid syntax
    Given I have a valid file "spec/foundation.json"
    When I run:
      """
      fspec add-diagram "Architecture Diagrams" "Invalid Diagram" "invalid mermaid syntax"
      """
    Then the command should display a warning "⚠ Mermaid syntax may be invalid"
    But the diagram should still be added (warning only, not error)

  Scenario: Fail if section does not exist
    Given I have a valid file "spec/foundation.json"
    When I run:
      """
      fspec add-diagram "Nonexistent Section" "Diagram" "graph TB"
      """
    Then the command should exit with code 1
    And the output should display "✗ Section 'Nonexistent Section' not found"
    And "spec/foundation.json" should not be modified

  Scenario: Rollback if markdown generation fails
    Given I have a valid file "spec/foundation.json"
    When I run:
      """
      fspec add-diagram "Architecture Diagrams" "New Diagram" "graph TB"
      """
    And markdown generation fails due to template error
    Then the command should exit with code 1
    And "spec/foundation.json" should be rolled back to previous state
    And the output should display "✗ Failed to regenerate FOUNDATION.md - changes rolled back"

  Scenario: Support multiple diagram sections
    Given "spec/foundation.json" has multiple sections that can contain diagrams
    When I run:
      """
      fspec add-diagram "Architecture Diagrams" "Diagram 1" "graph TB"
      """
    Then the diagram should be added to the correct section
    And other sections should not be affected
