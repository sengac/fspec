@phase7 @cli @migration @utility @critical @integration-test
Feature: Migrate Existing Markdown to JSON

  """
  Architecture notes:
  - One-time migration command to convert existing FOUNDATION.md → foundation.json
  - One-time migration command to convert existing TAGS.md → tags.json
  - Parses markdown structure and extracts data into JSON format
  - Validates generated JSON against schemas
  - Creates backup of original markdown files
  - Verifies migration by regenerating markdown and comparing

  Critical implementation requirements:
  - MUST parse all sections from FOUNDATION.md correctly
  - MUST extract all tag categories and tags from TAGS.md
  - MUST validate generated JSON against schemas
  - MUST create backups before migration (.md.backup)
  - MUST verify migration by regenerating and comparing
  - MUST handle Mermaid diagrams in FOUNDATION.md
  - MUST preserve all table data and statistics
  - MUST handle special characters and escaping

  Migration verification:
  1. Parse MD → JSON
  2. Validate JSON against schema
  3. Generate MD from JSON
  4. Compare generated MD with original MD
  5. If identical or acceptable differences, migration successful
  6. If major differences, report issues

  References:
  - Markdown parsing: remark/unified ecosystem
  """

  Background: User Story
    As a developer setting up JSON-backed documentation
    I want to migrate existing FOUNDATION.md and TAGS.md to JSON
    So that I can start using structured JSON editing and generation

  Scenario: Migrate FOUNDATION.md to foundation.json
    Given I have an existing file "spec/FOUNDATION.md"
    When I run `fspec migrate-to-json --type=foundation`
    Then the command should exit with code 0
    And a file "spec/foundation.json" should be created
    And it should be valid according to "spec/schemas/foundation.schema.json"
    And a backup "spec/FOUNDATION.md.backup" should be created
    And the output should display "✓ Migrated spec/FOUNDATION.md → spec/foundation.json"
    And the output should display "✓ Backup created: spec/FOUNDATION.md.backup"

  Scenario: Migrate TAGS.md to tags.json
    Given I have an existing file "spec/TAGS.md"
    When I run `fspec migrate-to-json --type=tags`
    Then the command should exit with code 0
    And a file "spec/tags.json" should be created
    And it should be valid according to "spec/schemas/tags.schema.json"
    And a backup "spec/TAGS.md.backup" should be created
    And the output should display "✓ Migrated spec/TAGS.md → spec/tags.json"

  Scenario: Migrate both files at once
    Given I have "spec/FOUNDATION.md" and "spec/TAGS.md"
    When I run `fspec migrate-to-json --all`
    Then both files should be migrated to JSON
    And both JSON files should be valid against their schemas
    And backups should be created for both markdown files

  Scenario: Verify migration by regenerating markdown
    Given I have an existing file "spec/FOUNDATION.md"
    When I run `fspec migrate-to-json --type=foundation --verify`
    Then the command should:
      | step                              |
      | Parse FOUNDATION.md → JSON        |
      | Validate JSON against schema      |
      | Generate MD from JSON             |
      | Compare generated MD with original|
      | Report any differences            |
    And if differences are acceptable, the output should display "✓ Migration verified successfully"

  Scenario: Extract Mermaid diagrams correctly
    Given "spec/FOUNDATION.md" contains Mermaid diagrams:
      """
      ```mermaid
      graph TB
        AI[AI Agent]
        FSPEC[fspec CLI]
      ```
      """
    When I run `fspec migrate-to-json --type=foundation`
    Then "spec/foundation.json" should contain:
      """json
      {
        "architectureDiagrams": [
          {
            "title": "diagram title",
            "mermaidCode": "graph TB\n  AI[AI Agent]\n  FSPEC[fspec CLI]"
          }
        ]
      }
      """

  Scenario: Extract tag categories and tags correctly
    Given "spec/TAGS.md" contains tag categories with tables
    When I run `fspec migrate-to-json --type=tags`
    Then "spec/tags.json" should contain all categories
    And each category should have all tags from the table
    And each tag should have name, description, and category-specific fields
    And tag combination examples should be preserved
    And statistics should be extracted

  Scenario: Fail if markdown file is malformed
    Given "spec/FOUNDATION.md" has malformed structure
    When I run `fspec migrate-to-json --type=foundation`
    Then the command should exit with code 1
    And the output should display parsing errors
    And no JSON file should be created
    And no backup should be created

  Scenario: Fail if JSON already exists (unless --force)
    Given "spec/foundation.json" already exists
    When I run `fspec migrate-to-json --type=foundation`
    Then the command should exit with code 1
    And the output should display "✗ spec/foundation.json already exists. Use --force to overwrite"

  Scenario: Force overwrite existing JSON
    Given "spec/foundation.json" already exists
    When I run `fspec migrate-to-json --type=foundation --force`
    Then the existing JSON should be backed up to "spec/foundation.json.backup"
    And a new "spec/foundation.json" should be created from FOUNDATION.md
    And the output should display "✓ Backed up existing spec/foundation.json"

  Scenario: Report migration statistics
    Given I have "spec/FOUNDATION.md" and "spec/TAGS.md"
    When I run `fspec migrate-to-json --all --verbose`
    Then the output should display:
      | statistic                           |
      | Number of sections migrated         |
      | Number of diagrams extracted        |
      | Number of commands extracted        |
      | Number of tags extracted            |
      | Number of tag categories found      |
      | JSON file sizes                     |
      | Validation results                  |
