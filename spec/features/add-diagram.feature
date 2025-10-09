@phase6
@cli
@foundation-management
@modification
@medium
@unit-test
Feature: Add Mermaid Diagram to FOUNDATION.md
  """
  Architecture notes:
  - Reads spec/foundation.json and updates architectureDiagrams array
  - Diagrams are stored as objects with title, mermaidCode, optional description
  - Regenerates FOUNDATION.md from updated foundation.json
  - Supports multiple diagram types (flowchart, sequence, class, etc.)
  - Creates foundation.json if it doesn't exist (with schema validation)
  - Preserves all other foundation.json content

  Critical implementation requirements:
  - MUST load foundation.json (or create from template)
  - MUST accept diagram title
  - MUST accept Mermaid diagram code
  - MUST validate Mermaid syntax using bundled mermaid.parse()
  - MUST reject invalid Mermaid syntax with helpful error messages
  - MUST add diagram to architectureDiagrams array
  - MUST replace existing diagram with same title
  - MUST validate updated JSON against foundation.schema.json
  - MUST write updated foundation.json
  - MUST regenerate FOUNDATION.md using generateFoundationMd()
  - Exit code 0 for success, 1 for errors

  Workflow:
  1. Load spec/foundation.json (or create from template if missing)
  2. Validate diagram title and code (not empty)
  3. Validate Mermaid syntax using mermaid.parse()
  4. Find existing diagram with same title or add new
  5. Update architectureDiagrams array
  6. Validate updated JSON against schema
  7. Write spec/foundation.json
  8. Regenerate spec/FOUNDATION.md from JSON
  9. Display success message

  References:
  - foundation.schema.json: src/schemas/foundation.schema.json
  - generateFoundationMd(): src/generators/foundation-md.ts
  """

  Background: User Story
    As a developer documenting system architecture
    I want to add Mermaid diagrams to FOUNDATION.md
    So that I can visualize system design and data flow

  Scenario: Add new diagram to existing section
    Given I have a FOUNDATION.md with an "Architecture" section
    When I run `fspec add-diagram Architecture "Component Diagram" "graph TD\n  A-->B"`
    Then the command should exit with code 0
    And the "Architecture" section should contain a diagram titled "Component Diagram"
    And the diagram should be in a mermaid code block
    And other content in the section should be preserved

  Scenario: Add diagram to new section
    Given I have a FOUNDATION.md without a "Data Flow" section
    When I run `fspec add-diagram "Data Flow" "User Login Flow" "sequenceDiagram\n  User->>Server: Login"`
    Then the command should exit with code 0
    And a new "Data Flow" section should be created
    And the section should contain the diagram

  Scenario: Replace existing diagram with same title
    Given I have a diagram titled "System Overview" in the "Architecture" section
    When I run `fspec add-diagram Architecture "System Overview" "graph LR\n  New-->Diagram"`
    Then the command should exit with code 0
    And the "System Overview" diagram should be updated
    And the old diagram content should be replaced

  Scenario: Add multiple diagrams to same section
    Given I have a diagram "Diagram 1" in the "Architecture" section
    When I run `fspec add-diagram Architecture "Diagram 2" "graph TD\n  C-->D"`
    Then the command should exit with code 0
    And the "Architecture" section should contain both diagrams
    And both "Diagram 1" and "Diagram 2" should be present

  Scenario: Create FOUNDATION.md if it doesn't exist
    Given I have no FOUNDATION.md file
    When I run `fspec add-diagram Architecture "Initial Diagram" "graph TD\n  Start-->End"`
    Then the command should exit with code 0
    And a FOUNDATION.md file should be created
    And it should contain the "Architecture" section with the diagram

  Scenario: Preserve existing FOUNDATION.md sections
    Given I have a FOUNDATION.md with sections "What We Are Building" and "Why"
    When I run `fspec add-diagram Architecture "Diagram" "graph TD\n  A-->B"`
    Then the command should exit with code 0
    And the "What We Are Building" section should be preserved
    And the "Why" section should be preserved
    And the "Architecture" section should be added

  Scenario: Support different Mermaid diagram types
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Flows "Sequence Diagram" "sequenceDiagram\n  A->>B: Message"`
    Then the command should exit with code 0
    And the diagram should use sequenceDiagram syntax

  Scenario: Add class diagram
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram "Class Structure" "Domain Model" "classDiagram\n  Class01 <|-- Class02"`
    Then the command should exit with code 0
    And the diagram should use classDiagram syntax

  Scenario: Reject empty diagram code
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "Empty" ""`
    Then the command should exit with code 1
    And the output should show "Diagram code cannot be empty"

  Scenario: Reject empty diagram title
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "" "graph TD\n  A-->B"`
    Then the command should exit with code 1
    And the output should show "Diagram title cannot be empty"

  Scenario: Reject empty section name
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram "" "Title" "graph TD\n  A-->B"`
    Then the command should exit with code 1
    And the output should show "Section name cannot be empty"

  Scenario: Format diagram with proper markdown
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "Flow" "graph TD\n  A-->B"`
    Then the command should exit with code 0
    And the diagram should be formatted as:
      """
      ### Flow

      ```mermaid
      graph TD
        A-->B
      ```
      """

  Scenario: Handle multi-line diagram code
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "Complex" "graph TD\n  A-->B\n  B-->C\n  C-->D"`
    Then the command should exit with code 0
    And all diagram lines should be preserved
    And the diagram should be properly indented

  Scenario: JSON-backed workflow - modify JSON and regenerate MD
    Given I have a valid foundation.json file
    When I run `fspec add-diagram "Architecture Diagrams" "New System Diagram" "graph TD\n  A-->B"`
    Then the foundation.json file should be updated with the new diagram
    And the foundation.json should validate against foundation.schema.json
    And the new diagram should be in the architectureDiagrams array
    And the diagram object should have section, title, and mermaidCode fields
    And the diagram section field should be "Architecture Diagrams"
    And FOUNDATION.md should be regenerated from foundation.json
    And FOUNDATION.md should contain the new diagram in a mermaid code block
    And FOUNDATION.md should have the auto-generation warning header

  Scenario: Reject invalid Mermaid syntax
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "Bad Diagram" "graph TD\n  A--"`
    Then the command should exit with code 1
    And the output should show "Invalid Mermaid syntax"
    And the output should show the mermaid parse error details from mermaid.parse()
    And the diagram should not be added to foundation.json

  Scenario: Reject diagram with invalid diagram type
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "Invalid Type" "invalidDiagram\n  A-->B"`
    Then the command should exit with code 1
    And the output should show "Invalid Mermaid syntax"

  Scenario: Accept valid flowchart diagram
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "Valid Flow" "flowchart TD\n  A-->B\n  B-->C"`
    Then the command should exit with code 0
    And the diagram should be added successfully

  Scenario: Accept valid sequence diagram
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "Valid Sequence" "sequenceDiagram\n  Alice->>Bob: Hello\n  Bob-->>Alice: Hi"`
    Then the command should exit with code 0
    And the diagram should be added successfully

  Scenario: Reject malformed graph syntax
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "Malformed" "graph TD\n  A->->B"`
    Then the command should exit with code 1
    And the output should show "Invalid Mermaid syntax"

  Scenario: Provide helpful error message for syntax errors
    Given I have a FOUNDATION.md
    When I run `fspec add-diagram Architecture "Error Test" "graph TD\n  A[Missing bracket"`
    Then the command should exit with code 1
    And the output should show "Invalid Mermaid syntax"
    And the output should contain the actual mermaid parse error message
    And the error message should include line number information if available
