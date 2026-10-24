@done
@CMPCT-071
@providers
@context-management
@compaction
@phase-1
Feature: Builtin provider output limit serialization
  """
  OpenAI-compatible providers share the core Chat Completions conversion for
  selected streaming preparation, nonstreaming submission and direct completion.
  The configured core output limit must reach the physical max_completion_tokens
  field without callers duplicating it in provider-specific optional parameters.
  An absent limit stays absent and unrelated optional parameters remain intact.
  """

  Background: User Story
    As an agent using a selected OpenAI-compatible provider
    I want my configured output reservation represented in the physical request
    So that the provider cannot generate beyond the limit used for admission

  Scenario: Streaming Chat preserves configured and absent output limits
    Given a selected streaming Chat model with configured or absent core output limits
    When each request is prepared for admission and physically streamed
    Then exact prepared and physical bytes retain the selected output limit and optional parameters
    And an absent limit stays omitted without introducing the legacy output limit field

  Scenario: Nonstreaming Chat preserves configured and absent output limits
    Given a selected nonstreaming Chat model with configured or absent core output limits
    When each request is prepared for admission and physically submitted
    Then exact prepared and physical bytes retain the selected output limit and optional parameters
    And an absent limit stays omitted without introducing the legacy output limit field

  Scenario: Direct Chat completion preserves configured and absent output limits
    Given a selected direct Chat model with configured or absent core output limits
    When each request completes through the physical HTTP adapter
    Then physical requests retain the selected output limit and optional parameters
    And an absent limit stays omitted without introducing the legacy output limit field
