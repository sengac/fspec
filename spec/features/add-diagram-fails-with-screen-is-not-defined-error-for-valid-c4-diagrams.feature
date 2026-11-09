@bug-fix
@high
@validation
@mermaid
@diagram
@cli
@BUG-082
Feature: add-diagram fails with 'screen is not defined' error for valid C4 diagrams
  """
  Valid C4Context diagrams pass add-diagram's Mermaid validation (merman parser, utils/mermaid_validation.rs) and are stored in spec/foundation.json; the FOUNDATION.md regeneration validates and renders them too. The original failure ('screen is not defined') came from the TypeScript client running mermaid render() without a browser.
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
    Then add-diagram succeeds
    And the diagram should be added to foundation.json
    And spec/FOUNDATION.md shows the diagram in a mermaid code block

  Scenario: Add C4Context diagram from reported issue
    Given I have a project with foundation.json
    When I run add-diagram with the exact C4Context code from GitHub issue #5
    Then add-diagram succeeds
    And the diagram should be added to foundation.json
    And the result carries no error

  Scenario: Add simple C4Context diagram
    Given I have a project with foundation.json
    When I run add-diagram with a simple C4Context diagram with one Person and one System
    Then add-diagram succeeds
    And the diagram should be added to foundation.json architectureDiagrams array
    And spec/FOUNDATION.md shows the diagram in a mermaid code block
