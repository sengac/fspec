@agent-core
@phase-1
@context-management
@compaction
@CMPCT-054
Feature: Terminal compaction dependency lifetime
  """
  A turn owns its compaction builder registration until terminal decisions finish.
  Cleanup releases the registration on normal return and cancellation. Unavailable
  builders fail explicitly without substituting a DAG or publishing completion.
  Integration tests drive the actual outer loop and the registry lifecycle owner.
  A physical provider rejection supplies the overflow; the separate builder request
  must carry the captured target identity and fail without publishing a generation.
  Terminal resolution has already released ordinary turn handlers while retaining
  compaction dependencies; an in-loop builder request cannot satisfy this boundary.
  """

  Scenario: Terminal recovery invokes the turn builder before cleanup
    Given a background session with compactable history and an overflowing terminal error
    When the real outer loop resolves the terminal error
    Then the registered turn builder runs before registration cleanup
    And the physical request belongs to the captured target builder rather than ordinary inference
    And ordinary turn handlers are released while compaction dependencies remain registered
    And its failure leaves committed history unchanged without completion

  Scenario: Missing recovery dependencies fail without replacing context
    Given committed history without a registered compaction builder
    When terminal overflow recovery is requested
    Then recovery returns a typed unavailable failure
    And committed history remains unchanged without completion
    And inference remains blocked until the selected operation is resolved

  Scenario: Cancellation releases the operation registration
    Given an operation-owned compaction builder registration
    When its owning task is cancelled
    Then the builder registration is removed
    And a later operation can register and release its own builder
