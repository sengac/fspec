@done
@agent-core
@session-management
@persistence
@BUG-250
Feature: Restore original terminal outcomes into cold session display
  """
  Cold display combines original dialogue and private terminal projection with
  original record IDs and timestamps. Model restoration excludes terminal prose.
  The canonical paged archive exports the same journal projection. Promoted
  message identities anchor notices after their original request content;
  unpromoted final request content follows the original suffix. Durable journal
  admission order, not wall-clock timestamps, determines notice order. Pinned
  snapshots exclude later journal appends and later original promotions.
  """

  Scenario: Cold session display includes one original safe failure after its prefix
    Given an assistant prefix followed by a durably archived provider failure
    When the session is cold restored twice
    Then each display contains the prefix followed by one safe failure with its original identity and timestamp
    And restored model history contains no terminal prose
