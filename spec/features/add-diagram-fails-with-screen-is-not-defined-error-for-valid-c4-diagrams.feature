@bug-fix
@high
@validation
@mermaid
@diagram
@cli
@BUG-082
Feature: add-diagram fails with 'screen is not defined' error for valid C4 diagrams

  """
  Fix Mermaid C4 diagram validation to support C4Context syntax. The error occurs because Mermaid's render() doesn't recognize C4 diagram types without proper initialization. Solution is to skip validation for C4 diagrams or use C4-aware validation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. C4Context diagram syntax must be recognized as valid Mermaid
  #   2. Mermaid validation must support C4 diagram types (C4Context, C4Container, C4Component, C4Dynamic, C4Deployment)
  #   3. C4 diagrams with valid syntax must not throw 'screen is not defined' error
  #
  # EXAMPLES:
  #   1. Add C4Context diagram with Person, System, and Relationships - should succeed without errors
  #   2. Add C4Context diagram similar to reported issue example - validates successfully
  #   3. Add simple C4Context with one Person and one System - validates and adds to foundation.json
  #
  # ========================================

  Background: User Story
    As a developer adding C4 diagrams to foundation
    I want to add valid C4Context diagrams without errors
    So that I can document system architecture using C4 notation

  Scenario: Add C4Context diagram with complex example
    Given I have a project with foundation.json
    When I run add-diagram with a C4Context diagram containing Person, System, System_Ext, and Rel elements
    Then the command should exit with code 0
    And the diagram should be added to foundation.json
    And no "screen is not defined" error should occur


  Scenario: Add C4Context diagram from reported issue
    Given I have a project with foundation.json
    When I run add-diagram with the exact C4Context code from GitHub issue #5
    Then the command should exit with code 0
    And the diagram should be added to foundation.json
    And the Mermaid validation should not throw an error


  Scenario: Add simple C4Context diagram
    Given I have a project with foundation.json
    When I run add-diagram with a simple C4Context diagram with one Person and one System
    Then the command should exit with code 0
    And the diagram should be added to foundation.json architectureDiagrams array
    And foundation.json should validate against the schema

