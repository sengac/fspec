@done
@session-management
@session
@BUG-405
@rust
Feature: Session operations never fall back to a SessionManager singleton
  """
  The NAPI front door drove SessionManager::instance(), and several lookups
  fell back to that singleton when no manager was passed or registered:
  agent-loop fspec_handler and agent_manager_handler (create_handler and
  await_idle), sessions tool_admission (handler and validator),
  footer_poller::emission_target and BackgroundSession::set_work_unit_context.
  With the NAPI crate gone the fallback only hid a missing registration.
  SessionManager::instance() is deleted; each site now requires the owning or
  registered manager and reports its absence as an error.
  BackgroundSession::set_work_unit_context also returns its focus and
  persistence failures instead of logging and returning.
  Tests: rust/sessions/tests/bug405_no_session_manager_singleton.rs.
  """

  Background: User Story
    As a developer wiring a front door to the session manager
    I want a missing manager to be an error
    So that a forgotten registration never silently works against an empty global

  Scenario: Tool admission for a session with no owning manager is refused
    Given a session whose owning session manager has been dropped
    When a tool invocation is admitted for that session
    Then the admission fails naming the missing session manager

  Scenario: A block notification with no registered chunk sender is reported
    Given no chunk sender is registered for the footer and block notifications
    When the emission target is resolved
    Then it is an error instead of a global broadcast

  Scenario: Binding a work unit reports a failed focus update
    Given a session whose owning session manager has been dropped
    When a work unit is bound to the session
    Then the binding fails instead of being logged and skipped

  Scenario: The SessionManager singleton no longer exists
    Given the codelet-sessions and agent-loop sources
    When they are scanned for the SessionManager singleton
    Then neither defines nor calls SessionManager::instance()
