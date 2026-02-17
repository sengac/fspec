@bridge
@tools
@BRIDGE-012
Feature: Bridge session chunk filtering

  """
  Bridge tool receives session_id from the tool call context.
  It filters chunks to only relay those matching its bridged session.
  Chunks from other sessions are ignored and not relayed to WebSocket.
  """

  Background: User Story
    As a developer
    I want the bridge to only relay chunks for its connected session
    So that Telegram users see only their session's output

  Scenario: Bridge relays only bridged session chunks
    Given a bridge is connected to session "session-telegram"
    And session "session-telegram" is running
    And session "session-other" is running
    When session "session-telegram" emits a TextDelta chunk
    And session "session-other" emits a TextDelta chunk
    Then the bridge should relay the chunk from session "session-telegram"
    And the bridge should not relay the chunk from session "session-other"

  Scenario: Bridge receives session_id from tool call context
    Given a Bridge tool invocation with session context
    When the bridge is initialized
    Then it should extract session_id from the tool call context
    And use that session_id for filtering chunks

  Scenario: Bridge input and response flow
    Given a bridge is connected to session "session-x"
    When input arrives from the bridge WebSocket
    Then Rust should emit WatcherInput chunk with session_id "session-x"
    And Rust should emit LLM response chunks with session_id "session-x"
    And the bridge should relay all chunks with session_id "session-x"
