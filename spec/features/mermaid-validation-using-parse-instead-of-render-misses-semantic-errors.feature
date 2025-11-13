@done
@validator
@medium
@validation
@attachment-management
@mermaid
@bug-fix
@VAL-001
Feature: Mermaid validation using parse() instead of render() misses semantic errors

  """
  Uses mermaid.render() instead of mermaid.parse() for complete validation. Catches both syntax and semantic errors (subgraph format, style constraints, etc.). Requires JSDOM for headless browser environment. Error messages include diagram number and error details for easy debugging.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Mermaid validation must catch ALL errors that would occur during browser rendering
  #   2. Validation must reject semantically invalid diagrams (e.g., quoted subgraph titles)
  #   3. Validation must occur at attachment time (one-time operation)
  #   4. Error messages must clearly indicate what is wrong and how to fix it
  #
  # EXAMPLES:
  #   1. User attaches markdown with quoted subgraph title 'subgraph "Server Side"' - validation REJECTS with clear error
  #   2. User attaches markdown with proper subgraph syntax 'subgraph ServerSide[Server Side]' - validation PASSES
  #   3. User attaches markdown with invalid subgraph identifier 'subgraph INVALID\!\!\!' - validation REJECTS
  #   4. User attaches markdown with broken syntax (missing closing bracket) - validation REJECTS (already working)
  #   5. User attaches markdown with multiple diagrams, one invalid - validation REJECTS with clear indication which diagram failed
  #
  # ========================================

  Background: User Story
    As a developer using fspec to document architecture
    I want to validate mermaid diagrams for both syntax and semantic errors
    So that I catch rendering errors before viewing in browser

  Scenario: Reject markdown with quoted subgraph title
    Given I have a markdown file with a mermaid diagram containing 'subgraph "Server Side"'
    When I run 'fspec add-attachment DOC-001 <file-path>'
    Then the command should fail with exit code 1
    And the error message should indicate the subgraph title format is invalid
    And the error message should suggest using 'subgraph ID[Title]' syntax

  Scenario: Accept markdown with proper subgraph syntax
    Given I have a markdown file with a mermaid diagram containing 'subgraph ServerSide[Server Side]'
    When I run 'fspec add-attachment DOC-001 <file-path>'
    Then the command should succeed
    And the attachment should be added to the work unit

  Scenario: Reject markdown with invalid subgraph identifier
    Given I have a markdown file with a mermaid diagram containing 'subgraph INVALID!!!'
    When I run 'fspec add-attachment DOC-001 <file-path>'
    Then the command should fail with exit code 1
    And the error message should indicate the subgraph identifier is invalid

  Scenario: Reject markdown with broken syntax
    Given I have a markdown file with a mermaid diagram with missing closing bracket
    When I run 'fspec add-attachment DOC-001 <file-path>'
    Then the command should fail with exit code 1
    And the error message should indicate syntax error

  Scenario: Reject markdown with multiple diagrams where one is invalid
    Given I have a markdown file with 3 mermaid diagrams
    And the second diagram contains 'subgraph "Invalid Quotes"'
    When I run 'fspec add-attachment DOC-001 <file-path>'
    Then the command should fail with exit code 1
    And the error message should indicate which diagram failed (diagram 2)
    And the error message should include the specific error for that diagram
