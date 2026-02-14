/**
 * Feature: spec/features/telegram-bridge-endpoint.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Tests the Telegram bridge endpoint that relays codelet sessions to Telegram.
 *
 * BRIDGE-002: Telegram Bridge Endpoint
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Use vi.hoisted to define mocks that can be used in vi.mock
const {
  mockWsOn,
  mockWsClose,
  mockWsSend,
  mockBotSendMessage,
  mockBotOn,
  mockBotStopPolling,
  MockWebSocketServer,
  MockTelegramBot,
} = vi.hoisted(() => {
  const mockWsOn = vi.fn();
  const mockWsClose = vi.fn();
  const mockWsSend = vi.fn();
  const mockBotSendMessage = vi.fn().mockResolvedValue({});
  const mockBotOn = vi.fn();
  const mockBotStopPolling = vi.fn().mockResolvedValue(undefined);

  class MockWebSocketServer {
    on = mockWsOn;
    close = mockWsClose;
    constructor() {}
  }

  class MockTelegramBot {
    on = mockBotOn;
    sendMessage = mockBotSendMessage;
    stopPolling = mockBotStopPolling;
    constructor() {}
  }

  return {
    mockWsOn,
    mockWsClose,
    mockWsSend,
    mockBotSendMessage,
    mockBotOn,
    mockBotStopPolling,
    MockWebSocketServer,
    MockTelegramBot,
  };
});

vi.mock('ws', () => ({
  WebSocketServer: MockWebSocketServer,
  WebSocket: {
    OPEN: 1,
  },
}));

vi.mock('node-telegram-bot-api', () => ({
  default: MockTelegramBot,
}));

vi.mock('dotenv', () => ({
  config: vi.fn(),
}));

// Import types and functions from implementation
import type { OutboundMessage } from '../telegram-endpoint';
import {
  startEndpoint,
  stopEndpoint,
  resetState,
  getState,
  formatForTelegram,
  escapeMarkdownV2,
  truncateMessage,
  handleStreamChunk,
  handleTelegramMessage,
} from '../telegram-endpoint';

describe('Feature: Telegram Bridge Endpoint', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    // Reset mock implementations
    mockWsOn.mockClear();
    mockWsClose.mockClear();
    mockWsSend.mockClear();
    mockBotSendMessage.mockClear().mockResolvedValue({});
    mockBotOn.mockClear();
    mockBotStopPolling.mockClear().mockResolvedValue(undefined);
    // Reset environment variables
    delete process.env.TELEGRAM_BOT_TOKEN;
    delete process.env.TELEGRAM_CHAT_ID;
    delete process.env.WEBSOCKET_PORT;
    delete process.env.WEBSOCKET_HOST;
    // Reset module state
    resetState();
  });

  afterEach(async () => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    // Stop endpoint if running
    const state = getState();
    if (state.isRunning) {
      await stopEndpoint();
    }
    resetState();
  });

  // -------------------------------------------
  // Endpoint Startup & Configuration
  // -------------------------------------------

  describe('Scenario: Start endpoint with required configuration', () => {
    it('should start WebSocket server and Telegram bot when TELEGRAM_BOT_TOKEN is set', () => {
      // @step Given TELEGRAM_BOT_TOKEN is set in .env
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';

      // @step When I start the telegram endpoint
      const result = startEndpoint();

      // @step Then the WebSocket server should listen on the configured port
      expect(result).toBeDefined();
      expect(result.wss).toBeDefined();
      expect(result.isRunning).toBe(true);

      // @step And the Telegram bot should connect with polling mode
      expect(result.bot).toBeDefined();

      // @step And the endpoint should be ready to accept codelet connections
      expect(mockWsOn).toHaveBeenCalledWith('connection', expect.any(Function));
      expect(mockWsOn).toHaveBeenCalledWith('error', expect.any(Function));
    });
  });

  describe('Scenario: Fail to start without required bot token', () => {
    it('should exit with error when TELEGRAM_BOT_TOKEN is not set', () => {
      // @step Given TELEGRAM_BOT_TOKEN is not set in .env
      delete process.env.TELEGRAM_BOT_TOKEN;

      // @step When I attempt to start the telegram endpoint
      // @step Then the endpoint should exit with an error message
      expect(() => startEndpoint()).toThrow();

      // @step And the error message should indicate TELEGRAM_BOT_TOKEN is required
      expect(() => startEndpoint()).toThrow(/TELEGRAM_BOT_TOKEN/);
    });
  });

  // -------------------------------------------
  // Chat Association
  // -------------------------------------------

  describe('Scenario: Use pre-configured chat ID for immediate message delivery', () => {
    it('should send messages immediately when TELEGRAM_CHAT_ID is pre-configured', async () => {
      // @step Given TELEGRAM_BOT_TOKEN is set in .env
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';

      // @step And TELEGRAM_CHAT_ID is set in .env
      process.env.TELEGRAM_CHAT_ID = '12345678';

      // @step And the endpoint is running
      startEndpoint();

      // @step When a codelet session connects
      // (WebSocket connection simulated by state)

      // @step And the AI responds with "Hello"
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'text', text: 'Hello' },
      };
      await handleStreamChunk(chunk);

      // Advance timers to trigger buffer flush
      await vi.advanceTimersByTimeAsync(500);

      // @step Then the message should be sent immediately to the pre-configured Telegram chat
      expect(mockBotSendMessage).toHaveBeenCalledWith('12345678', 'Hello', {
        parse_mode: 'MarkdownV2',
      });
    });
  });

  describe('Scenario: Learn chat ID from first Telegram message', () => {
    it('should drop chunks until chat ID is learned from Telegram message', async () => {
      // @step Given TELEGRAM_BOT_TOKEN is set in .env
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';

      // @step And TELEGRAM_CHAT_ID is not set
      delete process.env.TELEGRAM_CHAT_ID;

      // @step And the endpoint is running
      startEndpoint();

      // @step When a codelet session connects
      // (WebSocket connection simulated by state)

      // @step And the AI responds with "Hello"
      const chunk1: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'text', text: 'Hello' },
      };

      // @step Then the chunk should be dropped with a console warning
      await handleStreamChunk(chunk1);
      await vi.advanceTimersByTimeAsync(500);
      expect(mockBotSendMessage).not.toHaveBeenCalled();

      // @step When a user sends "hi" in Telegram
      handleTelegramMessage('87654321', 'hi');

      // @step Then the chat ID should be learned from that message
      expect(getState().chatId).toBe('87654321');

      // @step When the AI responds with "How can I help?"
      const chunk2: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'text', text: 'How can I help?' },
      };
      await handleStreamChunk(chunk2);

      // Advance timers to trigger buffer flush
      await vi.advanceTimersByTimeAsync(500);

      // @step Then the message should be sent to the learned Telegram chat
      expect(mockBotSendMessage).toHaveBeenCalledWith(
        '87654321',
        expect.stringContaining('How can I help'),
        { parse_mode: 'MarkdownV2' }
      );
    });
  });

  // -------------------------------------------
  // Connection Management
  // -------------------------------------------

  describe('Scenario: Reject additional codelet connections', () => {
    it('should reject second connection when one is already active', () => {
      // @step Given the endpoint is running
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      startEndpoint();

      // @step And a codelet session is already connected
      // Capture the connection handler
      const connectionHandler = mockWsOn.mock.calls.find(
        call => call[0] === 'connection'
      )?.[1];
      expect(connectionHandler).toBeDefined();

      // Simulate first connection
      const mockWs1 = {
        on: vi.fn(),
        close: vi.fn(),
        send: vi.fn(),
        readyState: 1,
      };
      connectionHandler(mockWs1);
      expect(getState().currentSession.ws).toBe(mockWs1);

      // @step When another codelet session attempts to connect
      const mockWs2 = {
        on: vi.fn(),
        close: vi.fn(),
        send: vi.fn(),
        readyState: 1,
      };
      connectionHandler(mockWs2);

      // @step Then the connection should be rejected
      expect(mockWs2.close).toHaveBeenCalledWith(
        4000,
        'Session already connected'
      );

      // @step And the first session should remain connected
      expect(getState().currentSession.ws).toBe(mockWs1);
    });
  });

  describe('Scenario: Accept new connection after session disconnect', () => {
    it('should accept new connection after previous session disconnects', () => {
      // @step Given the endpoint is running
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      startEndpoint();

      // Capture the connection handler
      const connectionHandler = mockWsOn.mock.calls.find(
        call => call[0] === 'connection'
      )?.[1];
      expect(connectionHandler).toBeDefined();

      // @step And a codelet session is connected
      const mockWs1 = {
        on: vi.fn(),
        close: vi.fn(),
        send: vi.fn(),
        readyState: 1,
      };
      connectionHandler(mockWs1);
      expect(getState().currentSession.ws).toBe(mockWs1);

      // @step When the codelet session disconnects
      // Get the close handler that was registered on mockWs1
      const closeHandler = mockWs1.on.mock.calls.find(
        call => call[0] === 'close'
      )?.[1];
      expect(closeHandler).toBeDefined();
      closeHandler();

      // @step Then the endpoint should accept new connections
      expect(getState().currentSession.ws).toBeNull();

      // @step And a new codelet session should be able to connect
      const mockWs2 = {
        on: vi.fn(),
        close: vi.fn(),
        send: vi.fn(),
        readyState: 1,
      };
      connectionHandler(mockWs2);
      expect(getState().currentSession.ws).toBe(mockWs2);
      expect(mockWs2.close).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Learn session ID from connected message', () => {
    it('should store session ID when connected message is received', () => {
      // @step Given the endpoint is running
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      startEndpoint();

      // @step And a codelet session connects via WebSocket
      const connectionHandler = mockWsOn.mock.calls.find(
        call => call[0] === 'connection'
      )?.[1];
      expect(connectionHandler).toBeDefined();

      const mockWs = {
        on: vi.fn(),
        close: vi.fn(),
        send: vi.fn(),
        readyState: 1,
      };
      connectionHandler(mockWs);
      expect(getState().currentSession.sessionId).toBeNull();

      // @step When the codelet sends a "connected" message with session_id
      const messageHandler = mockWs.on.mock.calls.find(
        call => call[0] === 'message'
      )?.[1];
      expect(messageHandler).toBeDefined();

      const connectedMessage = JSON.stringify({
        type: 'connected',
        session_id: 'abc-123-session-uuid',
        data: {},
      });
      messageHandler(connectedMessage);

      // @step Then the session ID should be stored
      expect(getState().currentSession.sessionId).toBe('abc-123-session-uuid');
    });

    it('should use learned session ID when relaying Telegram messages', () => {
      // @step Given the endpoint is running with a connected session
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      startEndpoint();

      const connectionHandler = mockWsOn.mock.calls.find(
        call => call[0] === 'connection'
      )?.[1];
      const mockWs = {
        on: vi.fn(),
        close: vi.fn(),
        send: vi.fn(),
        readyState: 1,
      };
      connectionHandler(mockWs);

      // @step And the session ID is learned from connected message
      const wsMessageHandler = mockWs.on.mock.calls.find(
        call => call[0] === 'message'
      )?.[1];
      wsMessageHandler(
        JSON.stringify({
          type: 'connected',
          session_id: 'learned-session-123',
          data: {},
        })
      );

      // @step When a Telegram message arrives
      const inbound = handleTelegramMessage('12345678', 'Hello from Telegram');

      // @step Then the inbound message should include the learned session ID
      expect(inbound.session_id).toBe('learned-session-123');
      expect(inbound.message).toBe('Hello from Telegram');
    });
  });

  // -------------------------------------------
  // Outbound: StreamChunk → Telegram
  // -------------------------------------------

  describe('Scenario: Relay text chunk to Telegram with MarkdownV2 formatting', () => {
    it('should format text chunk with MarkdownV2 and send to Telegram', async () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // (WebSocket connection simulated by state)

      // @step When the codelet sends a text chunk "Hello, I can help"
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'text', text: 'Hello, I can help' },
      };

      // @step Then the message should be formatted with MarkdownV2
      const formatted = formatForTelegram(chunk.data);
      expect(formatted).toBe('Hello, I can help');

      // @step And the message should be sent to the linked Telegram chat
      await handleStreamChunk(chunk);
      await vi.advanceTimersByTimeAsync(500);
      expect(mockBotSendMessage).toHaveBeenCalledWith(
        '12345678',
        'Hello, I can help',
        { parse_mode: 'MarkdownV2' }
      );
    });
  });

  describe('Scenario: Relay thinking chunk with emoji prefix', () => {
    it('should format thinking chunk with 💭 prefix', async () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // (WebSocket connection simulated by state)

      // @step When the codelet sends a thinking chunk "Let me analyze this..."
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'thinking', thinking: 'Let me analyze this...' },
      };

      // @step Then the message should be formatted as "💭 Let me analyze this..."
      const formatted = formatForTelegram(chunk.data);
      expect(formatted).toContain('💭');
      // Note: dots are escaped in MarkdownV2
      expect(formatted).toContain('Let me analyze this\\.\\.\\.');

      // @step And the message should be sent to the linked Telegram chat
      await handleStreamChunk(chunk);
      await vi.advanceTimersByTimeAsync(500);
      expect(mockBotSendMessage).toHaveBeenCalledWith(
        '12345678',
        expect.stringContaining('💭'),
        { parse_mode: 'MarkdownV2' }
      );
    });
  });

  describe('Scenario: Relay tool_call chunk with tool indicator', () => {
    it('should format tool_call chunk and store tool name for correlation', async () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // (WebSocket connection simulated by state)

      // @step When the codelet sends a tool_call chunk with name "Read" and id "abc123"
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'tool_call', name: 'Read', id: 'abc123' },
      };

      // @step Then the message should be formatted as "🔧 Running: Read"
      const formatted = formatForTelegram(chunk.data);
      expect(formatted).toContain('🔧');
      expect(formatted).toContain('Read');

      // @step And the message should be sent to the linked Telegram chat
      await handleStreamChunk(chunk);
      await vi.advanceTimersByTimeAsync(500);
      expect(mockBotSendMessage).toHaveBeenCalledWith(
        '12345678',
        expect.stringContaining('🔧'),
        { parse_mode: 'MarkdownV2' }
      );

      // @step And the tool name should be stored for later correlation
      expect(getState().toolNameMap.get('abc123')).toBe('Read');
    });
  });

  describe('Scenario: Relay tool_result chunk with correlated tool name', () => {
    it('should look up tool name and format result with tool indicator', async () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // (WebSocket connection simulated by state)

      // @step And a tool_call was received with name "Read" and id "abc123"
      const toolCallChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'tool_call', name: 'Read', id: 'abc123' },
      };
      await handleStreamChunk(toolCallChunk);
      await vi.advanceTimersByTimeAsync(500);
      mockBotSendMessage.mockClear();

      // @step When the codelet sends a tool_result chunk with tool_call_id "abc123" and content "file contents here"
      const resultChunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: {
          type: 'tool_result',
          tool_call_id: 'abc123',
          content: 'file contents here',
          is_error: false,
        },
      };

      // @step Then the endpoint should look up the tool name from the stored mapping
      // @step And the message should be formatted as "[Read] file contents here"
      const formatted = formatForTelegram(resultChunk.data);
      expect(formatted).toContain('\\[Read\\]');
      expect(formatted).toContain('file contents here');

      // @step And the message should be sent to the linked Telegram chat
      await handleStreamChunk(resultChunk);
      await vi.advanceTimersByTimeAsync(500);
      expect(mockBotSendMessage).toHaveBeenCalledWith(
        '12345678',
        expect.stringContaining('\\[Read\\]'),
        { parse_mode: 'MarkdownV2' }
      );
    });
  });

  describe('Scenario: Relay error chunk with error indicator', () => {
    it('should format error chunk with ❌ prefix', async () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // (WebSocket connection simulated by state)

      // @step When the codelet sends an error chunk "Connection failed"
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'error', error: 'Connection failed' },
      };

      // @step Then the message should be formatted as "❌ Error: Connection failed"
      const formatted = formatForTelegram(chunk.data);
      expect(formatted).toContain('❌');
      expect(formatted).toContain('Error');
      expect(formatted).toContain('Connection failed');

      // @step And the message should be sent to the linked Telegram chat
      await handleStreamChunk(chunk);
      expect(mockBotSendMessage).toHaveBeenCalledWith(
        '12345678',
        expect.stringContaining('❌'),
        { parse_mode: 'MarkdownV2' }
      );
    });
  });

  describe('Scenario: Display completion marker for done chunk', () => {
    it('should send ✓ marker when done chunk is received', async () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // (WebSocket connection simulated by state)

      // @step When the codelet sends a done chunk
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'done' },
      };

      // @step Then a "✓" completion marker should be sent to the linked Telegram chat
      const formatted = formatForTelegram(chunk.data);
      expect(formatted).toBe('✓');

      await handleStreamChunk(chunk);
      expect(mockBotSendMessage).toHaveBeenCalledWith('12345678', '✓', {
        parse_mode: 'MarkdownV2',
      });
    });
  });

  // -------------------------------------------
  // Message Formatting & Truncation
  // -------------------------------------------

  describe('Scenario: Truncate long messages to fit Telegram limit', () => {
    it('should truncate messages over 4096 chars preserving beginning and end', () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // Simulate WebSocket connection

      // @step When the codelet sends a text chunk with 10000 characters
      const longText = 'A'.repeat(10000);

      // @step Then the message should be truncated to fit within 4096 characters
      const truncated = truncateMessage(longText, 4096);
      expect(truncated.length).toBeLessThanOrEqual(4096);

      // @step And the first ~1500 characters should be preserved
      expect(truncated.slice(0, 100)).toBe(longText.slice(0, 100));

      // @step And a truncation indicator should be added in the middle
      expect(truncated).toContain('...');
      expect(truncated).toMatch(/\[\.\.\.\d+ chars omitted\.\.\.\]/);

      // @step And the last ~1500 characters should be preserved
      expect(truncated.slice(-100)).toBe(longText.slice(-100));
    });
  });

  describe('Scenario: Properly close code blocks when truncating mid-block', () => {
    it('should close and re-open code blocks when truncating mid-block', () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // Simulate WebSocket connection

      // @step When the codelet sends a text chunk with a code block that exceeds 4096 characters
      const longCode =
        '```javascript\n' + 'console.log("test");\n'.repeat(500) + '```';

      // @step Then the open code block should be closed before the truncation marker
      const truncated = truncateMessage(longCode, 4096);

      // Count opening and closing code block markers
      const openMarkers = (truncated.match(/```/g) || []).length;

      // @step And the code block should be re-opened after the truncation marker if needed
      // The number of ``` markers should be even (all blocks closed)
      expect(openMarkers % 2).toBe(0);

      // @step And the message should be valid MarkdownV2
      // No unclosed code blocks
    });
  });

  describe('Scenario: Preserve code block language markers', () => {
    it('should preserve language markers in code blocks', () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // Simulate WebSocket connection

      // @step When the codelet sends a text chunk containing "```python\nprint('hello')\n```"
      const codeText = "```python\nprint('hello')\n```";
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'text', text: codeText },
      };

      // @step Then the code block should be preserved with the language marker
      const formatted = formatForTelegram(chunk.data);
      expect(formatted).toContain('```python');

      // @step And the message should be formatted as valid MarkdownV2
      // MarkdownV2 escaping should be applied to special chars outside code blocks
    });
  });

  // -------------------------------------------
  // Inbound: Telegram → Codelet
  // -------------------------------------------

  describe('Scenario: Relay Telegram message to codelet as input', () => {
    it('should format Telegram message as input message and send via WebSocket', () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // Simulate WebSocket connection with session_id

      // @step When a user sends "build the app" in Telegram
      const result = handleTelegramMessage('12345678', 'build the app');

      // @step Then the endpoint should send a JSON message via WebSocket
      expect(result).toBeDefined();

      // @step And the message should have type "input"
      expect(result.type).toBe('input');

      // @step And the message should contain the session_id
      expect(result.session_id).toBeDefined();

      // @step And the message should contain "build the app"
      expect(result.message).toBe('build the app');
    });
  });

  describe('Scenario: Update active chat when user messages from different device', () => {
    it('should update active chat ID when message arrives from different chat', async () => {
      // @step Given the endpoint is running with chat ID "111"
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '111';
      startEndpoint();

      // @step And a codelet session is connected
      // (WebSocket connection simulated by state)

      // @step When a user sends a message from chat ID "222"
      handleTelegramMessage('222', 'hello from new device');

      // @step Then the active chat ID should be updated to "222"
      expect(getState().chatId).toBe('222');

      // @step And subsequent chunks should be sent to chat ID "222"
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'text', text: 'Response' },
      };
      await handleStreamChunk(chunk);
      await vi.advanceTimersByTimeAsync(500);
      expect(mockBotSendMessage).toHaveBeenCalledWith('222', 'Response', {
        parse_mode: 'MarkdownV2',
      });
    });
  });

  describe('Scenario: Route messages from multiple Telegram users to single session', () => {
    it('should route all Telegram messages to the connected session', async () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '111';
      startEndpoint();

      // @step And a codelet session is connected
      // (WebSocket connection simulated by state)

      // @step When user A sends "hello" from chat ID "111"
      const result1 = handleTelegramMessage('111', 'hello');
      expect(result1.message).toBe('hello');
      expect(result1.type).toBe('input');

      // @step And user B sends "hi there" from chat ID "222"
      const result2 = handleTelegramMessage('222', 'hi there');
      expect(result2.message).toBe('hi there');
      expect(result2.type).toBe('input');

      // @step Then both messages should be routed to the connected codelet session
      // Both have the same session_id format

      // @step And the most recent chat ID "222" should become the active chat for responses
      expect(getState().chatId).toBe('222');

      // Verify response goes to chat 222
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'text', text: 'Response' },
      };
      await handleStreamChunk(chunk);
      await vi.advanceTimersByTimeAsync(500);
      expect(mockBotSendMessage).toHaveBeenCalledWith('222', 'Response', {
        parse_mode: 'MarkdownV2',
      });
    });
  });

  // -------------------------------------------
  // Error Handling
  // -------------------------------------------

  describe('Scenario: Handle Telegram API errors gracefully', () => {
    it('should log error and continue when Telegram API fails', async () => {
      // @step Given the endpoint is running with a linked Telegram chat
      process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
      process.env.TELEGRAM_CHAT_ID = '12345678';
      startEndpoint();

      // @step And a codelet session is connected
      // (WebSocket connection simulated by state)

      // @step And the Telegram API returns an error
      const telegramError = new Error('Telegram API rate limited');
      mockBotSendMessage.mockRejectedValueOnce(telegramError);

      // Spy on console.error
      const consoleSpy = vi
        .spyOn(console, 'error')
        .mockImplementation(() => {});

      // @step When the codelet sends a text chunk "Hello"
      const chunk: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'text', text: 'Hello' },
      };

      // @step Then the error should be logged to console
      await handleStreamChunk(chunk);
      await vi.advanceTimersByTimeAsync(500);
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining('[telegram-endpoint] Telegram API error:'),
        expect.any(String)
      );

      // @step And the message should be dropped (no retry)
      // First call failed, no automatic retry

      // @step And the endpoint should continue receiving chunks
      mockBotSendMessage.mockResolvedValueOnce({});
      const chunk2: OutboundMessage = {
        type: 'chunk',
        session_id: 'test-session-uuid',
        data: { type: 'text', text: 'World' },
      };
      await handleStreamChunk(chunk2);
      await vi.advanceTimersByTimeAsync(500);
      expect(mockBotSendMessage).toHaveBeenCalledTimes(2);

      consoleSpy.mockRestore();
    });
  });

  // -------------------------------------------
  // Utility Function Tests
  // -------------------------------------------

  describe('MarkdownV2 Escaping', () => {
    it('should escape all MarkdownV2 special characters', () => {
      const specialChars = '_*[]()~`>#+-=|{}.!';
      const escaped = escapeMarkdownV2(specialChars);

      // Each special character should be escaped with backslash
      expect(escaped).toContain('\\_');
      expect(escaped).toContain('\\*');
      expect(escaped).toContain('\\[');
      expect(escaped).toContain('\\]');
      expect(escaped).toContain('\\(');
      expect(escaped).toContain('\\)');
      expect(escaped).toContain('\\~');
      expect(escaped).toContain('\\`');
      expect(escaped).toContain('\\>');
      expect(escaped).toContain('\\#');
      expect(escaped).toContain('\\+');
      expect(escaped).toContain('\\-');
      expect(escaped).toContain('\\=');
      expect(escaped).toContain('\\|');
      expect(escaped).toContain('\\{');
      expect(escaped).toContain('\\}');
      expect(escaped).toContain('\\.');
      expect(escaped).toContain('\\!');
    });

    it('should not escape characters inside code blocks', () => {
      const textWithCode =
        'Use *bold* and ```code with * and _``` with_underscores';
      const escaped = escapeMarkdownV2(textWithCode);

      // Code block content should remain unchanged
      expect(escaped).toContain('```code with * and _```');
      // But text outside should be escaped (the * in *bold* and underscores)
      expect(escaped).toContain('\\*bold\\*');
      expect(escaped).toContain('with\\_underscores');
    });
  });
});
