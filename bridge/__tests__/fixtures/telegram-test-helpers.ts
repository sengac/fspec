/**
 * Shared test fixtures for Telegram bridge tests.
 *
 * These helpers provide properly-typed mock objects for testing
 * Telegram slash commands and endpoint functionality.
 *
 * DRY: Extracted from telegram-slash-commands.test.ts, telegram-pause-commands.test.ts,
 * and stop-command-idle-detection.test.ts to avoid duplication.
 */

import { vi, expect, type Mock } from 'vitest';
import type {
  SlashCommandState,
  MinimalBot,
  MinimalWebSocket,
  AgentState,
  PauseInfo,
} from '../../telegram-slash-commands';

// ============================================================================
// WebSocket Constants
// ============================================================================

/** WebSocket.OPEN constant */
export const WS_OPEN = 1;

// ============================================================================
// Mock Interfaces
// ============================================================================

/**
 * Mock bot that satisfies MinimalBot interface with testable mock functions.
 * Uses intersection type to preserve both interface compliance and mock access.
 */
export interface MockBot extends MinimalBot {
  sendMessage: Mock &
    ((
      chatId: string | number,
      text: string,
      options?: { parse_mode?: string }
    ) => Promise<unknown>);
}

/**
 * Mock WebSocket that satisfies MinimalWebSocket interface with testable mock functions.
 */
export interface MockWebSocket extends MinimalWebSocket {
  send: Mock & ((data: string) => void);
}

// ============================================================================
// Options Interfaces
// ============================================================================

/**
 * Options for creating mock state with pause support (BRIDGE-014).
 */
export interface MockStateOptions {
  /** Agent state: idle, thinking, or executing */
  agentState?: AgentState;
  /** Whether agent is paused waiting for access decision */
  isPaused?: boolean;
  /** Information about the current pause prompt */
  pauseInfo?: PauseInfo;
  /** Chat ID (defaults to '12345') */
  chatId?: string;
  /** Session ID (defaults to 'test-session-123') */
  sessionId?: string;
  /** Whether session is running (defaults to true) */
  isRunning?: boolean;
}

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a properly typed mock bot for testing.
 * The mock bot satisfies MinimalBot interface and allows verification of calls.
 */
export function createMockBot(): MockBot {
  return {
    sendMessage: vi.fn().mockResolvedValue(undefined),
  };
}

/**
 * Create a properly typed mock WebSocket for testing.
 * The mock WebSocket satisfies MinimalWebSocket interface and allows verification of calls.
 */
export function createMockWebSocket(): MockWebSocket {
  return {
    readyState: WS_OPEN,
    send: vi.fn(),
  };
}

/**
 * Create a properly typed mock state for testing.
 * Supports all configuration options including pause state for BRIDGE-014.
 *
 * @param bot - Mock bot instance
 * @param ws - Mock WebSocket instance
 * @param options - Optional configuration for the state
 * @returns SlashCommandState configured for testing
 */
export function createMockState(
  bot: MockBot,
  ws: MockWebSocket,
  options?: MockStateOptions
): SlashCommandState {
  return {
    bot,
    chatId: options?.chatId ?? '12345',
    currentSession: {
      ws,
      sessionId: options?.sessionId ?? 'test-session-123',
    },
    isRunning: options?.isRunning ?? true,
    agentState: options?.agentState ?? 'idle',
    // BRIDGE-014: Pause state fields
    isPaused: options?.isPaused ?? false,
    pauseInfo: options?.pauseInfo,
  };
}

// ============================================================================
// Pause State Helpers (BRIDGE-014)
// ============================================================================

/**
 * Create a pause info object for testing sensitive file access prompts.
 *
 * @param message - The pause message (e.g., 'Sensitive file access (.ssh)')
 * @param toolName - The tool that triggered the pause (e.g., 'Read')
 * @param details - Additional details (e.g., '~/.ssh/config')
 * @returns PauseInfo object for use in mock state
 */
export function createPauseInfo(
  message: string,
  toolName?: string,
  details?: string
): PauseInfo {
  return {
    kind: 'triple',
    message,
    toolName,
    details,
  };
}

/**
 * Create a mock state that is paused and waiting for access decision.
 * Convenience function for BRIDGE-014 pause state tests.
 *
 * @param bot - Mock bot instance
 * @param ws - Mock WebSocket instance
 * @param pauseMessage - The pause message to display
 * @param pauseDetails - Additional pause details (file path, etc.)
 * @returns SlashCommandState configured in paused state
 */
export function createPausedMockState(
  bot: MockBot,
  ws: MockWebSocket,
  pauseMessage: string = 'Sensitive file access',
  pauseDetails?: string
): SlashCommandState {
  return createMockState(bot, ws, {
    isPaused: true,
    pauseInfo: createPauseInfo(pauseMessage, 'Read', pauseDetails),
  });
}

// ============================================================================
// WebSocket Message Helpers
// ============================================================================

/**
 * Parse a JSON message sent to a mock WebSocket.
 * Useful for verifying control messages sent via ws.send().
 *
 * @param mockWs - The mock WebSocket to get messages from
 * @param callIndex - Which call to parse (defaults to 0, the first call)
 * @returns Parsed JSON object from the sent message
 */
export function getWebSocketMessage(
  mockWs: MockWebSocket,
  callIndex: number = 0
): Record<string, unknown> {
  const calls = mockWs.send.mock.calls;
  if (callIndex >= calls.length) {
    throw new Error(
      `WebSocket.send was called ${calls.length} times, but tried to access call ${callIndex}`
    );
  }
  return JSON.parse(calls[callIndex][0]) as Record<string, unknown>;
}

/**
 * Assert that a session:control envelope was sent with the expected action.
 * Convenience function for testing the multiplexed control channel.
 *
 * Expected envelope shape:
 *   { service: "session", type: "control", session_id: "...",
 *     data: { action: "...", response?: "..." } }
 *
 * @param mockWs - The mock WebSocket to check
 * @param expectedAction - The expected action field value inside `data`
 * @param expectedResponse - Optional expected response field value (for pause_response)
 */
export function assertControlMessageSent(
  mockWs: MockWebSocket,
  expectedAction: string,
  expectedResponse?: string
): void {
  expect(mockWs.send).toHaveBeenCalled();
  const message = getWebSocketMessage(mockWs);
  expect(message.service).toBe('session');
  expect(message.type).toBe('control');
  const data = message.data as { action?: string; response?: string };
  expect(data).toBeDefined();
  expect(data.action).toBe(expectedAction);
  if (expectedResponse !== undefined) {
    expect(data.response).toBe(expectedResponse);
  }
}

/**
 * Assert that the bot sent a message containing expected text.
 * Convenience function for verifying Telegram bot responses.
 *
 * @param mockBot - The mock bot to check
 * @param expectedText - Text that should be contained in the message
 * @param callIndex - Which call to check (defaults to 0, the first call)
 */
export function assertBotMessageContains(
  mockBot: MockBot,
  expectedText: string,
  callIndex: number = 0
): void {
  expect(mockBot.sendMessage).toHaveBeenCalled();
  const calls = mockBot.sendMessage.mock.calls;
  if (callIndex >= calls.length) {
    throw new Error(
      `Bot.sendMessage was called ${calls.length} times, but tried to access call ${callIndex}`
    );
  }
  const sentMessage = calls[callIndex][1] as string;
  expect(sentMessage).toContain(expectedText);
}

/**
 * Get the message text sent by the bot.
 *
 * @param mockBot - The mock bot to get the message from
 * @param callIndex - Which call to get (defaults to 0, the first call)
 * @returns The message text sent by the bot
 */
export function getBotMessage(mockBot: MockBot, callIndex: number = 0): string {
  const calls = mockBot.sendMessage.mock.calls;
  if (callIndex >= calls.length) {
    throw new Error(
      `Bot.sendMessage was called ${calls.length} times, but tried to access call ${callIndex}`
    );
  }
  return calls[callIndex][1] as string;
}
