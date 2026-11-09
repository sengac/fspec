@done
@tui
@session-creation
@rust
@PROV-101
@BUG-410
Feature: TUI surfaces a declined session creation instead of swallowing it

  # ARCHITECTURE NOTES (PROV-101 FIX 1, BUG-410):
  # create_session returns Result<SessionId, String>. With no default model set
  # it returns an Err naming the missing default model (BUG-410 replaced the
  # former in-band empty-id sentinel). The TUI has ONE creation-failure path:
  # every create_session Err becomes Action::SessionOperationFailed, which the
  # session storage dialog shows, and NO SessionContext is appended or seeded.
  # There is no empty-id check and no separate decline action. Test is fully
  # offline against the MockBackend fixture (no network, no env mutation).
  Background: User Story
    As a developer integrating provider/model/profile selection
    I want the TUI to fail loudly when session creation is declined
    So that a session that was never created is never shown as active

  Scenario: The TUI surfaces a declined session creation as an explicit error
    Given an App whose backend rejects create_session because no default model is set
    When the user confirms creating a non-isolated session
    Then the session failure dialog names the missing default model
    And no session becomes the active session

  Scenario: A created session becomes the active session
    Given an App whose backend creates the session "s-1"
    When the user confirms creating a non-isolated session
    Then the session "s-1" becomes the active session
