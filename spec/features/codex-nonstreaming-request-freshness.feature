@done
@CMPCT-078
@CMPCT-080
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Codex nonstreaming requests retain their prepared generation authority
  """
  Nonstreaming completions retain the same authority while awaiting a response;
  they cannot relabel an old response with a newer committed generation.
  """

  Scenario Outline: Nonstreaming responses cannot outlive their checkpoint authority
    Given a nonstreaming Codex request is waiting for its physical response
    When checkpoint authority changes through <invalidation> before the response arrives
    Then completion returns a typed checkpoint failure without publishing the stale response
    And the authoritative checkpoint and persistence observer remain unchanged
    And an authentic fresh request succeeds without a replacement transport

    Examples:
      | invalidation           |
      | failed restore         |
      | generation replacement |
