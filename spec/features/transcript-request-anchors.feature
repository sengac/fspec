@done
@agent-core
@session-management
@persistence
@BUG-250
Feature: Preserve causal order when responses fail before assistant output
  """
  Each runtime request durably anchors its original-message prefix before any
  transcript publication. Empty failed responses need no invented assistant
  original. Archive projection preserves request order independently of clocks
  and excludes all lifecycle metadata from reduced model history.
  """

  Scenario: Consecutive empty failures retain their causal archive order
    Given two requests failing before assistant output followed by a successful response
    When their canonical archival pages are repeatedly reassembled
    Then each safe failure follows its user input before the next request
    And the successful original appears once without transcript metadata in model history
