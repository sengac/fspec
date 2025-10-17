@attachments
@mermaid
@file-ops
@validation
@phase2
@FEAT-015
Feature: Validate Mermaid diagrams in attachments
  """
  Uses existing mermaid-validation.ts utility (same as add-diagram command). Integration point: add-attachment command at file validation stage. Must detect file extensions (.mmd, .mermaid, .md) and extract/validate Mermaid syntax before copying to attachments directory. Error messages follow existing format pattern from add-diagram but adapted for attachment context.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Mermaid diagram attachments must be validated using the same mermaid.parse() logic as add-diagram command
  #   2. Only files with .mmd or .mermaid extensions should trigger Mermaid validation
  #   3. Invalid Mermaid syntax must prevent attachment with detailed error message including line numbers
  #   4. Valid Mermaid diagrams should be attached successfully with confirmation message
  #   5. Use similar error format to add-diagram but adapted for attachment context (e.g., 'Failed to attach diagram.mmd' instead of 'Failed to add diagram')
  #   6. No --skip-validation flag - all Mermaid diagrams must pass validation before attachment
  #   7. Yes, validate .md files that contain mermaid code blocks (triple-backtick mermaid syntax)
  #
  # EXAMPLES:
  #   1. User attaches valid flowchart.mmd with 'graph TD' syntax - attachment succeeds with confirmation
  #   2. User attaches sequence.mermaid with invalid syntax - attachment fails with detailed error message showing line number
  #   3. User attaches diagram.png - no Mermaid validation performed (not .mmd or .mermaid extension)
  #   4. User attaches erDiagram.mmd with valid ER diagram syntax - attachment succeeds
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we also validate .md files that contain mermaid code blocks?
  #   A: true
  #
  #   Q: Should the error message format match exactly with add-diagram errors for consistency?
  #   A: true
  #
  #   Q: Should we add a --skip-validation flag to allow attaching invalid diagrams for drafts?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec with AI agents
    I want to attach Mermaid diagrams to work units with automatic validation
    So that I catch diagram syntax errors immediately and get helpful feedback to fix them

  Scenario: Attach valid Mermaid diagram with .mmd extension
    Given I have a work unit AUTH-001 in the backlog
    And I have a file flowchart.mmd with valid Mermaid syntax (graph TD)
    When I run 'fspec add-attachment AUTH-001 flowchart.mmd'
    Then the attachment should be added successfully
    And the output should display a success message
    And the file should be copied to spec/attachments/AUTH-001/

  Scenario: Attach invalid Mermaid diagram with syntax errors
    Given I have a work unit AUTH-001 in the backlog
    And I have a file sequence.mermaid with invalid Mermaid syntax
    When I run 'fspec add-attachment AUTH-001 sequence.mermaid'
    Then the attachment should fail
    And the output should display an error message with 'Failed to attach sequence.mermaid'
    And the error message should include the line number of the syntax error
    And the file should NOT be copied to the attachments directory

  Scenario: Attach non-Mermaid file without validation
    Given I have a work unit AUTH-001 in the backlog
    And I have a file diagram.png (image file)
    When I run 'fspec add-attachment AUTH-001 diagram.png'
    Then the attachment should be added successfully without Mermaid validation
    And the file should be copied to spec/attachments/AUTH-001/

  Scenario: Attach markdown file with valid Mermaid code block
    Given I have a work unit AUTH-001 in the backlog
    And I have a file architecture.md containing a mermaid code block with valid syntax
    When I run 'fspec add-attachment AUTH-001 architecture.md'
    Then the mermaid code block should be extracted and validated
    And the attachment should be added successfully
    And the file should be copied to spec/attachments/AUTH-001/
