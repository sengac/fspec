@integration
@providers
@provider-abstraction
Feature: Provider extensions preserve shared session and tool behavior
  """
  Shared persistence treats provider envelope extensions as opaque data.
  Provider adapters own typed continuation, routing and status through a
  pluggable session lifecycle. Process observers live for the process rather
  than a single model turn. File changes use the shared typed diff renderer.
  """

  Scenario: Unknown provider state survives persistence and blob rehydration
    Given a session checkpoint from an independently implemented provider
    When its envelopes are persisted and rehydrated
    Then its opaque state and explicit reset markers are preserved
    And canonical message fields cannot be replaced by extension metadata

  Scenario: Process clear survives completion of the originating model turn
    Given a facade command that remains alive without output
    And the quiet-process prompt has fired
    When its model turn unregisters callbacks and the command exits
    Then the original observer receives exactly one clear within a detector tick
    And another session receives no events

  Scenario: Patch changes use the shared colored diff renderer
    Given a successful patch containing additions removals and multiple files
    When its tool result reaches the agent view
    Then added lines use the same green styling as Write
    And removed lines use the same red styling as Edit
    And each affected file is identified in the expanded diff

  Scenario: Rejected patches retain their error output
    Given a patch tool call that fails validation
    When its error result reaches the agent view
    Then the error is displayed instead of a successful diff

  Scenario: Independent provider extensions receive session lifecycle events
    Given an independently implemented provider session extension
    When state is restored a turn begins and the conversation is reset
    Then the extension receives each lifecycle event and its persistence writer
    And other sessions do not share its state
