@BUG-582
@BUG-561
@done
@CMPCT-045
@CMPCT-046
@REFAC-034
@BUG-345
@agent-core
@tools
@context-management
Feature: GenerateCompaction tool wiring into provider agents and the agent loop
  """
  Split from spec/features/generate-compaction-tool.feature (REFAC-034); that feature keeps the
  CMPCT-045 example map. Rule 13: GenerateCompactionTool::new(session_id) is wired into every
  provider's create_rig_agent() tool chain (claude, openai, gemini, codex, zai, github-copilot,
  custom provider) beside DeepSearchTool, and the per-session handler is registered with the
  session's other service handlers each turn and removed in the turn's cleanup. Rule 22
  (CMPCT-046c): the compactor sub-agent's built-in provider match supports the same arms as the
  DeepSearch sub-agent's match.
  BUG-345: REFAC-031 moved OpenAI's create_rig_agent to rust/providers/src/openai/agent.rs; the
  builders scenario reads each provider's agent-builder file (Codex: codex/agent.rs, Copilot:
  copilot/rig_agent.rs, custom: custom/custom_provider.rs, the rest their provider module).
  """

  Background: User Story
    As a AI agent (or any session running in codelet)
    I want GenerateCompaction available in every provider's agent and registered for my session
    So that I can compact a session on demand whichever provider the session uses

  @context-management
  @session
  @regression
  @agent-core
  @providers
  @source-shape
  Scenario: Compactor sub-agent provider arms match the DeepSearch clone
    Given the compactor sub-agent's built-in provider match exists
    When it is compared with the DeepSearch sub-agent's provider match
    Then it supports the same arms: claude, openai, gemini, codex, zai, and github-copilot/copilot
    And the copilot arm returns the same distinct pending-builder error shape as DeepSearch instead of falling through to "Unsupported provider"

  @tools
  @providers
  @integration
  Scenario: GenerateCompaction tool wired into provider agent builders
    Given all provider implementations exist (Claude, OpenAI, Gemini, Codex, ZAI, GitHub Copilot, custom provider)
    When each provider's create_rig_agent() method builds an agent
    Then GenerateCompactionTool::new(session_id) is included in the tool chain beside DeepSearchTool
    And the parent agent can invoke GenerateCompaction like any other tool

  @agent-core
  @context-management
  @integration
  Scenario: Handler registration lifecycle mirrors the DeepSearch pattern
    Given a background session is created in the agent loop
    When the session's handlers are registered
    Then a GenerateCompaction handler is registered for the session id capturing the provider, model, project path, and owning SessionManager
    And the handler is re-registered after model or provider changes
    And the handler is removed for the session id during end-of-turn cleanup
