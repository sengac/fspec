@done
@PROV-140
@rust
@providers
@provider-settings
Feature: OpenAI provider honors the per-profile streaming flag

  """
  OpenAIProvider (codelet/providers/src/openai.rs) gains a streaming: bool
  field sourced from the OPENAI_STREAMING environment variable in
  from_api_key_with_options (defaulting to true when the var is unset or not
  "false"). supports_streaming() returns that field instead of a hardcoded
  true. This is the predicate the agent runner branches on to select the
  streaming vs non-streaming request path.
  """

  Background: User Story
    As a fspec user who disabled streaming on an OpenAI profile
    I want the provider to know streaming is off
    So that the runtime can choose the non-streaming request path

  Scenario: Provider reports streaming disabled when the env flag is false
    Given the OPENAI_STREAMING environment variable is set to false
    When an OpenAI provider is constructed from an api key
    Then supports_streaming returns false

  Scenario: Provider defaults to streaming enabled when the env flag is unset
    Given the OPENAI_STREAMING environment variable is not set
    When an OpenAI provider is constructed from an api key
    Then supports_streaming returns true
