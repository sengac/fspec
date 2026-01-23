@watcher
@codelet
@WATCH-006
Feature: Watcher Injection Message Format

  """
  Add WatcherInput struct in session_manager.rs near SessionRole types (before BackgroundSession)
  Add format_watcher_input() function to create the structured prefix format
  Add watcher_input_tx: mpsc::Sender<WatcherInput> and watcher_input_rx channel pair to BackgroundSession
  Add StreamChunk::watcher_input() constructor in types.rs to create WatcherInput chunk type
  Add receive_watcher_input() method to BackgroundSession that sends to watcher_input_tx
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. WatcherInput struct contains: source_session_id, role_name, authority_level, and message content
  #   2. WatcherInput messages are formatted with structured prefix: [WATCHER: role | Authority: level | Session: id]
  #   3. BackgroundSession has receive_watcher_input() method that accepts WatcherInput and queues it for processing
  #   4. StreamChunk::watcher_input() constructor creates a chunk_type of 'WatcherInput' with the formatted message
  #   5. Authority level in format is derived from SessionRole.authority (Peer or Supervisor)
  #   6. Watcher input is queued via mpsc channel, not processed synchronously
  #
  # EXAMPLES:
  #   1. Watcher 'code-reviewer' (Peer, session abc123) sends 'Consider adding error handling' → formats to '[WATCHER: code-reviewer | Authority: Peer | Session: abc123] Consider adding error handling'
  #   2. Watcher 'security-auditor' (Supervisor, session xyz789) sends 'CRITICAL: SQL injection vulnerability detected' → parent receives WatcherInput chunk with formatted message
  #   3. Parent session with no watchers calls receive_watcher_input() → WatcherInput is queued successfully (watchers can be attached to any session)
  #   4. WatcherInput with empty message → error: message cannot be empty
  #   5. Multiline watcher message → formatted message preserves newlines after the prefix header
  #
  # ========================================

  Background: User Story
    As a watcher session
    I want to inject messages into the parent session stream
    So that the parent session can receive and respond to watcher observations and suggestions

  @wip
  Scenario: Format peer watcher message with structured prefix
    Given a watcher session with role "code-reviewer" and authority "Peer"
    And the watcher session id is "abc123"
    When the watcher sends message "Consider adding error handling"
    Then the formatted message should be "[WATCHER: code-reviewer | Authority: Peer | Session: abc123] Consider adding error handling"

  @wip
  Scenario: Format supervisor watcher message with structured prefix
    Given a watcher session with role "security-auditor" and authority "Supervisor"
    And the watcher session id is "xyz789"
    When the watcher sends message "CRITICAL: SQL injection vulnerability detected"
    Then the parent should receive a WatcherInput chunk
    And the chunk should contain the formatted message with structured prefix

  @wip
  Scenario: Receive watcher input queues message asynchronously
    Given a parent session exists
    When receive_watcher_input is called with a valid WatcherInput
    Then the input should be queued via the watcher input channel
    And the method should return immediately without blocking

  @wip
  Scenario: Empty watcher message returns error
    Given a watcher session with role "test-watcher" and authority "Peer"
    And the watcher session id is "test123"
    When the watcher sends an empty message
    Then an error should be returned with message "message cannot be empty"

  @wip
  Scenario: Multiline watcher message preserves formatting
    Given a watcher session with role "code-reviewer" and authority "Peer"
    And the watcher session id is "abc123"
    When the watcher sends a multiline message:
      """
      Issue found on line 42:
      - Missing null check
      - Consider using Option<T>
      """
    Then the formatted message should have the prefix on the first line
    And subsequent lines should be preserved without additional prefixes
