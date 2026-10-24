@context-management
@phase-1
@agent-core
@compaction
@bug
@high
@CMPCT-065
Feature: Normalize inclusive usage before additive context accounting
  """
  Implementation:
  - P1; CB-14. Read attached cmpct-065-usage-normalization.md and 00-shared-contract.md. Evidence classification and exact source anchors are in the specific brief. No runtime reproduction claimed. No native/flat/hybrid substitution, forced salvage, silent replay repair or transport downgrade is permitted.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Retain the agent-owned hierarchical DAG and immutable originals. Failure preserves the last committed generation, returns typed failure, blocks unsafe inference and emits no completion; no memory, replay, transport, request-mode or admission fallback. Follow the attached shared contract.
  #   2. Normalize once at declared provider boundaries into coherent inclusive context occupancy and separate categorized spend/detail; cached/reasoning subsets count once.
  #   3. Preserve provider-specific semantics, including Anthropic's distinct input/cache categories. Cost discounts never reduce context occupancy.
  #   4. Unknown/inconsistent usage semantics remain explicit; no invented disjointness, dropped detail or broad rescue estimate presented as authoritative usage.
  #
  # EXAMPLES:
  #   1. Given inclusive input 1000 with cached subset 400 and output 100 with reasoning subset 30, when normalized, then context counts 1100, not 1530.
  #   2. Given Anthropic disjoint input/cache fields, when the same normalized API is used, then all actual input occupancy is retained without subtracting cache twice.
  #
  # ========================================
  Background: User Story
    As an agent operator
    I want measured context occupancy without duplicate cache or reasoning counts
    So that admission uses actual occupancy while billing details remain intact

  Scenario: Inclusive provider subsets count once
    Given an OpenAI-compatible or Codex request reports input 1000 including cached 400 and output 100 including reasoning 30
    When the provider usage reaches context accounting
    Then occupancy is 1100 and measured cache and reasoning detail remains available

  Scenario: Anthropic disjoint categories retain full occupancy
    Given an Anthropic request reports fresh input 600 cache read 400 cache creation 50 and output 100
    When the provider usage reaches context accounting
    Then occupancy is 1150 and all measured cache categories remain intact

  Scenario: Gemini live and final usage preserve the same occupancy
    Given a Gemini request reports prompt 1000 cached 400 candidates 70 thoughts 30 and total 1100
    When live and final provider usage reach context accounting
    Then both report occupancy 1100 and preserve cached 400 and reasoning 30

  Scenario: Inconsistent provider subsets remain unknown
    Given a provider reports cached or reasoning subsets greater than their inclusive parent counts
    When normalization is requested
    Then no authoritative normalized usage is returned and raw detail is not rewritten as a measured zero

  Scenario: Inclusive subset normalization preserves detail across valid counts
    Given valid inclusive usage counts and their cache and reasoning subsets
    When usage is normalized and serialized for transport
    Then occupancy equals inclusive input plus output for every count
    And restored usage preserves measured subset detail
