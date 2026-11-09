@done
@REFAC-031
@providers
@refactor
@code-quality
Feature: OpenAI provider split into small modules
  """
  rust/providers/src/openai.rs keeps the module docs, the limit and anti-ramble
  constants, normalize_base_url, the OpenAIProvider struct, its Debug and
  ProviderAdapter impls. Its inherent and trait impls move verbatim into child
  modules: openai/construction.rs (constructors, with_max_output_tokens, base_url,
  is_local_endpoint), openai/agent.rs (model listing, client, create_rig_agent),
  openai/llm_provider.rs (ModelLimitsResolver, rig_response_to_completion,
  LlmProvider) and openai/tests.rs (unit tests). Public paths under
  codelet_providers::openai are unchanged. Coverage mappings into the moved
  lines are re-pointed.
  """

  Background: User Story
    As an fspec developer
    I want the OpenAI provider split into small files
    So that PROV-171 changes to its environment limit overrides touch files under 300 lines

  Scenario: OpenAI provider files stay under 300 lines
    Given the OpenAI provider source and every module split from it in rust/providers
    When the line count of each file is measured
    Then no file has 300 or more lines

  Scenario: The OpenAI provider surface is unchanged
    Given code that names OpenAIProvider through codelet_providers::openai
    When a provider is built for a local base URL with streaming disabled
    Then it reports that base URL, a local endpoint, streaming disabled and the openai provider name
