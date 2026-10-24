@done
@CMPCT-071
@providers
@compaction
@context-management
@phase-1
Feature: Selected preparation certifies actual physical provider bytes
  """
  Preparation uses the actual selected adapter serializer without invoking the
  adapter or mutating its source checkpoint. Complete request fit must precede
  durable promotion; unsupported serializers and unknown media cannot authorize
  inference, alternate requests or substitute memory.
  """

  Scenario: Selected preparation uses the physical provider request without changing history
    Given an actual selected provider with instructions schemas and a verified suffix
    When its next generation request is prepared and admitted before promotion
    Then the certificate retains the physical request bytes and the unchanged source checkpoint

  Scenario: Selected preparation cannot certify unknown media or unavailable serialization
    Given a selected provider request with media or no supported physical serializer
    When preparation attempts to certify the complete request
    Then preparation returns a typed failure without sending or changing history
