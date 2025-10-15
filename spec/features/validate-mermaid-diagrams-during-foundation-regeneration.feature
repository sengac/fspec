@done
@DOC-004
@phase7
@cli
@generator
@foundation-management
@validation
@mermaid
@critical
Feature: Validate Mermaid Diagrams During Foundation Regeneration
  """
  Architecture notes:
  - Uses validateMermaidSyntax() function extracted to src/utils/mermaid-validation.ts
  - Validates ALL diagrams in foundation.json.architectureDiagrams array after JSON schema validation
  - Collects all validation errors before failing (doesn't fail fast)
  - Error format includes: array index, diagram title, detailed Mermaid error message
  - Reuses existing JSDOM + mermaid.parse() validation logic from add-diagram command

  Critical implementation requirements:
  - MUST extract validateMermaidSyntax() to src/utils/mermaid-validation.ts
  - MUST validate diagrams AFTER JSON schema validation passes
  - MUST validate ALL diagrams and collect all errors (no fail-fast)
  - MUST include array index, title, and Mermaid error in error output
  - MUST exit with code 1 if any diagram validation fails
  - MUST guide AI to fix foundation.json and regenerate
  - MUST NOT write FOUNDATION.md if any diagram validation fails

  References:
  - Existing validation: src/commands/add-diagram.ts lines 103-163
  - Mermaid validation: https://mermaid.js.org/
  - Generator command: src/commands/generate-foundation-md.ts
  """

  Background: User Story
    As a developer maintaining foundation.json documentation
    I want to validate all Mermaid diagrams when regenerating FOUNDATION.md
    So that I catch syntax errors immediately with precise error locations and can fix them before the markdown file is written

  Scenario: Generate FOUNDATION.md with all valid diagrams
    Given I have a valid file "spec/foundation.json" with 3 Mermaid diagrams
    And all diagrams have valid Mermaid syntax
    When I run `fspec generate-foundation-md`
    Then the command should exit with code 0
    And the file "spec/FOUNDATION.md" should be created
    And the output should display "✓ Generated spec/FOUNDATION.md from spec/foundation.json"

  Scenario: Fail generation with detailed error for single invalid diagram
    Given I have a file "spec/foundation.json" with 3 diagrams
    And the diagram at architectureDiagrams[1] with title "Data Flow" has invalid Mermaid syntax
    When I run `fspec generate-foundation-md`
    Then the command should exit with code 1
    And the output should display "Diagram validation failed" in red
    And the output should show the diagram position "architectureDiagrams[1]"
    And the output should show the diagram title "Data Flow"
    And the output should show the detailed Mermaid error message
    And the file "spec/FOUNDATION.md" should not be modified

  Scenario: Show all validation errors for multiple invalid diagrams
    Given I have a file "spec/foundation.json" with 5 diagrams
    And diagrams at architectureDiagrams[1] and architectureDiagrams[3] have invalid syntax
    When I run `fspec generate-foundation-md`
    Then the command should exit with code 1
    And the output should display "Found 2 invalid diagrams"
    And the output should show errors for BOTH architectureDiagrams[1] and architectureDiagrams[3]
    And each error should include position, title, and Mermaid error message
    And the file "spec/FOUNDATION.md" should not be modified

  Scenario: Error message includes guidance to fix and regenerate
    Given I have a file "spec/foundation.json" with an invalid diagram
    When I run `fspec generate-foundation-md`
    Then the command should exit with code 1
    And the output should display "Fix the diagram(s) in spec/foundation.json"
    And the output should display "Run 'fspec generate-foundation-md' again after fixing"

  Scenario: Successful regeneration after fixing invalid diagrams
    Given I previously failed generation due to invalid diagrams
    And I have now fixed all diagram syntax errors in "spec/foundation.json"
    When I run `fspec generate-foundation-md` again
    Then the command should exit with code 0
    And the file "spec/FOUNDATION.md" should be created
    And the output should display "✓ Generated spec/FOUNDATION.md from spec/foundation.json"
