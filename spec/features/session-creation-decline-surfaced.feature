@done
@tui
@session-creation
@rust
@PROV-101
Feature: TUI surfaces a declined session creation instead of swallowing it

  # ARCHITECTURE NOTES (PROV-101 FIX 1):
  # create_session returns an empty SessionId when no default model is set
  # (decline). Changing the SessionManagerHandle / RPC wire return type to a
  # typed Result has an unacceptable blast radius (dozens of call sites across
  # rpc-server, rpc-embedded, fspec, fspec-tui and napi tests, plus the
  # FspecBackend trait and three transport impls, with no git safety net).
  # Per the authorized fallback, the TUI callers instead detect the empty id and
  # surface it EXPLICITLY: a shared helper maps the create_session result to
  # Action::SessionCreated for a real id, or Action::SessionCreationDeclined for
  # an empty id. The decline action pushes a Priority::Critical ErrorDialog and
  # NO caller appends an empty-id SessionContext. Test is fully offline against
  # the MockBackend fixture (no network, no env mutation).
  Background: User Story
    As a developer integrating provider/model/profile selection
    I want the TUI to fail loudly when session creation is declined
    So that an empty-id session is never silently created behind my back

  Scenario: A declined session creation maps to an explicit decline action
    Given a create_session result whose session id value is empty
    When the TUI builds the follow-up action for the result
    Then the follow-up action is the session-creation-declined action

  Scenario: A successful session creation maps to a session-created action
    Given a create_session result whose session id value is not empty
    When the TUI builds the follow-up action for the result
    Then the follow-up action is the session-created action

  Scenario: The TUI surfaces a declined session creation as an explicit error
    Given an App whose backend declines create_session with an empty session id
    When the user confirms creating a non-isolated session
    Then an error dialog is shown to the user
    And no session becomes the active session
