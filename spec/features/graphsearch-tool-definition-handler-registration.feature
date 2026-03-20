@KGRAPH-003
Feature: GraphSearch Tool Definition & Handler Registration

  """
  Tool definition in codelet-tools, handler in codelet-napi. Follows SessionSearch pattern. Serde-tagged enum with 8 action types. Handler registered per-session, cleaned up on session end. codelet-tools has no nanograph dependency.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. GraphSearch tool follows SessionSearch pattern: definition in codelet-tools, handler map in codelet-tools (lazy global HashMap), concrete handler in codelet-napi, registration at session start
  #   2. Tool must support 8 action types: search, neighbors, path, related, decisions, history, stats, index
  #   3. Tool args use serde-tagged enum (action_type discriminator) with per-action optional params, matching SessionSearch pattern
  #   4. Concrete handler in codelet-napi delegates to graph module functions — tool definition crate (codelet-tools) has no nanograph dependency
  #   5. Handler registered per-session at session start, cleaned up after agent loop completes (same as SessionSearch lifecycle)
  #   6. Stats and search actions work on empty graph — return empty results, not errors
  #   7. Tool added to agent builder alongside other tools (ReadTool, WriteTool, SessionSearchTool, etc.) in provider create_rig_agent
  #
  # EXAMPLES:
  #   1. Agent calls GraphSearch with action_type 'stats' — gets JSON with node/edge counts, no errors on empty graph
  #   2. Agent calls GraphSearch with action_type 'search' and query 'authentication' — gets matching concepts from graph
  #   3. GraphSearch tool appears in tool listing when agent starts a session — available immediately for LLM to call
  #   4. Calling GraphSearch without a registered handler returns descriptive error — not a panic or crash
  #
  # ========================================

  Background: User Story
    As an agent developer
    I want to query the knowledge graph from any agent session using a GraphSearch tool
    So that I can explore concepts, decisions, relationships, and session history stored in the graph database

  Scenario: Stats action returns JSON on empty graph
    Given the GraphSearch handler is registered for a session
    When the agent calls GraphSearch with action_type 'stats'
    Then the result contains JSON with node and edge type counts all at zero
    And the graph database is empty
    And no error is returned


  Scenario: Search action returns matching concepts
    Given the GraphSearch handler is registered for a session
    When the agent calls GraphSearch with action_type 'search' and query 'authentication'
    Then the result contains a JSON array of matching Concept nodes
    And each result includes the concept name, category, and summary


  Scenario: Tool is available when agent session starts
    Given an agent session is started
    When the tool definitions are listed
    Then GraphSearch appears in the list with its JSON schema
    And the schema describes all 8 action types with their parameters


  Scenario: Unregistered handler returns descriptive error
    Given no GraphSearch handler is registered for the current session
    When the agent calls GraphSearch with any action
    Then the result is a descriptive error message indicating the handler is not available
    And no panic or crash occurs

