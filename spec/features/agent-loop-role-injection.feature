@done
@session-management
@RPC-082
@rust
@agent-loop
@rpc
@role
@bug120
Feature: Agent loop injects session role as the system prompt every turn
  """
  RPC-082 (child of RPC-072 family). BUG-120 parity: session.get_role()
  must be read per turn and passed as preamble to create_rig_agent so
  the SystemPromptFacade installs it as the system prompt.

  Architecture notes:
  - Structural test approach mirrors the pattern used by
  copilot_create_rig_agent_signature_matches_dispatch_macro_contract
  in rust/agent-loop/src/dispatch.rs:198-214.
  - Every provider (Claude/OpenAI/Gemini/ZAI/Codex/Copilot/CustomProvider)
  exposes create_rig_agent(session_id, preamble: Option<&str>, thinking).
  - Out of scope: live behavioral test through /role slash command —
  that path is covered by RPC-063 (done). RPC-082 only verifies the
  agent loop reads the stored role.

  Parity reference: rust/napi/src/agent_loop.rs:91-96.
  """

  Background: 
    Given the BUG-120 fix requires session.get_role() to be read every turn
    And the canonical agent loop passes the result as preamble to provider.create_rig_agent

  Scenario: BackgroundSession round-trips set_role / get_role / clear_role
    Given a fresh BackgroundSession
    When get_role is called
    Then it returns None
    When set_role is called with "You are a pirate"
    And get_role is called
    Then it returns Some("You are a pirate")
    When clear_role is called
    And get_role is called
    Then it returns None

  Scenario: run_with_provider! macro reads session.get_role() and passes it to create_rig_agent
    Given the source file rust/agent-loop/src/dispatch.rs
    When the macro body of run_with_provider! is extracted
    Then it contains the expression "session.get_role()"
    And it binds the result to a "role_preamble" local
    And it passes "role_preamble.as_deref()" as the second positional argument to "provider.create_rig_agent"

  Scenario: OpenAI inlined arm reads session.get_role() and passes it to create_rig_agent
    Given the source file rust/agent-loop/src/agent_loop.rs
    When the inlined "openai" match arm is extracted
    Then it contains "let role_preamble = session.get_role();"
    And the subsequent provider.create_rig_agent call uses "role_preamble.as_deref()" as the second argument

  Scenario: Custom-provider fallthrough arm reads session.get_role() and passes it to CustomProvider::create_rig_agent
    Given the source file rust/agent-loop/src/agent_loop.rs
    When the "_" fallthrough match arm is extracted
    Then it contains "let role_preamble = session.get_role();"
    And the subsequent CustomProvider::create_rig_agent call uses "role_preamble.as_deref()" as the fifth positional argument

  Scenario: Every dispatched provider type accepts an Option<&str> preamble in create_rig_agent
    Given the seven dispatched provider arms (claude, openai, gemini, zai, codex, github-copilot, copilot) plus the custom-provider fallthrough
    Then each provider type exposes create_rig_agent(session_id, preamble: Option<&str>, thinking)
    And a compile-time closure assertion against the signature succeeds for each provider
