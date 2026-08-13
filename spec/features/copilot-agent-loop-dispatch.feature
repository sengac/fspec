@done
@authentication
@providers
@PROV-057
Feature: Copilot agent loop dispatch in run_with_provider macro
  """
  PROV-057 L3 (agent-loop half): The run_with_provider! macro in
  rust/napi/src/session_manager.rs must have a 'github-copilot' |
  'copilot' arm so the agent loop dispatches to CopilotProvider instead
  of falling through to the 'Unsupported provider' default. The arm
  constructs a CopilotProvider via provider_manager.get_github_copilot()
  and returns a rig_agent stream — see
  rust/providers/src/copilot/rig_agent.rs.
  """

  Background: User Story
    As a fspec user
    I want the agent loop to route github-copilot sessions to CopilotProvider
    So that chat messages actually stream a response instead of returning "Unsupported provider"

  @copilot
  @agent-loop
  @dispatch
  Scenario: Agent loop dispatches github-copilot to CopilotProvider
    Given a session has selected a "github-copilot/gpt-4o" model
    And valid Copilot credentials exist on disk
    When the agent loop processes a chat message
    Then the run_with_provider macro matches the "github-copilot" arm
    And it constructs a CopilotProvider via provider_manager.get_github_copilot()
    And the response stream completes without an "Unsupported provider" error
