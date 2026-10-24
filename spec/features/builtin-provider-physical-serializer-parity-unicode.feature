@done
@CMPCT-071
@providers
@context-management
@compaction
@phase-1
Feature: Builtin provider physical serializer parity Unicode
  """
  Arbitrary Unicode tool descriptions pass through the actual selected builtin
  adapters. Admission bytes equal captured physical request bodies without
  normalizing or dropping descriptions, mandatory context or tool history.
  """

  Background: User Story
    As an agent promoting validated hierarchical memory
    I want admission bytes to equal the next selected provider request
    So that provider-specific overhead cannot bypass the capacity veto

  Scenario: Builtin physical serializers preserve arbitrary Unicode tool descriptions
    Given selected builtin adapters with arbitrary Unicode tool descriptions
    When each request is prepared for admission then physically sent
    Then the physical bodies equal the admission bytes and retain the exact descriptions
