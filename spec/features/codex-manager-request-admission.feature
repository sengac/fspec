@providers
@context-management
@phase-1
@bug
Feature: Codex manager construction preserves catalog-backed request admission
  """
  ProviderManager owns Codex catalog resolution. Codex selections, including profiles,
  use the OpenAI source catalog without changing their Codex routing identity.
  Catalog metadata remains separate from explicit per-model overrides.
  Live Codex providers inherit resolved admission limits without guessed defaults.
  Production-construction regressions redirect only the network client, never
  configure admission on the provider or completion model themselves.
  """

  Background: User Story
    As a user selecting a Codex model
    I want catalog-backed limits applied to the live provider
    So that fitting requests run and unadmitted requests never reach the network

  Scenario: Unprofiled Codex selections resolve and refresh source catalog limits
    Given an OpenAI catalog with two models and no Codex catalog
    When I select each model directly as Codex without a profile
    Then the selected Codex identity is retained and each model uses its own catalog limits

  Scenario: Explicit overrides remain separate from catalog limits
    Given a catalogued Codex model with an explicit context override
    When I resolve limits and subsequently select that model without overrides
    Then the override wins only for its field and the later selection restores catalog limits

  Scenario: Profile selections inherit catalog limits for fields without overrides
    Given a profile model whose identifier also exists in the OpenAI catalog
    When I select it with an explicit profile context limit and automatic output
    Then the context override wins and output uses the matching catalog limit

  Scenario: Unknown selections clear previously known limits
    Given a Codex selection with known catalog limits
    When I select an uncatalogued model without overrides
    Then both limits remain unknown instead of retaining previous or guessed values

  Scenario: A manager-built Codex provider sends a fitting completion
    Given a Codex provider built by the manager from catalog limits
    When I request a fitting nonstreaming completion without manually configuring admission
    Then one physical request succeeds

  Scenario: A manager-built Codex provider sends a fitting stream
    Given a Codex provider built by the manager from catalog limits
    When I request a fitting stream without manually configuring admission
    Then one physical request yields text and a successful final response

  Scenario: A manager-built Codex provider blocks missing and oversized admission
    Given manager-built Codex providers with missing or insufficient selected limits
    When I request both nonstreaming and streaming completions
    Then each request returns its typed admission failure without a physical send
