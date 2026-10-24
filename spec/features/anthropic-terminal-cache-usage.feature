@agent-core
@context-management
@phase-1
@compaction
@bug
@high
@CMPCT-066
Feature: Preserve Anthropic cache usage through terminal streaming reconstruction
  """
  Implementation:
  - P1; CB-15. Read attached cmpct-066-anthropic-terminal-cache.md and 00-shared-contract.md. Evidence classification and exact source anchors are in the specific brief. No runtime reproduction claimed. No native/flat/hybrid substitution, forced salvage, silent replay repair or transport downgrade is permitted.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Retain the agent-owned hierarchical DAG and immutable originals. Failure preserves the last committed generation, returns typed failure, blocks unsafe inference and emits no completion; no memory, replay, transport, request-mode or admission fallback. Follow the attached shared contract.
  #   2. Merge compatible MessageStart/Delta usage within the same physical request; terminal Usage retains measured cache-read/creation detail and correct output.
  #   3. Preserve absent versus measured-zero semantics and normalized inclusive occupancy at live, final, hook, spend and persisted boundaries.
  #   4. A malformed/incomplete terminal usage outcome is explicit; do not fabricate zeros or borrow detail from another request to make compaction appear safe.
  #
  # EXAMPLES:
  #   1. Given nonzero MessageStart cache values and terminal Delta without cache fields, when the finish hook runs, then measured cache occupancy is still present.
  #   2. Given adjacent cached and uncached requests, when terminal usage is reconstructed, then the second request cannot inherit the first request's cache details.
  #
  # ========================================
  Background: User Story
    As an agent operator
    I want measured cache usage preserved through completion
    So that context admission and spend do not lose request evidence

  Scenario: Terminal cache measurements survive the stream
    Given an Anthropic stream starts with fresh input 600 cache read 400 and cache creation 50
    When the terminal delta reports output 100 without repeating cache fields
    Then live and final usage retain cache read 400 and cache creation 50
    And the finish hook records occupancy 1150

  Scenario: Adjacent requests do not share cache evidence
    Given a completed cached Anthropic request
    When the next request omits cache measurements
    Then the next final usage preserves absent cache fields without borrowing prior measurements

  Scenario: Measured zero cache remains distinct from absent cache
    Given an Anthropic request explicitly reports zero cache read and creation
    When its terminal delta completes the request
    Then the final usage retains measured zero cache fields

  Scenario: Terminal usage without a request start is rejected
    Given an Anthropic stream has no measured message start usage
    When a terminal delta reports output without input measurements
    Then the adapter reports incomplete usage instead of a measured zero input

  Scenario: Overflowing cache occupancy is rejected without a successful terminal response
    Given an Anthropic stream reports cache usage that overflows total occupancy
    When the terminal delta completes the request
    Then the adapter reports invalid usage without publishing a successful terminal response

  Scenario: Nonstreaming cache usage agrees with terminal streaming measurements
    Given an Anthropic nonstreaming response reports fresh input 600 cache read 400 cache creation 50 and output 100
    When the real adapter normalizes the completion and its raw usage
    Then both usages retain occupancy 1150 with the same disjoint cache categories
