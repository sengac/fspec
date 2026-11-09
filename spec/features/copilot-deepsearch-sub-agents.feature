@PROV-163
@agent-core
@provider-abstraction
Feature: DeepSearch sub-agents run on GitHub Copilot
  """
  rust/agent-loop/src/deep_search_handler.rs used to return "github-copilot DeepSearch agent builder pending" for Copilot
  sessions because CopilotProvider::client did not exist yet. It now builds the sub-agent with
  auxiliary_agent_builder!'s default arm (provider.client().agent(provider.model())), like the other providers, so the
  Copilot sub-agent shares the HTTP middleware and bearer-token refresh. Research:
  spec/attachments/PROV-163/fspec-copilot-comparison.md.
  """

  Background: User Story
    As an fspec user working with GitHub Copilot
    I want DeepSearch to work on Copilot
    So that research sub-agents use the provider my session runs on

  Scenario: A Copilot session's DeepSearch query is answered by a Copilot sub-agent
    Given a github-copilot provider backed by a local Copilot endpoint that answers "found it"
    When DeepSearch runs a query for that provider
    Then the sub-agent's answer "found it" is returned
    And the Copilot endpoint received the DeepSearch request
