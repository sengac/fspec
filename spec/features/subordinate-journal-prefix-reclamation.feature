@done
@agent-core
@agent-manager
@rust
@BUG-217
Feature: Reclaim consumed subordinate journal prefixes during sustained output
  """
  A journal retains the common unread suffix, not the entire session history.
  Before publication it reclaims a sufficiently large common consumed prefix,
  even when no cursor reaches the end. Compaction uses bounded copying memory
  and leaves absolute cursor positions and unread event ordering unchanged.
  Storage is bounded by unread data plus amortized compaction slack.
  """

  Background: User Story
    As an operator running long-lived subordinate sessions
    I want already consumed journal history reclaimed
    So that small continuous delivery lag does not exhaust temporary storage

  Scenario: Continuously lagging subscribers retain only their unread suffix and bounded slack
    Given two journal subscribers continuously trail publication by different small amounts
    When sustained output grows far beyond their unread backlog
    Then retained journal storage stays bounded by unread data and compaction slack
    And both subscribers receive every remaining event exactly once in order

  Scenario: Cancelling the slowest subscriber allows its consumed prefix to be reclaimed
    Given a slow journal subscriber retains a large prefix already read by another subscriber
    When the slow subscriber is cancelled and another event is published
    Then the retained journal no longer includes that obsolete prefix
    And the remaining subscriber receives its unread events in order
