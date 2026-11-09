@provider-abstraction
@done
@providers
@rust
@PROV-174
Feature: OpenAI output limit must be positive
  """
  ProviderManager::get_openai (rust/providers/src/manager/impl_3.rs) caps the
  OpenAI request with the session's resolved max output tokens (PROV-164).
  OpenAIProvider::with_max_output_tokens takes a NonZeroUsize, so a resolved
  limit of 0 cannot be silently ignored (it used to keep the previous value):
  get_openai fails with a config error naming the zero limit. Tests:
  rust/providers/tests/prov174_openai_positive_output_limit.rs.
  """

  Background: User Story
    As a user who configures model output limits
    I want a zero output limit reported as an error
    So that a request is never sent with a limit I did not set

  Scenario: A zero output limit fails OpenAI provider construction
    Given an OpenAI selection for "gpt-4o" whose resolved max output tokens is 0
    When the session's OpenAI provider is built
    Then building fails with an error naming the zero output limit

  Scenario: A positive output limit is sent with the provider
    Given an OpenAI selection for "gpt-4o" whose resolved max output tokens is 16384
    When the session's OpenAI provider is built
    Then the provider's max output tokens is 16384
