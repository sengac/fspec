@done
@session-creation
@session
@providers
@rust
@provider
@PROV-101
Feature: Session creation requires an explicit model

  # ARCHITECTURE NOTES (PROV-101 #1/#2):
  # handle_impl.rs create_session / create_isolated_session must NOT default to
  # "anthropic/claude-opus-4-5" via .unwrap_or_else when get_default_model() is
  # None. With no default: create_session returns an empty SessionId (declined)
  # and create_isolated_session returns Err — no anthropic substitution. With an
  # explicit default the session adopts THAT provider/model.
  # Tests run fully offline: a temp data dir is seeded with a trimmed models.json
  # so registry validation needs no network; dummy creds satisfy detection.

  Background: User Story
    As a developer integrating provider/model/profile selection
    I want session creation to fail loudly when no model is explicitly set
    So that the system never silently substitutes anthropic/claude

  Scenario: create_session declines when no default model is set
    Given a SessionManager with no default model set
    When I call create_session with no role
    Then the returned session id value is empty
    And no session exists in the manager

  Scenario: create_session uses the explicit default model, never anthropic
    Given a SessionManager with the default model set to "google/gemini-2.5-pro"
    When I call create_session with no role
    Then the returned session id value is not empty
    And the created session model is "google/gemini-2.5-pro"

  Scenario: create_isolated_session errors when no default model is set
    Given a SessionManager with no default model set
    When I call create_isolated_session with no role
    Then create_isolated_session returns an error
    And no session exists in the manager
