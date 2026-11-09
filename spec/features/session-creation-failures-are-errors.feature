@BUG-582
@done
@BUG-399
@rpc
@session
@session-creation
@BUG-425
Feature: Session creation failures are errors
  """
  SessionManagerHandle offered two creators: the infallible create_session, whose SessionManager implementation logged
  a failure and returned SessionId::new(String::new()), and create_session_checked, which returns the error. The RPC
  create_session (kept at its frozen wire position) called the infallible one and answered a daemon without a session
  manager with the placeholder id "rpc-no-session-manager". A caller could not tell either sentinel from a real id.

  Now the trait has only create_session_checked (rust/core/src/session_manager_handle/handle_trait_1.rs), and the RPC
  create_session returns Result<SessionId, String> by delegating to it (rust/rpc/src/service_implementation/
  interaction_sessions.rs). The MCP dispatch of create_session maps the Err to InvokeError::Daemon. The method name and
  variant order are unchanged, so rust/rpc/tests/fixtures/wire_order_frozen.txt is untouched; only the response type
  of create_session changes.
  """

  Background: User Story
    As a client of the fspec daemon
    I want a failed session creation to come back as an error
    So that I never treat a placeholder id as a live session

  Scenario: A creation failure reaches the RPC create_session caller as an error
    Given a daemon over a real SessionManager whose default model is the malformed model string "not-a-model-string"
    When a client invokes create_session over the RPC
    Then the call fails with the daemon's model parse error
    And the session manager holds no live session

  Scenario: A daemon without a session manager refuses create_session
    Given a daemon built without a session manager
    When a client invokes create_session over the RPC
    Then the call fails with an error saying session creation requires a session manager

  Scenario: The session manager handle has no infallible session creator
    Given the SessionManagerHandle trait source
    When its session creation methods are listed
    Then no create_session method returns a bare SessionId
    And create_session_checked is a required method returning Result<SessionId, String>

  Scenario: The checked creator is a wire-compatibility alias of create_session
    Given a daemon built without a session manager
    When a client invokes create_session_checked over the RPC
    Then the call fails with an error saying session creation requires a session manager
    And the implementation of create_session_checked delegates to create_session
    And the TUI transports create sessions with create_session
