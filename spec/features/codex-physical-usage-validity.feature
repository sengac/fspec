@done
@CMPCT-074
@compaction
@agent-core
@context-management
@phase-1
@bug
Feature: Codex validates physical usage before aggregation
  """
  Individually invalid physical measurements must never be rehabilitated by
  later aggregation. Missing detail remains unknown rather than measured zero.
  Individually valid measurements whose sum is unrepresentable fail boundedly
  without publishing completion or changing the committed checkpoint.
  """

  Background: User Story
    As an agent using Codex physical response evidence
    I want only valid bounded measurements included in aggregate spend
    So that accounting cannot invent measurements or disguise invalid evidence

  Scenario: Invalid final physical usage cannot publish completion
    Given a Codex terminal whose cache subset exceeds input tokens
    When the physical stream is consumed
    Then invalid usage fails without measured usage or final completion
    And the committed checkpoint remains unchanged

  Scenario: Later valid usage cannot rehabilitate an invalid physical measurement
    Given invalid physical usage followed by a valid continuation terminal
    When the physical stream is consumed
    Then invalid usage fails before continuation without measured usage or final completion
    And the committed checkpoint remains unchanged

  Scenario: Missing physical detail remains unknown in aggregate spend
    Given Codex physical usage without cache or reasoning details
    When the physical stream finishes
    Then aggregate inclusive totals remain measured and both detail subsets remain unknown

  Scenario: Aggregate physical usage overflow fails boundedly
    Given two individually valid Codex measurements whose sum exceeds the token range
    When the continuation stream is consumed
    Then accounting fails without final completion or a changed checkpoint

  Scenario: Missing final physical usage cannot publish completion
    Given a Codex final terminal without physical usage
    When the physical stream is consumed
    Then missing usage fails without measured usage or final completion
    And the committed checkpoint remains unchanged

  Scenario: Missing physical usage cannot start a continuation
    Given a Codex continuation terminal without physical usage
    When the physical stream is consumed
    Then missing usage fails before continuation without measured usage or final completion
    And the committed checkpoint remains unchanged
