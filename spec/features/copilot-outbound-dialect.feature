@done
@BUG-268
@providers
@provider-abstraction
Feature: Copilot outbound dialect
  """
  Use the unchanged PROV-055 repository selector in every actual front door.
  Reuse existing Chat/Responses adapters, middleware and typed tool identities.
  Synthetic numeric-loopback tests do not prove live model/account support.
  The complete approved opaque/refresh ownership contract is attached to BUG-268;
  endpoint-only success cannot close the whole card.
  """

  Background: User Story
    As a Copilot user
    I want the selected API dialect to control real requests
    So that advertised endpoint selection is not a disconnected helper

  Scenario: Client model requests honor the endpoint table
    Given synthetic Copilot credentials and local endpoints for the documented model table
    When actual client completion models send requests
    Then each URI and request body match the selected dialect with Copilot headers

  Scenario: Nonstreaming provider requests honor both dialects
    Given synthetic Copilot credentials and local endpoints for Chat and Responses
    When the provider completes a message through its public nonstreaming entry point
    Then each URI and request body match the selected dialect with Copilot headers

  Scenario: Native agents retain tool nudge and Done continuation
    Given an armed native Copilot agent and a local tool stop and Done sequence in each dialect
    When the shared stream loop completes the task
    Then tool results retain invocation identities and exactly one nudge precedes accepted Done
    And physical usage is billed once per request without extra inference

  Scenario: Public client agents execute in both dialects
    Given a public Copilot client agent and synthetic loopback endpoints
    When the client agent actually completes a prompt for each selected dialect
    Then captured URI body headers and ordinary answer match the chosen dialect

  Scenario: Nested Responses images retain real middleware classification
    Given a Copilot Responses request carries an image within a user input message
    When the real completion model sends that request through middleware
    Then the captured nested image is preserved and the vision request header is true
