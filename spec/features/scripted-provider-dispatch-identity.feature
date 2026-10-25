@done
@BUG-267
@providers
@provider-abstraction
Feature: Scripted provider dispatch identity
  """
  Shared dispatch must use the already selected typed implementation, not
  reinterpret a Custom slug as a native provider. This repair is limited to
  selections without an explicit facade override. Explicit facade routing,
  incompatible native getter rejection, test-support stub routing, credentials,
  shadowing escape hatch and admission remain unchanged. Actual shared-loop
  requests use isolated temporary configs and synthetic numeric-loopback HTTP.
  """

  Background: User Story
    As a user selecting a scripted provider
    I want dispatch to retain the selected implementation
    So that a built-in name cannot silently replace my script

  Scenario: Built-in-name scripts receive real shared-loop requests
    Given registered scripts collide with each supported native provider name
    When their selected custom implementations run through the shared loop
    Then each script receives its own dialect and resolved model with armed native Done
    And attributed Done completes each task with exact physical usage and no nudge

  Scenario: Alias-named scripts retain selected identity
    Given registered scripts use native provider aliases
    When their selected custom implementations run through the shared loop
    Then each script receives its own dialect and resolved model with armed native Done
    And attributed Done completes each task with exact physical usage and no nudge

  Scenario: Noncolliding custom dispatch remains compatible
    Given a registered script has a noncolliding provider name
    When its selected custom implementation runs through the shared loop
    Then the script receives its own dialect and resolved model with armed native Done
    And attributed Done completes the task with exact physical usage and no nudge

  Scenario: Native selection and disabled shadowing retain native dispatch
    Given a genuine native OpenAI selection or a colliding script with shadowing disabled
    When the selected implementation runs through the shared loop
    Then only the native dialect reaches the local native endpoint with armed Done
    And attributed Done completes the task with exact physical usage and no nudge

  Scenario: Explicit incompatible facade remains fail closed
    Given a selected Custom claude has an explicit openai facade override
    When the selected implementation runs through the shared loop
    Then the incompatible native getter rejects the turn without any HTTP request
