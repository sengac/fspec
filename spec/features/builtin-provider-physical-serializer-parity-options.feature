@done
@CMPCT-071
@providers
@context-management
@compaction
@phase-1
Feature: Builtin provider physical serializer parity options
  """
  Admission and physical submission share the selected provider serializer.
  Explicit tool policies and mandatory documents survive serialization, while
  unrepresentable policies and malformed typed parameters fail before sending.
  These regressions retain exact physical-byte comparisons and source requests.
  """

  Background: User Story
    As an agent promoting validated hierarchical memory
    I want admission bytes to equal the next selected provider request
    So that provider-specific overhead cannot bypass the capacity veto

  Scenario: Anthropic preserves explicit API key tool policies
    Given a selected Anthropic API key model with an explicit tool policy
    When each supported tool policy is prepared then physically sent
    Then prepared and physical bytes retain the selected tool policy
    And preparation sends nothing and leaves the input request unchanged

  Scenario: Anthropic rejects unrepresentable API key tool policies
    Given a selected Anthropic API key model requiring multiple specific tools
    When admission preparation and physical submission are attempted
    Then both return errors without sending or changing the source request

  Scenario: Anthropic retains mandatory textual documents
    Given selected Anthropic API key and OAuth models with mandatory textual documents
    When each complete request is prepared then physically sent
    Then prepared and physical bytes retain the complete document text and metadata
    And preparation sends nothing and leaves the input request unchanged

  Scenario: OpenAI Responses rejects malformed optional parameters without panicking
    Given a selected OpenAI Responses model with malformed typed optional parameters
    When admission preparation and physical submission are attempted
    Then both return errors without sending or changing the source request
