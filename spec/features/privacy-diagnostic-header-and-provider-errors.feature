@BUG-239
@phase-1
@cli
@logging
@integration
@security
Feature: Privacy diagnostic header and provider errors
  """
  Shared diagnostic sinks redact recognizable complete Authorization and Cookie
  header values, including Basic credentials and all cookie pairs. Valid JSON
  embedded in diagnostic errors is classified by its content keys so provider
  error bodies cannot bypass the default policy through message interpolation.
  Actionable error message/status metadata survives; explicit local opt-in may
  include bounded redacted JSON content. This is not perfect arbitrary prose
  secret detection. Tests exercise the real application stream loop and HTTP
  provider adapter, not source text or copies of its logging expressions.
  """

  Background: User Story
    As an operator
    I want provider failures without exposed credentials or conversation bodies
    So that diagnostics remain actionable and safe to review

  Scenario: Embedded credential headers redact complete values in every privacy mode
    Given default and explicit local payload diagnostic modes
    When errors contain Basic authorization and cookies with multiple session pairs
    Then neither mode exposes the encoded credential or any cookie pair
    And the surrounding status and actionable root cause remain readable

  Scenario: Application provider failures do not expose serialized conversation bodies
    Given a local provider returns an error body containing conversation payloads
    When the real application stream loop reports the provider failure
    Then default diagnostics omit the serialized conversation markers
    And the provider error message and application correlation remain readable

  Scenario: Streaming provider HTTP failures do not expose debug wrapped conversation bodies
    Given a streaming provider returns an HTTP failure with conversation JSON
    When the real provider adapter emits the streaming error through diagnostic sinks
    Then default sinks omit conversation markers from the debug wrapped body
    And status correlation and the provider root cause remain readable

  Scenario: Known provider instruction fields follow payload authorization and bounds
    Given diagnostics contain Anthropic system and Responses instructions fields
    When default and explicit local payload diagnostic modes format the JSON
    Then default logs omit both instruction markers and local opt in bounds each field
    And actionable error metadata remains readable

  Scenario: Credential headers inside JSON preserve trailing error metadata
    Given a JSON error string contains a credential header between newline separated causes
    When default and explicit local payload diagnostic modes format the error
    Then credentials are absent from the JSON diagnostic record
    And the same record retains the final root cause and status
