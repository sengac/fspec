@wip
@deferred
@session-management
@acceptance
@real-provider
@rust
@agent-loop
@rpc
Feature: Real Anthropic provider produces streaming text and token updates end-to-end
  """
  Family-acceptance scenario for the RPC-072 agent-loop port. Depends
  on the foundation refit (RPC-072) AND every child card RPC-080..091
  being done. This is the bar the user originally demanded: the
  screenshot pasted on 2026-05-28 is the broken baseline.

  Will be picked up by the last child card to land in the family — or
  by a dedicated acceptance card — once streaming + tokens + persistence
  + history + tools + lifecycle hooks all converge.

  Originally scenario "Real Anthropic provider produces streaming text +
  token updates end-to-end" from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want a real Anthropic call from the fspec binary to stream text + update tokens
    So that the agent-loop refit is verified end-to-end against a real provider

  Scenario: Real Anthropic provider produces streaming text + token updates end-to-end
    Given a real fspec binary is launched against the user's Anthropic credentials
    And the active model is "claude-opus-4-5" (or any other available Claude model)
    And the user opens a Work Agent on the RPC-072 card
    When the user types "what does this card do" and presses Enter
    Then within 30 seconds streaming assistant Markdown text appears in scrollback
    And the TUI header tokens widget shows non-zero down and up arrows
    And no ErrorDialog modal is pushed
