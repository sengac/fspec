@done
@AMGR-010
Feature: Agent messaging — plain, bidirectional, any-to-any

  """
  Add Message variant to AgentManagerAction enum in types.rs: Message { session_id: String, message: String }. Add MessageDelivered variant to AgentManagerResult.
  Handler implementation in agent_manager_handler.rs: Message action looks up target session by ID from SessionManager, calls session.receive_incoming_message() with IncomingMessage { source_session_id, role_name, message, images: None }. Uses try_send which returns TrySendError::Full for channel-full case.
  The sender's role is obtained from the calling session's role field (Option<String> on BackgroundSession, simplified from SupervisorRole in AMGR-008). If None, pass empty string as role_name.
  No new channels or select branches needed — the existing incoming_message_rx branch in agent_loop's tokio::select! already handles delivery to the target. Messages are formatted via format_incoming_message() before being sent to the LLM.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Any session can send a message to any other session by ID — no access control on sending (supervisor→subordinate, subordinate→supervisor, peer-to-peer)
  #   2. Messages are delivered through the target session's existing incoming_message channel (mpsc, capacity 16) — reuses the IncomingMessage infrastructure
  #   3. Messages queue if the target is mid-generation — no interruption of LLM processing. Picked up on next agent_loop iteration via tokio::select!
  #   4. If the incoming_message channel is full (16 pending messages), the send fails with error code delivery_failed
  #   5. Message action requires both session_id (target) and message (text content) parameters. Missing either returns error code invalid_parameter
  #   6. Success response shape: { delivered: true, session_id: 'target-id' }. Error response shape: { error: true, code: string, message: string }
  #   7. The message is delivered as an IncomingMessage with source_session_id set to the sender's session ID and role_name set to the sender's role (or empty if no role)
  #   8. Sending a message to your own session ID is allowed (self-messaging) — no special case needed
  #
  # EXAMPLES:
  #   1. Supervisor spawns subordinate, then sends message(session_id=subordinate, message='Analyze auth.rs for security issues') — gets { delivered: true, session_id: 'subordinate-uuid' }
  #   2. Subordinate sends message back to its spawner with results — message(session_id=spawner, message='Found 2 SQL injection vulnerabilities') — gets { delivered: true }
  #   3. Agent sends message to nonexistent session_id — gets { error: true, code: 'session_not_found', message: '...' }
  #   4. Agent sends 17 messages rapidly to a target that is busy processing — first 16 succeed, 17th gets { error: true, code: 'delivery_failed', message: '...' }
  #   5. Two subordinates of the same supervisor send messages to each other (peer-to-peer) — both deliveries succeed, no spawner relationship required
  #   6. Agent calls message action without session_id — gets { error: true, code: 'invalid_parameter', message: 'session_id is required' }
  #   7. Target session receives message while idle — message appears as next input and triggers agent processing with formatted sender info
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to send plain text messages to other agent sessions
    So that agents can coordinate work, delegate tasks, and report results back

  @message @spawn
  Scenario: Supervisor sends task to subordinate
    Given a supervisor session has spawned a subordinate session
    When the supervisor calls AgentManager with action "message", session_id of subordinate, and message "Analyze auth.rs for security issues"
    Then the response should contain "delivered" as true
    And the response should contain "session_id" matching the subordinate's ID
    And the subordinate's incoming message channel should contain the message

  @message @spawn
  Scenario: Subordinate reports results to supervisor
    Given a supervisor session has spawned a subordinate session
    When the subordinate calls AgentManager with action "message", session_id of supervisor, and message "Found 2 SQL injection vulnerabilities"
    Then the response should contain "delivered" as true
    And the response should contain "session_id" matching the supervisor's ID

  @message @error
  Scenario: Message to nonexistent session returns error
    Given a session with AgentManager available
    When the agent calls AgentManager with action "message", session_id "nonexistent-uuid", and message "hello"
    Then the response should contain "error" as true
    And the response should contain "code" as "session_not_found"
    And the response should contain a "message" string

  @message @capacity
  Scenario: Channel full returns delivery failed error
    Given a target session with its incoming message channel full at capacity 16
    When the agent calls AgentManager with action "message" to the target session
    Then the response should contain "error" as true
    And the response should contain "code" as "delivery_failed"
    And the response should contain a "message" string

  @message @peer
  Scenario: Peer-to-peer messaging between subordinates
    Given a supervisor has spawned two subordinate sessions A and B
    When subordinate A calls AgentManager with action "message" to subordinate B with message "coordinate on task X"
    Then the response should contain "delivered" as true
    And subordinate B's incoming message channel should contain the message from A

  @message @validation
  Scenario: Missing session_id returns invalid parameter error
    Given a session with AgentManager available
    When the agent calls AgentManager with action "message" without a session_id
    Then the response should contain "error" as true
    And the response should contain "code" as "invalid_parameter"

  @message @validation
  Scenario: Missing message text returns invalid parameter error
    Given a session with AgentManager available
    When the agent calls AgentManager with action "message" with session_id but without message text
    Then the response should contain "error" as true
    And the response should contain "code" as "invalid_parameter"

  @message @format
  Scenario: Delivered message includes sender identity
    Given a supervisor session with role "security-reviewer" has spawned a subordinate
    When the supervisor sends a message "Check for XSS" to the subordinate
    Then the IncomingMessage should have source_session_id matching the supervisor's ID
    And the IncomingMessage should have role_name "security-reviewer"
    And the IncomingMessage should have the message text "Check for XSS"

  @message @format
  Scenario: Sender without role delivers empty role name
    Given a supervisor session with no role has spawned a subordinate
    When the supervisor sends a message "Do analysis" to the subordinate
    Then the IncomingMessage should have role_name as empty string

  @message @self
  Scenario: Self-messaging is allowed
    Given a session with AgentManager available
    When the agent calls AgentManager with action "message" targeting its own session_id with message "note to self"
    Then the response should contain "delivered" as true
    And the session's incoming message channel should contain the self-addressed message

  @message @integration
  Scenario: Message action is dispatched through AgentManagerAction enum
    Given the AgentManagerAction enum includes a Message variant with session_id and message fields
    When a message action is deserialized from JSON input
    Then it should produce a Message variant with the correct session_id and message values
