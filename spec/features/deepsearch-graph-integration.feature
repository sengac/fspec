@KGRAPH-009
Feature: DeepSearch Graph Integration
  """
  Modifies DeepSearch sub-agent builder in rust/tools to conditionally add GraphSearch tool. Injects graph context into system prompt before spawning sub-agent. All integration is opt-in — zero behavior change when graph is absent.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. GraphSearch tool is added to DeepSearch sub-agent's toolset only when the graph database exists
  #   2. DeepSearch works identically when no graph database is present (backward compatible)
  #   3. When graph exists and has data, related concepts are injected into the DeepSearch system prompt as context
  #
  # EXAMPLES:
  #   1. Graph DB initialized → DeepSearch has 8 tools (7 default + GraphSearch)
  #   2. No graph DB → DeepSearch has 7 default tools, no error
  #   3. Graph has concepts related to query → system prompt includes knowledge graph context section
  #
  # ========================================
  Background: User Story
    As an agent developer
    I want to have DeepSearch sub-agents optionally use the knowledge graph for context-enriched research
    So that research queries are informed by existing concept relationships and decisions

  Scenario: GraphSearch tool added when graph database is initialized
    Given the knowledge graph database is initialized and available
    When a DeepSearch sub-agent is being built
    Then the GraphSearch tool is included in the sub-agent's toolset
    And the sub-agent has 8 tools total

  Scenario: DeepSearch works without graph database
    Given no knowledge graph database exists
    When a DeepSearch sub-agent is being built
    Then the sub-agent has the default 7 tools
    And no error is raised

  Scenario: Graph context injected into system prompt when data exists
    Given the knowledge graph contains concepts related to the search query
    When the DeepSearch system prompt is constructed
    Then a knowledge graph context section is appended to the prompt
    And the context includes related concept names and relationships
