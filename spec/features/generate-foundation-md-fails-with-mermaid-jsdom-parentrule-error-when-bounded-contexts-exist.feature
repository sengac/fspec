@BUG-123
Feature: generate-foundation-md fails with mermaid JSDOM parentRule error when bounded contexts exist
  """
  Redefines CSSStyleDeclaration.prototype.parentRule with getter+setter before mermaid import
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. JSDOM's CSSStyleDeclaration.parentRule has only a getter — mermaid.render() tries to set it, causing a crash
  #   2. The fix must redefine CSSStyleDeclaration.prototype.parentRule with a getter+setter before importing mermaid
  #   3. mermaid.render() must remain the validation approach — no fallback to parse()-only
  #
  # EXAMPLES:
  #   1. generate-foundation-md succeeds with bounded contexts after the fix
  #   2. add-diagram command succeeds with valid mermaid syntax after the fix
  #   3. Invalid mermaid diagrams are still correctly rejected after the fix
  #
  # ========================================
  Background: User Story
    As a developer
    I want to generate foundation markdown when bounded contexts exist
    So that document my project's domain architecture

  Scenario: Mermaid validation succeeds with valid diagram after parentRule fix
    Given a valid mermaid flowchart diagram
    When I validate the diagram using validateMermaidSyntax
    Then the validation result should be valid

  Scenario: Mermaid validation still rejects invalid diagrams after parentRule fix
    Given an invalid mermaid diagram with syntax errors
    When I validate the diagram using validateMermaidSyntax
    Then the validation result should not be valid
    Then the error message should describe the syntax problem

  Scenario: Bounded context mermaid diagram renders successfully
    Given a mermaid diagram with bounded context subgraphs
    When I validate the diagram using validateMermaidSyntax
    Then the validation result should be valid
