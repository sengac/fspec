@phase-1
@provider-abstraction
@providers
@streaming
@PROV-160
Feature: Response attempt marker serialization
  """
  Physical response attempt boundaries remain unit variants in rig's streaming API.
  Their JSON encodings identify start and abort separately without changing existing content encodings.
  Tagged multi-turn stream envelopes preserve these boundaries as well.
  """

  Scenario: Attempt boundaries retain their identity through JSON
    Given response attempt start and abort markers
    When each marker is serialized and deserialized
    Then their encodings differ and both identities are preserved

  Scenario: Multi-turn envelopes preserve attempt boundaries
    Given multi-turn assistant items containing start and abort markers
    When each envelope is serialized and deserialized
    Then both boundary identities are preserved

  Scenario: Existing content encodings remain unchanged
    Given text and final response content without attempt markers
    When the content is serialized and deserialized
    Then its existing JSON representation and identity are preserved
