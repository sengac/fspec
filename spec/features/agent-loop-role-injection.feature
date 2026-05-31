@wip
@deferred
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

  Originally scenario "Session role is injected as the system prompt
  every turn" from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want the /role I set to actually be applied to the LLM call
    So that the assistant adopts the persona I configured

  Scenario: Session role is injected as the system prompt every turn
    Given a Work Agent session backed by a stub provider that echoes its received preamble
    And the user runs the slash command "/role You are a pirate"
    When the user sends "hello"
    Then the stub provider observed a non-empty preamble argument to create_rig_agent
    And the preamble contains "You are a pirate"
    And the assistant reply reflects the role
