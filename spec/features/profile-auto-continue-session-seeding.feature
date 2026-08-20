@PROV-142
@wip
@rust
@session
@providers
Feature: Per-profile autoContinue session seeding

  """
  When a session is created against a profile model
  (openai:<profile>/<model>), the session's auto-continue state is seeded
  from the profile's stored autoContinue value BEFORE the first user message
  is dispatched. The seed happens in the shared session creation helper
  (create_background_session_inner,
  rust/sessions/src/session_creation_helper.rs) so both
  create_session_with_id and create_session_from_manifest paths get it.
  autoContinue 300 ⇒ continue_enabled=true, budget=300; autoContinue 0 or
  absent ⇒ continue_enabled=false (today's behavior). The profile default is
  a seed only — runtime /continue still overrides it. Verified by
  rust/sessions/tests/prov142_session_seed.rs.
  """

  Background: User Story
    As a developer using a local OpenAI-compatible server profile
    I want new sessions against that profile to start with the profile's auto-continue default
    So that I never have to re-type /continue at the start of each session

  Scenario: A session against a profile with autoContinue 300 starts with auto-continue on and budget 300
    Given a stored profile whose autoContinue value is 300
    When a session is created against a model of that profile
    Then the session's auto-continue is enabled
    And the session's continue budget is 300

  Scenario: A session against a profile with autoContinue 0 starts with auto-continue off
    Given a stored profile whose autoContinue value is 0
    When a session is created against a model of that profile
    Then the session's auto-continue is disabled

  Scenario: A session against a profile without an autoContinue key starts with auto-continue off
    Given a stored profile with no autoContinue key
    When a session is created against a model of that profile
    Then the session's auto-continue is disabled
