@done
@TEST-009
Feature: Mermaid validation test uses invalid test data
  """
  Uses validateMermaidSyntax() to validate Event Storm diagrams generated from eventStorm.items[]. Currently generate-foundation-md.ts validates architectureDiagrams but NOT Event Storm diagrams. Fix requires adding Event Storm diagram validation using mermaid.parse() before writing FOUNDATION.md. Invalid syntax must throw error with helpful message and line numbers.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. generateFoundationMd must validate Event Storm Mermaid diagrams before writing FOUNDATION.md
  #   2. Invalid Mermaid syntax in Event Storm bounded context names must trigger validation error
  #
  # EXAMPLES:
  #   1. Bounded context with unclosed bracket 'Context[Unclosed' causes validation error
  #   2. generateFoundationMd throws error with helpful message pointing to invalid syntax
  #   3. FOUNDATION.md is not written when Event Storm diagram validation fails
  #
  # ========================================
  Background: User Story
    As a developer writing tests for Mermaid validation
    I want to validate that Event Storm diagrams with invalid Mermaid syntax are rejected
    So that FOUNDATION.md is never generated with broken diagrams

  Scenario: Bounded context with malicious code injection causes validation error
    Given foundation.json has eventStorm field with bounded context 'Context"];malicious[("code'
    When generateFoundationMd is called with this Event Storm data
    Then a validation error should be thrown
    And the error should indicate invalid Mermaid syntax

  Scenario: generateFoundationMd throws error with helpful message pointing to invalid syntax
    Given foundation.json has Event Storm data with invalid Mermaid syntax in bounded context name
    When generateFoundationMd attempts to generate FOUNDATION.md
    Then an error should be thrown
    And the error message should help identify the syntax issue
    And the error message should be descriptive enough for developers to fix the problem

  Scenario: FOUNDATION.md is not written when Event Storm diagram validation fails
    Given foundation.json has eventStorm field with invalid bounded context 'Context"];malicious[("code'
    When generateFoundationMd is called
    Then an error should be thrown before FOUNDATION.md is written
    And FOUNDATION.md should not exist or should remain unchanged
    And no file with broken Mermaid diagrams should be created
