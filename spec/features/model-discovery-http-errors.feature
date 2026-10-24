@BUG-218
@done
@providers
@model-selection
@integration
Feature: Typed HTTP outcomes for local model discovery
  """
  Optional listing is unsupported only for HTTP 404/405. The error retains a
  typed status so callers never infer capability from free-form error strings.
  Preserve proxy prefixes and the existing single v1 suffix composition.
  All fixtures use loopback HTTP; no deployed endpoints are contacted.
  """

  Background: User Story
    As a local OpenAI-compatible profile user
    I want unsupported discovery to be distinct from other listing failures
    So that a model-listing 404 is not mistaken for failed inference

  Scenario: Discovery preserves proxy prefixes and composes one version suffix
    Given a local models endpoint behind a proxy prefix with bearer authentication
    When discovery uses bases with and without a trailing v1 and slash
    Then every request preserves the prefix and uses exactly one v1 models path

  Scenario: Unsupported model listing has a typed nonretryable error
    Given local models endpoints return HTTP 404 and 405
    When authenticated model discovery requests each endpoint
    Then each result identifies unsupported discovery with its HTTP status
    And the errors retain the provider identity and are not retryable

  Scenario: Other discovery errors are not unsupported listing
    Given local models endpoints return authentication server and malformed response failures
    When authenticated model discovery requests each endpoint
    Then none of the errors is classified as unsupported discovery
