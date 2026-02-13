@telegram
@bridge
@BRIDGE-002
Feature: Telegram Bridge Endpoint

  """
  WebSocket server using 'ws' npm package - listens for codelet BridgeManager connections
  Telegram Bot API via 'node-telegram-bot-api' with polling mode for receiving messages
  MarkdownV2 escaping for special characters: _ * [ ] ( ) ~ ` > # + - = | { } . !
  Tool tracking: maintain Map<id, name> from tool_call chunks; lookup tool_call_id from tool_result to display tool names
  Single chatId variable (not a map) - set from TELEGRAM_CHAT_ID env var or updated by most recent Telegram message
  Environment variables via 'dotenv': TELEGRAM_BOT_TOKEN (required), TELEGRAM_CHAT_ID (optional, for pre-configuration), WEBSOCKET_PORT (default 8080), WEBSOCKET_HOST (default localhost)
  Dependency:
  - node-telegram-bot-api ^0.66.0, ws ^8.16.0, dotenv ^16.3.1
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Telegram messages limited to 4096 characters - must truncate longer content
  #   2. Endpoint runs as WebSocket server - codelet's BridgeManager connects as client
  #   3. Uses node-telegram-bot-api npm package for Telegram Bot API integration
  #   4. Bot token from BotFather stored in .env file (TELEGRAM_BOT_TOKEN)
  #   5. MarkdownV2 formatting for code blocks, bold, italic - with proper character escaping
  #   6. Smart truncation preserves beginning (~1500 chars) and end (~1500 chars) with omission indicator in middle
  #   7. All StreamChunk types handled: text, thinking, tool_call, tool_result, done, error
  #   8. Single standalone TypeScript file: bridge/telegram-endpoint.ts
  #   9. Chunk messages (codelet → endpoint): {type: 'chunk', session_id, data: StreamChunk} - same format as BRIDGE-001 outbound
  #   10. Input messages (endpoint → codelet): {type: 'input', session_id, message: string} - same format as BRIDGE-001 inbound
  #   11. One endpoint instance serves one codelet session at a time. First WebSocket connection establishes the session. Additional connection attempts are rejected. When the session disconnects, the endpoint accepts a new connection. Multiple Telegram users can message the bot - all inputs route to the connected session.
  #   12. Log error to console, drop the message, continue receiving chunks. No buffering - Telegram API outages are usually brief and buffering risks memory growth and stale message floods.
  #   13. Display a simple ✓ completion marker when done chunk is received. Provides clear signal on mobile that the agent finished responding.
  #   14. Yes, detect open code blocks and close them with ``` before the truncation marker. Re-open with ``` after the marker if the end portion contains code. Prevents MarkdownV2 formatting chaos.
  #   15. Yes - tool_result chunk includes tool_call_id field which correlates to tool_call.id. Endpoint stores Map<id, name> from tool_call chunks, then looks up name when tool_result arrives.
  #   16. Hybrid approach: Optional TELEGRAM_CHAT_ID env var for pre-configuration. If not set, wait for first Telegram message to learn chat_id. While waiting, drop chunks with clear console warning. Once linked, most recent Telegram message updates the active chat_id (allows switching devices).
  #
  # EXAMPLES:
  #   1. Codelet sends text chunk 'Hello, I can help' → Endpoint formats with MarkdownV2 → Sent to linked Telegram chat
  #   2. Codelet sends 10,000 char response → Smart truncation: first 1500 + '[...6000 chars omitted...]' + last 1500 → Sent as single message under 4096 limit
  #   3. User sends 'build the app' in Telegram → Endpoint sends {type: 'input', session_id, message: 'build the app'} via WebSocket → Codelet receives as user input
  #   4. Codelet sends thinking chunk → Endpoint formats as '💭 [thinking content]' → Sent to Telegram
  #   5. Codelet sends tool_call chunk for 'Read' → Endpoint formats as '🔧 Running: Read' → Sent to Telegram
  #   6. Codelet sends error chunk → Endpoint formats as '❌ Error: [message]' → Sent to Telegram
  #   7. Code block in response → Endpoint preserves ```language markers → MarkdownV2 formatted code block in Telegram
  #   8. Endpoint starts with TELEGRAM_BOT_TOKEN in .env → WebSocket server listens on configured port → Ready to accept codelet connections
  #   9. Codelet sends tool_call {name: 'Read', id: 'abc'} → Endpoint stores mapping → Later tool_result {tool_call_id: 'abc', content: '...'} arrives → Lookup 'abc' → Display '[Read] ...'
  #   10. TELEGRAM_CHAT_ID set in .env → Endpoint starts → Codelet connects → AI responds → Chunks immediately flow to pre-configured chat
  #   11. No TELEGRAM_CHAT_ID → Endpoint starts with warning → Codelet connects → AI responds → Chunks dropped with console log → User messages bot → chatId learned → Subsequent chunks flow to user's chat
  #
  # QUESTIONS (ANSWERED):
  #   Q: How does a Telegram chat get associated with a specific codelet session? What if multiple codelet sessions are running?
  #   A: Hybrid approach: Optional TELEGRAM_CHAT_ID env var for pre-configuration. If not set, wait for first Telegram message to learn chat_id. While waiting, drop chunks with clear console warning. Once linked, most recent Telegram message updates the active chat_id (allows switching devices).
  #
  #   Q: What should happen if the Telegram Bot API is unavailable (rate limited, invalid token, network error)?
  #   A: Log error to console, drop the message, continue receiving chunks. No buffering - Telegram API outages are usually brief and buffering risks memory growth and stale message floods.
  #
  #   Q: Should 'done' chunks be displayed in Telegram (e.g., with ✓ marker), or silently ignored?
  #   A: Display a simple ✓ completion marker when done chunk is received. Provides clear signal on mobile that the agent finished responding.
  #
  #   Q: When truncating a message mid-code-block, should we add a closing ``` to prevent formatting issues?
  #   A: Yes, detect open code blocks and close them with ``` before the truncation marker. Re-open with ``` after the marker if the end portion contains code. Prevents MarkdownV2 formatting chaos.
  #
  #   Q: Does tool_result chunk include a correlation ID to identify the original tool_call? (Needed to display tool name in results)
  #   A: Yes - tool_result chunk includes tool_call_id field which correlates to tool_call.id. Endpoint stores Map<id, name> from tool_call chunks, then looks up name when tool_result arrives.
  #
  #   Q: What happens when chunks arrive but no Telegram user has messaged the bot yet? (No chat_id to send to) Options: A) Buffer until user messages, B) Drop chunks and log warning, C) Require TELEGRAM_CHAT_ID env var for pre-configuration, D) Something else?
  #   A: Hybrid approach (Option D): Optional TELEGRAM_CHAT_ID env var for pre-configuration. If not set, drop chunks with console warning until first Telegram message establishes the chat_id. Most recent message updates active chat (allows device switching).
  #
  # ========================================

  Background: User Story
    As a developer
    I want to run a standalone Telegram bridge endpoint
    So that I can monitor and interact with codelet sessions remotely from my phone via Telegram

  # -------------------------------------------
  # Endpoint Startup & Configuration
  # -------------------------------------------

  @startup
  Scenario: Start endpoint with required configuration
    Given TELEGRAM_BOT_TOKEN is set in .env
    When I start the telegram endpoint
    Then the WebSocket server should listen on the configured port
    And the Telegram bot should connect with polling mode
    And the endpoint should be ready to accept codelet connections

  @startup @error-handling
  Scenario: Fail to start without required bot token
    Given TELEGRAM_BOT_TOKEN is not set in .env
    When I attempt to start the telegram endpoint
    Then the endpoint should exit with an error message
    And the error message should indicate TELEGRAM_BOT_TOKEN is required

  @chat-association @pre-configured
  Scenario: Use pre-configured chat ID for immediate message delivery
    Given TELEGRAM_BOT_TOKEN is set in .env
    And TELEGRAM_CHAT_ID is set in .env
    And the endpoint is running
    When a codelet session connects
    And the AI responds with "Hello"
    Then the message should be sent immediately to the pre-configured Telegram chat

  @chat-association @dynamic
  Scenario: Learn chat ID from first Telegram message
    Given TELEGRAM_BOT_TOKEN is set in .env
    And TELEGRAM_CHAT_ID is not set
    And the endpoint is running
    When a codelet session connects
    And the AI responds with "Hello"
    Then the chunk should be dropped with a console warning
    When a user sends "hi" in Telegram
    Then the chat ID should be learned from that message
    When the AI responds with "How can I help?"
    Then the message should be sent to the learned Telegram chat

  @connection-management
  Scenario: Reject additional codelet connections
    Given the endpoint is running
    And a codelet session is already connected
    When another codelet session attempts to connect
    Then the connection should be rejected
    And the first session should remain connected

  @connection-management
  Scenario: Accept new connection after session disconnect
    Given the endpoint is running
    And a codelet session is connected
    When the codelet session disconnects
    Then the endpoint should accept new connections
    And a new codelet session should be able to connect

  # -------------------------------------------
  # Outbound: StreamChunk → Telegram
  # -------------------------------------------

  @outbound @text
  Scenario: Relay text chunk to Telegram with MarkdownV2 formatting
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When the codelet sends a text chunk "Hello, I can help"
    Then the message should be formatted with MarkdownV2
    And the message should be sent to the linked Telegram chat

  @outbound @thinking
  Scenario: Relay thinking chunk with emoji prefix
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When the codelet sends a thinking chunk "Let me analyze this..."
    Then the message should be formatted as "💭 Let me analyze this..."
    And the message should be sent to the linked Telegram chat

  @outbound @tool-call
  Scenario: Relay tool_call chunk with tool indicator
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When the codelet sends a tool_call chunk with name "Read" and id "abc123"
    Then the message should be formatted as "🔧 Running: Read"
    And the message should be sent to the linked Telegram chat
    And the tool name should be stored for later correlation

  @outbound @tool-result
  Scenario: Relay tool_result chunk with correlated tool name
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    And a tool_call was received with name "Read" and id "abc123"
    When the codelet sends a tool_result chunk with tool_call_id "abc123" and content "file contents here"
    Then the endpoint should look up the tool name from the stored mapping
    And the message should be formatted as "[Read] file contents here"
    And the message should be sent to the linked Telegram chat

  @outbound @error
  Scenario: Relay error chunk with error indicator
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When the codelet sends an error chunk "Connection failed"
    Then the message should be formatted as "❌ Error: Connection failed"
    And the message should be sent to the linked Telegram chat

  @outbound @done
  Scenario: Display completion marker for done chunk
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When the codelet sends a done chunk
    Then a "✓" completion marker should be sent to the linked Telegram chat

  # -------------------------------------------
  # Message Formatting & Truncation
  # -------------------------------------------

  @truncation
  Scenario: Truncate long messages to fit Telegram limit
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When the codelet sends a text chunk with 10000 characters
    Then the message should be truncated to fit within 4096 characters
    And the first ~1500 characters should be preserved
    And a truncation indicator should be added in the middle
    And the last ~1500 characters should be preserved

  @truncation @code-block
  Scenario: Properly close code blocks when truncating mid-block
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When the codelet sends a text chunk with a code block that exceeds 4096 characters
    Then the open code block should be closed before the truncation marker
    And the code block should be re-opened after the truncation marker if needed
    And the message should be valid MarkdownV2

  @formatting @code-block
  Scenario: Preserve code block language markers
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When the codelet sends a text chunk containing "```python\nprint('hello')\n```"
    Then the code block should be preserved with the language marker
    And the message should be formatted as valid MarkdownV2

  # -------------------------------------------
  # Inbound: Telegram → Codelet
  # -------------------------------------------

  @inbound
  Scenario: Relay Telegram message to codelet as input
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When a user sends "build the app" in Telegram
    Then the endpoint should send a JSON message via WebSocket
    And the message should have type "input"
    And the message should contain the session_id
    And the message should contain "build the app"

  @inbound @device-switch
  Scenario: Update active chat when user messages from different device
    Given the endpoint is running with chat ID "111"
    And a codelet session is connected
    When a user sends a message from chat ID "222"
    Then the active chat ID should be updated to "222"
    And subsequent chunks should be sent to chat ID "222"

  @inbound @multi-user
  Scenario: Route messages from multiple Telegram users to single session
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When user A sends "hello" from chat ID "111"
    And user B sends "hi there" from chat ID "222"
    Then both messages should be routed to the connected codelet session
    And the most recent chat ID "222" should become the active chat for responses

  # -------------------------------------------
  # Error Handling
  # -------------------------------------------

  @error-handling @telegram-api
  Scenario: Handle Telegram API errors gracefully
    Given the endpoint is running with a linked Telegram chat
    And a codelet session is connected
    When the codelet sends a text chunk "Hello"
    And the Telegram API returns an error
    Then the error should be logged to console
    And the message should be dropped
    And the endpoint should continue receiving chunks

