@provider-abstraction
@PROV-150
@providers
@streaming
@phase-1
Feature: Codex completed reasoning sections
  """
  Codex normalizes completed raw content and terminal-only sections into thinking
  deltas. Summary and raw identities remain separate, even at the same index.
  Disabling completed delivery does not disable incremental thinking or answers.
  """

  Scenario: Terminal and raw completed reasoning fills missing delivery
    Given raw reasoning completion and terminal-only summary sections
    When the Codex frontend consumes the response
    Then each completed readable section appears without requiring a delta

  Scenario: Raw and summary reasoning retain distinct identities
    Given raw reasoning and summary text interleaved at the same item index
    When both sections complete with additional Unicode text
    Then only unseen text appears under the original distinct section identities

  Scenario: Completed presentation can be disabled independently
    Given completed-section presentation disabled for Codex
    When reasoning deltas completed sections and answer deltas arrive
    Then incremental reasoning and answers still stream without completed-only text
