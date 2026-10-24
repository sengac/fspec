@done
@CMPCT-059
@compaction
@agent-core
@phase-1
@validation
Feature: Durable structured tool identity
  """
  Committed replay rehydrates blob-backed structured suffixes without projecting
  away provider invocation identity or modifying immutable original envelopes.
  """

  Scenario: Committed replay preserves distinct tool item and invocation identities
    Given a committed generation with blob-backed tool call and result suffixes using distinct item and invocation identities
    When a cold reader rehydrates the committed structured replay
    Then both suffixes retain their explicit invocation identity and original provider metadata
    And blob content is restored without modifying immutable originals
