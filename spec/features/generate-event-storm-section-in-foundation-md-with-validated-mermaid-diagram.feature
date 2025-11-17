@done
@critical
@foundation-management
@event-storming
@generator
@validation
@cli
@FOUND-017
Feature: Generate Event Storm section in FOUNDATION.md with validated Mermaid diagram

  """
  Use existing validateMermaidSyntax() utility from src/utils/mermaid-validation.ts to validate generated Mermaid diagram. Call validateMermaidSyntax() before writing to FOUNDATION.md. Reuse existing pattern from add-diagram command. Auto-regenerate FOUNDATION.md by calling generateFoundationMd() at end of add-foundation-bounded-context command.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Mermaid diagram syntax must be validated using mermaid.parse() before adding to FOUNDATION.md
  #   2. FOUNDATION.md must auto-regenerate when Event Storm commands are run (add-foundation-bounded-context, etc.)
  #   3. Event Storm section should include both text list AND Mermaid bounded context map
  #   4. Event Storm section only appears if foundation.json has eventStorm field with items
  #   5. Mermaid diagram should show bounded contexts as nodes in a graph
  #
  # EXAMPLES:
  #   1. User runs 'fspec add-foundation-bounded-context Work Management', FOUNDATION.md auto-regenerates with Domain Architecture section showing bounded context in text list and Mermaid diagram
  #   2. foundation.json has 7 bounded contexts, FOUNDATION.md includes Mermaid graph with 7 nodes
  #   3. Invalid Mermaid syntax detected during generation, error thrown with line number and helpful message
  #
  # ========================================

  Background: User Story
    As a AI agent using fspec for any project
    I want to see Event Storm bounded contexts visualized in FOUNDATION.md with a Mermaid diagram
    So that I understand the strategic domain architecture at a glance

  Scenario: FOUNDATION.md auto-regenerates with Event Storm section when bounded context added
    Given foundation.json exists with eventStorm field initialized
    When I run "fspec add-foundation-bounded-context 'Work Management'"
    Then FOUNDATION.md should be automatically regenerated
    And FOUNDATION.md should contain a "Domain Architecture" section
    And the Domain Architecture section should include a "Bounded Contexts" subsection with a text list
    And the text list should include "Work Management"
    And the Domain Architecture section should include a "Bounded Context Map" subsection with a Mermaid diagram
    And the Mermaid diagram should have a node for "Work Management"
    And the Mermaid diagram syntax should be validated using validateMermaidSyntax()

  Scenario: FOUNDATION.md includes Mermaid graph with all bounded contexts
    Given foundation.json has eventStorm field with 7 bounded contexts
    And the bounded contexts are "Work Management", "Specification", "Testing & Validation", "Quality Assurance", "Version Control Integration", "User Interface", and "Foundation Management"
    When FOUNDATION.md is generated
    Then the Mermaid bounded context map should have 7 nodes
    And each node should represent one bounded context
    And the diagram should be validated before being written to FOUNDATION.md

  Scenario: Mermaid validation error prevents FOUNDATION.md generation
    Given foundation.json has eventStorm field with bounded contexts
    And the generated Mermaid diagram contains invalid syntax
    When FOUNDATION.md generation is attempted
    Then an error should be thrown with the validation error message
    And FOUNDATION.md should not be written with invalid Mermaid syntax
    And the error message should help identify the syntax issue

  Scenario: Event Storm section only appears when eventStorm data exists
    Given foundation.json exists without eventStorm field
    When FOUNDATION.md is generated
    Then the Domain Architecture section should not appear in FOUNDATION.md
