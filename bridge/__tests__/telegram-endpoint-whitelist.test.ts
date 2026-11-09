/**
 * Feature: spec/features/telegram-user-whitelist.feature
 *
 * Endpoint-level tests for the Telegram user ID whitelist: they drive the real
 * startEndpoint() message handler with a mocked bot and WebSocket server.
 * The pure whitelist helpers are covered by telegram-whitelist.test.ts.
 * Moved out of telegram-endpoint.test.ts so each test file maps one feature (COV-075).
 *
 * BRIDGE-009: User ID Whitelist for Telegram Bridge
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

// Import functions from implementation
import {
  startEndpoint,
  stopEndpoint,
  resetState,
  getState,
} from '../telegram-endpoint';

describe('Telegram endpoint with a user ID whitelist', () => {
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
    delete process.env.TELEGRAM_ALLOWED_USER_IDS;
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
  // User ID Whitelist (BRIDGE-009)
  // -------------------------------------------

  describe('Feature: User ID Whitelist for Telegram Bridge', () => {
    describe('Scenario: Authorized user message is forwarded to rust', () => {
      it('should forward message when user ID is in whitelist', () => {
        // @step Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "123456789"
        process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
        process.env.TELEGRAM_ALLOWED_USER_IDS = '123456789';
        startEndpoint();

        // @step And the endpoint is running with a connected codelet session
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

        // Simulate session connected message
        const wsMessageHandler = mockWs.on.mock.calls.find(
          call => call[0] === 'message'
        )?.[1];
        wsMessageHandler(
          JSON.stringify({
            type: 'connected',
            session_id: 'test-session',
            data: {},
          })
        );

        // @step When a Telegram message arrives from user ID 123456789
        // Simulate Telegram message event
        const telegramHandler = mockBotOn.mock.calls.find(
          call => call[0] === 'message'
        )?.[1];
        expect(telegramHandler).toBeDefined();

        telegramHandler({
          chat: { id: 12345678 },
          from: { id: 123456789, first_name: 'Test', is_bot: false },
          text: 'Hello from authorized user',
        });

        // @step Then the message should be forwarded to the codelet session
        expect(mockWs.send).toHaveBeenCalledWith(
          expect.stringContaining('Hello from authorized user')
        );
      });
    });

    describe('Scenario: Unauthorized user message is dropped silently', () => {
      it('should drop message and log when user ID is not in whitelist', () => {
        // @step Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "123456789"
        process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
        process.env.TELEGRAM_ALLOWED_USER_IDS = '123456789';

        const consoleSpy = vi
          .spyOn(console, 'log')
          .mockImplementation(() => {});
        startEndpoint();

        // @step And the endpoint is running with a connected codelet session
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

        // @step When a Telegram message arrives from user ID 999999999
        const telegramHandler = mockBotOn.mock.calls.find(
          call => call[0] === 'message'
        )?.[1];
        expect(telegramHandler).toBeDefined();

        telegramHandler({
          chat: { id: 12345678 },
          from: { id: 999999999, first_name: 'Unauthorized', is_bot: false },
          text: 'Hello from unauthorized user',
        });

        // @step Then the message should not be forwarded to the codelet session
        expect(mockWs.send).not.toHaveBeenCalled();

        // @step And the log should contain "unauthorized user: 999999999"
        expect(consoleSpy).toHaveBeenCalledWith(
          expect.stringContaining('unauthorized user: 999999999')
        );

        consoleSpy.mockRestore();
      });
    });

    describe('Scenario: Multiple user IDs can be whitelisted', () => {
      it('should allow any user in comma-separated whitelist', () => {
        // @step Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "111,222,333"
        process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
        process.env.TELEGRAM_ALLOWED_USER_IDS = '111,222,333';
        startEndpoint();

        // @step And the endpoint is running with a connected codelet session
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

        // @step When a Telegram message arrives from user ID 222
        const telegramHandler = mockBotOn.mock.calls.find(
          call => call[0] === 'message'
        )?.[1];
        expect(telegramHandler).toBeDefined();

        telegramHandler({
          chat: { id: 12345678 },
          from: { id: 222, first_name: 'User222', is_bot: false },
          text: 'Hello from user 222',
        });

        // @step Then the message should be forwarded to the codelet session
        expect(mockWs.send).toHaveBeenCalledWith(
          expect.stringContaining('Hello from user 222')
        );
      });
    });

    describe('Scenario: No whitelist configured allows all users', () => {
      it('should allow all users when TELEGRAM_ALLOWED_USER_IDS is not set', () => {
        // @step Given the endpoint is configured without TELEGRAM_ALLOWED_USER_IDS
        process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
        delete process.env.TELEGRAM_ALLOWED_USER_IDS;
        startEndpoint();

        // @step And the endpoint is running with a connected codelet session
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

        // @step When a Telegram message arrives from user ID 999999999
        const telegramHandler = mockBotOn.mock.calls.find(
          call => call[0] === 'message'
        )?.[1];
        expect(telegramHandler).toBeDefined();

        telegramHandler({
          chat: { id: 12345678 },
          from: { id: 999999999, first_name: 'AnyUser', is_bot: false },
          text: 'Hello from any user',
        });

        // @step Then the message should be forwarded to the codelet session
        expect(mockWs.send).toHaveBeenCalledWith(
          expect.stringContaining('Hello from any user')
        );
      });
    });

    describe('Scenario: Message without from field is dropped when whitelist active', () => {
      it('should drop message when msg.from is undefined and whitelist is active', () => {
        // @step Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "123456789"
        process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
        process.env.TELEGRAM_ALLOWED_USER_IDS = '123456789';

        const consoleSpy = vi
          .spyOn(console, 'log')
          .mockImplementation(() => {});
        startEndpoint();

        // @step And the endpoint is running with a connected codelet session
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

        // @step When a Telegram message arrives without a from field
        const telegramHandler = mockBotOn.mock.calls.find(
          call => call[0] === 'message'
        )?.[1];
        expect(telegramHandler).toBeDefined();

        // Channel post or system message without 'from' field
        telegramHandler({
          chat: { id: 12345678 },
          // No 'from' field
          text: 'Channel announcement',
        });

        // @step Then the message should not be forwarded to the codelet session
        expect(mockWs.send).not.toHaveBeenCalled();

        // @step And the log should contain "no user ID"
        expect(consoleSpy).toHaveBeenCalledWith(
          expect.stringContaining('no user ID')
        );

        consoleSpy.mockRestore();
      });
    });

    describe('Scenario: Invalid user IDs in environment variable are filtered out', () => {
      it('should parse only valid numeric IDs from whitelist', () => {
        // @step Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "abc,456,xyz"
        process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
        process.env.TELEGRAM_ALLOWED_USER_IDS = 'abc,456,xyz';
        startEndpoint();

        // @step And the endpoint is running with a connected codelet session
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

        // @step When a Telegram message arrives from user ID 456
        const telegramHandler = mockBotOn.mock.calls.find(
          call => call[0] === 'message'
        )?.[1];
        expect(telegramHandler).toBeDefined();

        telegramHandler({
          chat: { id: 12345678 },
          from: { id: 456, first_name: 'User456', is_bot: false },
          text: 'Hello from user 456',
        });

        // @step Then the message should be forwarded to the codelet session
        expect(mockWs.send).toHaveBeenCalledWith(
          expect.stringContaining('Hello from user 456')
        );

        // Verify non-numeric IDs were filtered (user abc should not work)
        mockWs.send.mockClear();
        telegramHandler({
          chat: { id: 12345678 },
          from: { id: 789, first_name: 'User789', is_bot: false },
          text: 'Hello from user 789',
        });
        expect(mockWs.send).not.toHaveBeenCalled();
      });
    });

    describe('Scenario: Startup logs whitelist enabled message', () => {
      it('should log whitelist count on startup when configured', () => {
        // @step Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "111,222,333"
        process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
        process.env.TELEGRAM_ALLOWED_USER_IDS = '111,222,333';

        const consoleSpy = vi
          .spyOn(console, 'log')
          .mockImplementation(() => {});

        // @step When the endpoint starts up
        startEndpoint();

        // @step Then the log should contain "User whitelist enabled: 3 user(s)"
        expect(consoleSpy).toHaveBeenCalledWith(
          expect.stringContaining('User whitelist enabled: 3 user(s)')
        );

        consoleSpy.mockRestore();
      });
    });

    describe('Scenario: Startup logs no whitelist message', () => {
      it('should log no whitelist message on startup when not configured', () => {
        // @step Given the endpoint is configured without TELEGRAM_ALLOWED_USER_IDS
        process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
        delete process.env.TELEGRAM_ALLOWED_USER_IDS;

        const consoleSpy = vi
          .spyOn(console, 'log')
          .mockImplementation(() => {});

        // @step When the endpoint starts up
        startEndpoint();

        // @step Then the log should contain "No user whitelist configured - accepting all users"
        expect(consoleSpy).toHaveBeenCalledWith(
          expect.stringContaining(
            'No user whitelist configured - accepting all users'
          )
        );

        consoleSpy.mockRestore();
      });
    });
  });
});
