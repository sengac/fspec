@agent-core
@context-management
@compaction
@CMPCT-044
Feature: Terminal overflow recovery for background sessions
  """
  CMPCT-044 (terminal slice): when a background agent-loop turn dies with
  a provider context-overflow error the in-loop cascade could not
  resolve, the agent-loop terminal-error arm routes it through the SAME
  compactor recovery entry point the stream loop uses — the session's
  context is cleared to reminders plus a DAG, the session returns to
  Idle, and the next user message starts a fresh stream on the reduced
  context (the oversized payload is never replayed). This feature also
  carries the shared spawner contracts (handler registration
  lifecycle, the read-only tool surface, and the FRESH/INCREMENTAL
  prompt selection) that the compactor sub-agent must uphold on every
  path.
  """

  Background: User Story
    As a background agent session
    I want my terminal context-overflow errors to route through the compactor recovery
    So that I compact and resume instead of dying when the in-loop recovery cannot resolve the overflow

  @compaction
  @context-management
  @session
  @integration
  Scenario: Background session terminal overflow error compacts via the same entry point
    Given a background agent-loop session whose turn dies with a context-overflow error that no classifier matched
    When the agent-loop terminal-error arm processes the error
    Then the compactor recovery entry point runs for that session
    And the session's context is cleared to reminders plus a DAG
    And the session returns to Idle
    And the next user message starts a fresh stream on the reduced context

  @compaction
  @context-management
  @integration
  Scenario: Handler registration lifecycle mirrors the DeepSearch pattern
    Given a background session is created in the agent loop
    When the session's handlers are registered
    Then a compactor-sub-agent handler is registered for the session id capturing the provider, model, and project path
    And the handler is removed for the session id during end-of-turn cleanup

  @compaction
  @context-management
  @tools
  Scenario: Compactor sub-agent has only the read-only tool surface
    Given a compactor sub-agent is constructed
    When its tool list is built
    Then the sub-agent has exactly the seven read-only tools: Read, Grep, AstGrep, Glob, Ls, Bash, and SessionSearch
    And the sub-agent does NOT have the inject_summary tool
    And the sub-agent does NOT have any Write or Edit tool

  @compaction
  @context-management
  Scenario: Compaction instruction is FRESH when the parent has no DAG
    Given a parent session that contains no existing compaction DAG
    When the compactor sub-agent's task prompt is built
    Then the FRESH compaction instruction is used
    And the task prompt names the parent session id for every SessionSearch call
    And the task prompt instructs the sub-agent to output the complete DAG as its final response instead of calling inject_summary

  @compaction
  @context-management
  @regression
  Scenario: Compaction instruction is INCREMENTAL when the parent already has a DAG
    Given a parent session whose context already contains a compaction DAG ending at turn N
    When the compactor sub-agent's task prompt is built
    Then the INCREMENTAL compaction instruction is used with the existing DAG embedded
    And the task prompt tells the sub-agent to preserve D2 nodes, promote D0 to D1, and only survey turns from N+1 onward
