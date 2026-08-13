@BUG-120
Feature: Role injection into LLM system prompt
  """
  Fix is in run_with_provider! macro: read session.get_role() and pass it as the preamble parameter
  to create_rig_agent(). Each provider handles preamble differently:
  - Claude: SystemPromptFacade prepends Claude Code prefix + fspec guidance + role
  - Gemini: build_gemini_system_prompt() appends role as Project-Specific Instructions
  - OpenAI/ZAI: prepend_fspec_guidance() prepends fspec guidance + role
  - Codex: role is appended after CODEX_BASE_INSTRUCTIONS (base instructions always preserved)
  Single change in rust/napi/src/session_manager.rs run_with_provider! macro reads the role.
  Provider-specific handling ensures role integrates cleanly with each provider's system prompt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a role is set on a session, it must be injected into the LLM conversation as the system prompt preamble
  #   2. The run_with_provider! macro reads session.get_role() and passes it to create_rig_agent as preamble
  #   3. Each provider incorporates the preamble into its system prompt using its own facade/builder
  #      (Claude via SystemPromptFacade, Gemini via build_gemini_system_prompt, OpenAI/ZAI via
  #       prepend_fspec_guidance, Codex by appending to CODEX_BASE_INSTRUCTIONS)
  #   4. When a role is cleared, the preamble reverts to None (default facade-only behavior)
  #   5. When spawn is called with a role parameter, the role is set on the subordinate before its first agent loop turn
  #
  # EXAMPLES:
  #   1. User sets role 'You are a security reviewer' → on next turn, create_rig_agent receives role as preamble → LLM system prompt includes role text
  #   2. Agent calls set_role with role='code-reviewer' on subordinate → subordinate's next agent turn uses role as preamble
  #   3. User changes role from 'architect' to 'tester' → next turn uses 'tester' as preamble
  #   4. User clears role → create_rig_agent receives None as preamble → LLM uses default system prompt
  #   5. Supervisor spawns subordinate with role='test-writer' → subordinate's first agent turn uses role as preamble
  #
  # ========================================
  Background: User Story
    As a developer
    I want to have my /role and set_role role text actually affect the LLM's behavior
    So that roles I set on sessions work as intended as system prompt overlays

  @unit
  Scenario: Role is passed as preamble to create_rig_agent
    Given a session with role set to "You are a security reviewer"
    When the agent loop creates a new agent for a turn via run_with_provider
    Then create_rig_agent receives the role text as the preamble parameter
    And the system prompt includes "You are a security reviewer"

  @unit
  Scenario: No role results in None preamble
    Given a session with no role set
    When the agent loop creates a new agent for a turn via run_with_provider
    Then create_rig_agent receives None as the preamble parameter
    And the system prompt contains only facade defaults

  @unit
  Scenario: Cleared role reverts to None preamble
    Given a session with role set to "architect"
    When the role is cleared via clear_role
    And the agent loop creates a new agent for the next turn
    Then create_rig_agent receives None as the preamble parameter

  @unit
  Scenario: Role change takes effect on next turn
    Given a session with role set to "architect"
    When the role is changed to "tester"
    And the agent loop creates a new agent for the next turn
    Then create_rig_agent receives "tester" as the preamble parameter
    And the system prompt includes "tester"

  @unit
  Scenario: Spawned subordinate with role has preamble set on first turn
    Given a supervisor session
    When the supervisor spawns a subordinate with role "test-writer"
    Then the subordinate session has role "test-writer" stored
    And the subordinate's first agent turn passes "test-writer" as preamble to create_rig_agent
