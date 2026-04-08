@done
@authentication
@providers
@PROV-057
Feature: Copilot TUI launches OAuth login when credentials missing
  """
  PROV-057 TUI integration point: AgentView.tsx handleModelSelect must
  check whether the selected provider is github-copilot and credentials
  are missing, then dispatch startCopilotLogin from
  src/tui/utils/copilotLoginFlow.ts instead of showing a generic
  "requires credentials" error. The routing helper lives in
  src/tui/utils/copilotLoginDispatch.ts.
  """

  Background: User Story
    As a fspec user
    I want the TUI to launch the Copilot OAuth login flow when I pick a Copilot model with no credentials
    So that I can log in from the model picker instead of hitting a dead-end error message

  @copilot
  @tui
  Scenario: TUI launches Copilot OAuth login when user picks Copilot model with no credentials
    Given no copilot_auth.json exists on disk
    When the user selects a github-copilot model from the model picker
    Then the TUI dispatches startCopilotLogin from copilotLoginFlow.ts
    And the TUI does NOT display "Failed to switch model: ... requires credentials"
