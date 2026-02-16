/**
 * Feature: spec/features/telegram-slash-commands-for-agent-control.feature
 *
 * Tests for Telegram slash commands that allow users to control the agent session.
 * Commands like /help, /status, /stop, /clear are intercepted before being sent
 * to the agent and handled directly by the bridge.
 */

import { describe, it, expect, beforeEach, vi, type Mock } from 'vitest';
import type {
  SlashCommandState,
  MinimalBot,
  MinimalWebSocket,
} from '../telegram-slash-commands';

// WebSocket constants
const WS_OPEN = 1;

/**
 * Mock bot that satisfies MinimalBot interface with testable mock functions.
 * Uses intersection type to preserve both interface compliance and mock access.
 */
interface MockBot extends MinimalBot {
  sendMessage: Mock<
    [chatId: string | number, text: string, options?: { parse_mode?: string }],
    Promise<unknown>
  >;
}

/**
 * Mock WebSocket that satisfies MinimalWebSocket interface with testable mock functions.
 */
interface MockWebSocket extends MinimalWebSocket {
  send: Mock<[data: string], void>;
}

/**
 * Create a properly typed mock bot for testing
 */
function createMockBot(): MockBot {
  return {
    sendMessage: vi.fn().mockResolvedValue(undefined),
  };
}

/**
 * Create a properly typed mock WebSocket for testing
 */
function createMockWebSocket(): MockWebSocket {
  return {
    readyState: WS_OPEN,
    send: vi.fn(),
  };
}

/**
 * Create a properly typed mock state for testing
 */
function createMockState(bot: MockBot, ws: MockWebSocket): SlashCommandState {
  return {
    bot,
    chatId: '12345',
    currentSession: {
      ws,
      sessionId: 'test-session-123',
    },
    isRunning: true,
    agentState: 'idle',
  };
}

describe('Feature: Telegram Slash Commands for Agent Control', () => {
  let mockBot: MockBot;
  let mockWs: MockWebSocket;
  let mockState: SlashCommandState;

  beforeEach(() => {
    mockBot = createMockBot();
    mockWs = createMockWebSocket();
    mockState = createMockState(mockBot, mockWs);
  });

  describe('Scenario: Show available commands with /help', () => {
    it('should show available commands when user sends /help', async () => {
      // @step Given the Telegram bridge is connected to a session
      expect(mockState.currentSession.ws).not.toBeNull();
      expect(mockState.currentSession.sessionId).toBe('test-session-123');

      // @step When I send "/help"
      const text = '/help';
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand(text, mockState);

      // @step Then I should receive a message listing all available commands
      expect(result.handled).toBe(true);
      expect(mockBot.sendMessage).toHaveBeenCalledWith(
        '12345',
        expect.stringContaining('Available commands'),
        expect.any(Object)
      );

      // @step And the message should include "/help", "/status", "/stop", and "/clear"
      const sentMessage = mockBot.sendMessage.mock.calls[0][1] as string;
      expect(sentMessage).toContain('/help');
      expect(sentMessage).toContain('/status');
      expect(sentMessage).toContain('/stop');
      expect(sentMessage).toContain('/clear');
    });
  });

  describe('Scenario: Check status when agent is idle', () => {
    it('should show idle status when agent is idle', async () => {
      // @step Given the Telegram bridge is connected to a session
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent is idle
      mockState.agentState = 'idle';

      // @step When I send "/status"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/status', mockState);

      // @step Then I should receive a message saying "Agent is idle"
      expect(result.handled).toBe(true);
      expect(mockBot.sendMessage).toHaveBeenCalledWith(
        '12345',
        expect.stringContaining('idle'),
        expect.any(Object)
      );
    });
  });

  describe('Scenario: Check status when agent is processing', () => {
    it('should show thinking status when agent is processing', async () => {
      // @step Given the Telegram bridge is connected to a session
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent is thinking
      mockState.agentState = 'thinking';

      // @step When I send "/status"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/status', mockState);

      // @step Then I should receive a message saying "Agent is thinking..."
      expect(result.handled).toBe(true);
      expect(mockBot.sendMessage).toHaveBeenCalledWith(
        '12345',
        expect.stringContaining('thinking'),
        expect.any(Object)
      );
    });
  });

  describe('Scenario: Stop agent when it is running', () => {
    it('should stop agent when user sends /stop while agent is running', async () => {
      // @step Given the Telegram bridge is connected to a session
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent is executing an operation
      mockState.agentState = 'executing';

      // @step When I send "/stop"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/stop', mockState);

      // @step Then the agent operation should be interrupted
      expect(result.handled).toBe(true);
      expect(result.action).toBe('stop');

      // @step And I should receive confirmation that the operation was stopped
      expect(mockBot.sendMessage).toHaveBeenCalledWith(
        '12345',
        expect.stringContaining('stopped'),
        expect.any(Object)
      );
    });
  });

  describe('Scenario: Stop agent when it is already idle', () => {
    it('should show nothing to stop when agent is already idle', async () => {
      // @step Given the Telegram bridge is connected to a session
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent is idle
      mockState.agentState = 'idle';

      // @step When I send "/stop"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/stop', mockState);

      // @step Then I should receive a message saying "Nothing to stop"
      expect(result.handled).toBe(true);
      expect(mockBot.sendMessage).toHaveBeenCalledWith(
        '12345',
        expect.stringContaining('Nothing to stop'),
        expect.any(Object)
      );
    });
  });

  describe('Scenario: Clear conversation history', () => {
    it('should clear conversation history when user sends /clear', async () => {
      // @step Given the Telegram bridge is connected to a session
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the session has conversation history
      // (implicitly true - we have an active session)
      expect(mockState.currentSession.sessionId).toBeTruthy();

      // @step When I send "/clear"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/clear', mockState);

      // @step Then the conversation history should be cleared
      expect(result.handled).toBe(true);
      expect(result.action).toBe('clear');

      // @step And the session should be reset
      // (verified by action type)

      // @step And I should receive confirmation that the session was cleared
      expect(mockBot.sendMessage).toHaveBeenCalledWith(
        '12345',
        expect.stringContaining('cleared'),
        expect.any(Object)
      );
    });
  });

  describe('Scenario: Handle unknown slash command', () => {
    it('should show error for unknown slash commands', async () => {
      // @step Given the Telegram bridge is connected to a session
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step When I send "/unknown"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/unknown', mockState);

      // @step Then I should receive an error message
      expect(result.handled).toBe(true);
      expect(mockBot.sendMessage).toHaveBeenCalledWith(
        '12345',
        expect.stringContaining('Unknown command'),
        expect.any(Object)
      );

      // @step And the error message should list the available commands
      const sentMessage = mockBot.sendMessage.mock.calls[0][1] as string;
      expect(sentMessage).toContain('/help');
      expect(sentMessage).toContain('/status');
      expect(sentMessage).toContain('/stop');
      expect(sentMessage).toContain('/clear');
    });
  });

  describe('Scenario: Slash commands are case-insensitive', () => {
    it('should handle uppercase /HELP the same as /help', async () => {
      // @step Given the Telegram bridge is connected to a session
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step When I send "/HELP"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/HELP', mockState);

      // @step Then I should receive a message listing all available commands
      expect(result.handled).toBe(true);
      expect(mockBot.sendMessage).toHaveBeenCalledWith(
        '12345',
        expect.stringContaining('Available commands'),
        expect.any(Object)
      );

      // @step And the message should include "/help", "/status", "/stop", and "/clear"
      const sentMessage = mockBot.sendMessage.mock.calls[0][1] as string;
      expect(sentMessage).toContain('/help');
      expect(sentMessage).toContain('/status');
      expect(sentMessage).toContain('/stop');
      expect(sentMessage).toContain('/clear');
    });
  });

  describe('Scenario: Slash commands are not forwarded to the agent', () => {
    it('should not forward slash commands to the agent session', async () => {
      // @step Given the Telegram bridge is connected to a session
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step When I send "/help"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/help', mockState);

      // @step Then the message should NOT be forwarded to the agent session
      expect(result.handled).toBe(true);
      expect(mockWs.send).not.toHaveBeenCalled();

      // @step And the response should come directly from the bridge
      expect(mockBot.sendMessage).toHaveBeenCalled();
    });
  });

  describe('Non-slash messages', () => {
    it('should not handle regular messages', async () => {
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('Hello world', mockState);

      expect(result.handled).toBe(false);
      expect(mockBot.sendMessage).not.toHaveBeenCalled();
    });
  });
});
