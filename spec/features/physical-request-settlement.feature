@done
@CMPCT-069
@providers
@context-management
@compaction
@phase-1
Feature: Physical request settlement is durable and independent of current context
  """
  The durable ledger owns physical request identities and measured spend.
  Context replacement does not erase billed requests. Unknown measurements stay
  unknown, conflicting replay fails, and persistence failure blocks success.
  """

  Scenario: Physical requests settle once independently of context replacement
    Given measured intermediate and retry requests with stable physical identities
    When duplicate terminal events and a context replacement are persisted
    Then each physical request is billed once and the replaced context stays separate
    And cache evidence survives a cold store reload

  Scenario: Failed or cancelled requests never invent usage
    Given a physical request without measured final usage
    When the request fails or is cancelled before final measurement
    Then no spend is fabricated and previous measured requests remain settled

  Scenario: Conflicting replay is rejected without changing settlement
    Given a physical request was settled with measured usage
    When a replay reuses its identity with different measured counts
    Then settlement fails explicitly and the committed ledger remains unchanged

  Scenario: Persistence failure blocks successful settlement
    Given a measured final response and an unavailable session manifest
    When the real finish hook attempts request settlement
    Then inference is cancelled and no successful settlement is claimed
