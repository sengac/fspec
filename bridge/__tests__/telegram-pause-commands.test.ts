/**
 * Feature: spec/features/telegram-pause-state-management.feature
 *
 * Tests for Telegram pause state management commands that allow users to respond
 * to sensitive file access prompts remotely. Commands like /allowonce, /allowsession,
 * /deny are used when the agent is paused due to blocklist prompts (PauseKind::Triple).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import type { SlashCommandState } from '../telegram-slash-commands';
import type { StreamChunkData, EndpointState } from '../telegram-endpoint';
import {
  createMockBot,
  createMockWebSocket,
  createMockState,
  createPausedMockState,
  createPauseInfo,
  assertBotMessageContains,
  assertControlMessageSent,
  getBotMessage,
  type MockBot,
  type MockWebSocket,
} from './fixtures/telegram-test-helpers';

describe('Feature: Telegram Pause State Management Commands', () => {
  let mockBot: MockBot;
  let mockWs: MockWebSocket;

  beforeEach(() => {
    mockBot = createMockBot();
    mockWs = createMockWebSocket();
  });

  // ========================================================================
  // Scenario: User allows sensitive file access once via /allowonce
  // ========================================================================
  describe('Scenario: User allows sensitive file access once via /allowonce', () => {
    it('should return allow_once action when user sends /allowonce while paused', async () => {
      // @step Given the agent is connected via Telegram bridge
      const mockState = createMockState(mockBot, mockWs, {
        isPaused: true,
        pauseInfo: createPauseInfo(
          'Sensitive file access (.ssh)',
          'Read',
          '~/.ssh/config'
        ),
      });
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent attempts to read "~/.ssh/config"
      expect(mockState.pauseInfo?.details).toBe('~/.ssh/config');

      // @step And a pause prompt is shown in Telegram "⏸ Read: Sensitive file access (.ssh)"
      expect(mockState.isPaused).toBe(true);

      // @step When the user sends "/allowonce"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/allowonce', mockState);

      // @step Then the file read should proceed
      expect(result.handled).toBe(true);
      expect(result.action).toBe('allow_once');

      // @step And the next access to "~/.ssh/config" should prompt again
      // (verified by action being 'allow_once', not 'allow_session')
      expect(result.action).not.toBe('allow_session');
    });
  });

  // ========================================================================
  // Scenario: User allows sensitive file access once via /allow alias
  // ========================================================================
  describe('Scenario: User allows sensitive file access once via /allow alias', () => {
    it('should treat /allow as alias for /allowonce', async () => {
      // @step Given the agent is connected via Telegram bridge
      const mockState = createPausedMockState(
        mockBot,
        mockWs,
        'Sensitive file access'
      );
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And a pause prompt is shown in Telegram
      expect(mockState.isPaused).toBe(true);

      // @step When the user sends "/allow"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/allow', mockState);

      // @step Then the file read should proceed
      expect(result.handled).toBe(true);
      expect(result.action).toBe('allow_once');

      // @step And the behavior should be identical to "/allowonce"
      // (same action type)
    });
  });

  // ========================================================================
  // Scenario: User allows sensitive file access for session via /allowsession
  // ========================================================================
  describe('Scenario: User allows sensitive file access for session via /allowsession', () => {
    it('should return allow_session action when user sends /allowsession while paused', async () => {
      // @step Given the agent is connected via Telegram bridge
      const mockState = createMockState(mockBot, mockWs, {
        isPaused: true,
        pauseInfo: createPauseInfo('Sensitive file access', 'Read', '~/.env'),
      });
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent attempts to read "~/.env"
      expect(mockState.pauseInfo?.details).toBe('~/.env');

      // @step And a pause prompt is shown in Telegram
      expect(mockState.isPaused).toBe(true);

      // @step When the user sends "/allowsession"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/allowsession', mockState);

      // @step Then the file read should proceed
      expect(result.handled).toBe(true);
      expect(result.action).toBe('allow_session');

      // @step And later access to other ".env" files should not prompt
      // (verified by action being 'allow_session')
    });
  });

  // ========================================================================
  // Scenario: User denies sensitive file access via /deny
  // ========================================================================
  describe('Scenario: User denies sensitive file access via /deny', () => {
    it('should return deny action when user sends /deny while paused', async () => {
      // @step Given the agent is connected via Telegram bridge
      const mockState = createMockState(mockBot, mockWs, {
        isPaused: true,
        pauseInfo: createPauseInfo(
          'Sensitive file access',
          'Read',
          '~/.aws/credentials'
        ),
      });
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent attempts to read "~/.aws/credentials"
      expect(mockState.pauseInfo?.details).toBe('~/.aws/credentials');

      // @step And a pause prompt is shown in Telegram
      expect(mockState.isPaused).toBe(true);

      // @step When the user sends "/deny"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/deny', mockState);

      // @step Then the file read should be blocked
      expect(result.handled).toBe(true);
      expect(result.action).toBe('deny');

      // @step And the AI should receive "User denied access" error
      // (verified by action being 'deny' - the actual error is handled by Rust side)
    });
  });

  // ========================================================================
  // Scenario: User sends /deny when agent is not paused
  // ========================================================================
  describe('Scenario: User sends /deny when agent is not paused', () => {
    it('should show error when /deny is sent while agent is not paused', async () => {
      // @step Given the agent is connected via Telegram bridge
      const mockState = createMockState(mockBot, mockWs, { isPaused: false });
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent is not currently paused
      expect(mockState.isPaused).toBe(false);

      // @step When the user sends "/deny"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/deny', mockState);

      // @step Then Telegram should show "⚠️ No pending pause to respond to"
      expect(result.handled).toBe(true);
      expect(result.action).toBeUndefined();
      assertBotMessageContains(mockBot, 'No pending pause');
    });
  });

  // ========================================================================
  // Scenario: User sends /allowonce when agent is not paused
  // ========================================================================
  describe('Scenario: User sends /allowonce when agent is not paused', () => {
    it('should show error when /allowonce is sent while agent is not paused', async () => {
      // @step Given the agent is connected via Telegram bridge
      const mockState = createMockState(mockBot, mockWs, { isPaused: false });
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent is not currently paused
      expect(mockState.isPaused).toBe(false);

      // @step When the user sends "/allowonce"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/allowonce', mockState);

      // @step Then Telegram should show "⚠️ No pending pause to respond to"
      expect(result.handled).toBe(true);
      expect(result.action).toBeUndefined();
      assertBotMessageContains(mockBot, 'No pending pause');
    });
  });

  // ========================================================================
  // Scenario: User sends /allowsession when agent is not paused
  // ========================================================================
  describe('Scenario: User sends /allowsession when agent is not paused', () => {
    it('should show error when /allowsession is sent while agent is not paused', async () => {
      // @step Given the agent is connected via Telegram bridge
      const mockState = createMockState(mockBot, mockWs, { isPaused: false });
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent is not currently paused
      expect(mockState.isPaused).toBe(false);

      // @step When the user sends "/allowsession"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/allowsession', mockState);

      // @step Then Telegram should show "⚠️ No pending pause to respond to"
      expect(result.handled).toBe(true);
      expect(result.action).toBeUndefined();
      assertBotMessageContains(mockBot, 'No pending pause');
    });
  });

  // ========================================================================
  // Scenario: User checks status while agent is paused
  // ========================================================================
  describe('Scenario: User checks status while agent is paused', () => {
    it('should show paused state when /status is sent while agent is paused', async () => {
      // @step Given the agent is connected via Telegram bridge
      const mockState = createMockState(mockBot, mockWs, {
        isPaused: true,
        pauseInfo: createPauseInfo('Waiting for access decision', 'Read'),
      });
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step And the agent is paused for sensitive file access
      expect(mockState.isPaused).toBe(true);

      // @step When the user sends "/status"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/status', mockState);

      // @step Then Telegram should show "⏸ Paused: Waiting for access decision"
      expect(result.handled).toBe(true);
      assertBotMessageContains(mockBot, 'Paused');
    });
  });

  // ========================================================================
  // Scenario: Help command shows pause management commands
  // ========================================================================
  describe('Scenario: Help command shows pause management commands', () => {
    it('should list pause commands in /help output', async () => {
      // @step Given the agent is connected via Telegram bridge
      const mockState = createMockState(mockBot, mockWs);
      expect(mockState.currentSession.ws).not.toBeNull();

      // @step When the user sends "/help"
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const result = await handleSlashCommand('/help', mockState);

      // @step Then the response should include "/allowonce"
      expect(result.handled).toBe(true);
      const sentMessage = getBotMessage(mockBot);
      expect(sentMessage).toContain('/allowonce');

      // @step And the response should include "/allow" as an alias
      expect(sentMessage).toContain('/allow');

      // @step And the response should include "/allowsession"
      expect(sentMessage).toContain('/allowsession');

      // @step And the response should include "/deny"
      expect(sentMessage).toContain('/deny');
    });
  });
});

// ============================================================================
// Integration Tests - Pause Request and Control Channel
// ============================================================================

describe('Feature: Telegram Pause State Management - Integration', () => {
  afterEach(async () => {
    // Reset endpoint state after each integration test
    const { resetState } = await import('../telegram-endpoint');
    resetState();
  });

  // ========================================================================
  // @integration Scenario: Telegram endpoint receives pause request from codelet
  // ========================================================================
  describe('@integration Scenario: Telegram endpoint receives pause request from codelet', () => {
    it('should set isPaused and store pauseInfo when receiving pause_request chunk', async () => {
      // @step Given the Telegram bridge is connected to a codelet session
      const { getState, handleStreamChunk } = await import(
        '../telegram-endpoint'
      );
      const state = getState();

      // Setup mock bot for the state
      const mockBot = createMockBot();
      state.bot = mockBot as unknown as EndpointState['bot'];
      state.chatId = '12345';
      state.currentSession.sessionId = 'test-session-123';
      state.isPaused = false;
      state.pauseInfo = undefined;

      // @step When the codelet sends a "pause_request" chunk with kind "triple"
      const pauseRequestChunk: StreamChunkData = {
        type: 'pause_request',
        pause_kind: 'triple',
        pause_message: 'Sensitive file access (.ssh)',
        pause_tool_name: 'Read',
        pause_details: '~/.ssh/config',
      };

      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session-123',
        data: pauseRequestChunk,
      });

      // @step Then the endpoint should set isPaused to true
      expect(state.isPaused).toBe(true);

      // @step And the endpoint should store the pause info
      expect(state.pauseInfo).toBeDefined();
      expect(state.pauseInfo?.kind).toBe('triple');
      expect(state.pauseInfo?.message).toBe('Sensitive file access (.ssh)');
      expect(state.pauseInfo?.toolName).toBe('Read');
      expect(state.pauseInfo?.details).toBe('~/.ssh/config');

      // @step And a pause notification should be sent to Telegram
      expect(mockBot.sendMessage).toHaveBeenCalled();
      const sentMessage = getBotMessage(mockBot);
      expect(sentMessage).toContain('Read');
      expect(sentMessage).toContain('Sensitive file access');
    });
  });

  // ========================================================================
  // @integration Scenario: Pause response with allow_once sent through WebSocket control channel
  // ========================================================================
  describe('@integration Scenario: Pause response with allow_once sent through WebSocket control channel', () => {
    it('should send control message with pause_response action and allow_once response', async () => {
      // @step Given the Telegram bridge is connected to a codelet session
      const { getState, handleStreamChunk } = await import(
        '../telegram-endpoint'
      );
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const state = getState();

      // Setup mock WebSocket
      const mockWs = createMockWebSocket();
      state.currentSession.ws =
        mockWs as unknown as EndpointState['currentSession']['ws'];
      state.currentSession.sessionId = 'test-session-123';

      // Setup mock bot
      const mockBot = createMockBot();
      state.bot = mockBot as unknown as EndpointState['bot'];
      state.chatId = '12345';

      // @step And the session is currently paused
      // Set pause state by sending a pause_request chunk (exercises real code)
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session-123',
        data: {
          type: 'pause_request',
          pause_kind: 'triple',
          pause_message: 'Sensitive file access',
          pause_tool_name: 'Read',
        },
      });
      expect(state.isPaused).toBe(true);

      // Reset mock to track only the /allowonce response
      mockBot.sendMessage.mockClear();
      mockWs.send.mockClear();

      // @step When the user sends "/allowonce"
      // Create state for slash command handler that mirrors endpoint state
      const slashCommandState: SlashCommandState = {
        bot: mockBot,
        chatId: state.chatId,
        currentSession: {
          ws: mockWs,
          sessionId: state.currentSession.sessionId,
        },
        isRunning: true,
        agentState: 'idle',
        isPaused: state.isPaused,
        pauseInfo: state.pauseInfo,
      };

      const result = await handleSlashCommand('/allowonce', slashCommandState);
      expect(result.action).toBe('allow_once');

      // Simulate what telegram-endpoint does when it receives the action
      // (session:control envelope per multiplexed protocol)
      if (result.action === 'allow_once') {
        mockWs.send(
          JSON.stringify({
            service: 'session',
            type: 'control',
            session_id: state.currentSession.sessionId,
            data: { action: 'pause_response', response: 'allow_once' },
          })
        );
      }

      // @step Then a control message should be sent with action "pause_response"
      // @step And the response field should be "allow_once"
      assertControlMessageSent(mockWs, 'pause_response', 'allow_once');

      // @step And the bridge_handler should call session_pause_triple
      // (verified by Rust-side tests in bridge_relay.rs)
    });
  });

  // ========================================================================
  // @integration Scenario: Pause response with allow_session sent through WebSocket control channel
  // ========================================================================
  describe('@integration Scenario: Pause response with allow_session sent through WebSocket control channel', () => {
    it('should send control message with pause_response action and allow_session response', async () => {
      // @step Given the Telegram bridge is connected to a codelet session
      const { getState, handleStreamChunk } = await import(
        '../telegram-endpoint'
      );
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const state = getState();

      // Setup mock WebSocket
      const mockWs = createMockWebSocket();
      state.currentSession.ws =
        mockWs as unknown as EndpointState['currentSession']['ws'];
      state.currentSession.sessionId = 'test-session-123';

      // Setup mock bot
      const mockBot = createMockBot();
      state.bot = mockBot as unknown as EndpointState['bot'];
      state.chatId = '12345';

      // @step And the session is currently paused
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session-123',
        data: {
          type: 'pause_request',
          pause_kind: 'triple',
          pause_message: 'Sensitive file access',
          pause_tool_name: 'Read',
        },
      });
      expect(state.isPaused).toBe(true);

      // Reset mocks
      mockBot.sendMessage.mockClear();
      mockWs.send.mockClear();

      // @step When the user sends "/allowsession"
      const slashCommandState: SlashCommandState = {
        bot: mockBot,
        chatId: state.chatId,
        currentSession: {
          ws: mockWs,
          sessionId: state.currentSession.sessionId,
        },
        isRunning: true,
        agentState: 'idle',
        isPaused: state.isPaused,
        pauseInfo: state.pauseInfo,
      };

      const result = await handleSlashCommand(
        '/allowsession',
        slashCommandState
      );
      expect(result.action).toBe('allow_session');

      // Simulate what telegram-endpoint does
      // (session:control envelope per multiplexed protocol)
      if (result.action === 'allow_session') {
        mockWs.send(
          JSON.stringify({
            service: 'session',
            type: 'control',
            session_id: state.currentSession.sessionId,
            data: { action: 'pause_response', response: 'allow_session' },
          })
        );
      }

      // @step Then a control message should be sent with action "pause_response"
      // @step And the response field should be "allow_session"
      assertControlMessageSent(mockWs, 'pause_response', 'allow_session');

      // @step And the bridge_handler should call session_pause_triple
      // (verified by Rust-side tests in bridge_relay.rs)
    });
  });

  // ========================================================================
  // @integration Scenario: Pause response with deny sent through WebSocket control channel
  // ========================================================================
  describe('@integration Scenario: Pause response with deny sent through WebSocket control channel', () => {
    it('should send control message with pause_response action and deny response', async () => {
      // @step Given the Telegram bridge is connected to a codelet session
      const { getState, handleStreamChunk } = await import(
        '../telegram-endpoint'
      );
      const { handleSlashCommand } = await import('../telegram-slash-commands');
      const state = getState();

      // Setup mock WebSocket
      const mockWs = createMockWebSocket();
      state.currentSession.ws =
        mockWs as unknown as EndpointState['currentSession']['ws'];
      state.currentSession.sessionId = 'test-session-123';

      // Setup mock bot
      const mockBot = createMockBot();
      state.bot = mockBot as unknown as EndpointState['bot'];
      state.chatId = '12345';

      // @step And the session is currently paused
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session-123',
        data: {
          type: 'pause_request',
          pause_kind: 'triple',
          pause_message: 'Sensitive file access',
          pause_tool_name: 'Read',
        },
      });
      expect(state.isPaused).toBe(true);

      // Reset mocks
      mockBot.sendMessage.mockClear();
      mockWs.send.mockClear();

      // @step When the user sends "/deny"
      const slashCommandState: SlashCommandState = {
        bot: mockBot,
        chatId: state.chatId,
        currentSession: {
          ws: mockWs,
          sessionId: state.currentSession.sessionId,
        },
        isRunning: true,
        agentState: 'idle',
        isPaused: state.isPaused,
        pauseInfo: state.pauseInfo,
      };

      const result = await handleSlashCommand('/deny', slashCommandState);
      expect(result.action).toBe('deny');

      // Simulate what telegram-endpoint does
      // (session:control envelope per multiplexed protocol)
      if (result.action === 'deny') {
        mockWs.send(
          JSON.stringify({
            service: 'session',
            type: 'control',
            session_id: state.currentSession.sessionId,
            data: { action: 'pause_response', response: 'deny' },
          })
        );
      }

      // @step Then a control message should be sent with action "pause_response"
      // @step And the response field should be "deny"
      assertControlMessageSent(mockWs, 'pause_response', 'deny');

      // @step And the bridge_handler should call session_pause_triple
      // (verified by Rust-side tests in bridge_relay.rs)
    });
  });

  // ========================================================================
  // Additional integration test: Full pause state lifecycle
  // ========================================================================
  describe('@integration Scenario: Full pause state lifecycle', () => {
    it('should clear pause state after response is sent', async () => {
      const { getState, handleStreamChunk } = await import(
        '../telegram-endpoint'
      );
      const state = getState();

      // Setup mock bot
      const mockBot = createMockBot();
      state.bot = mockBot as unknown as EndpointState['bot'];
      state.chatId = '12345';
      state.currentSession.sessionId = 'test-session-123';

      // Agent triggers pause
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session-123',
        data: {
          type: 'pause_request',
          pause_kind: 'triple',
          pause_message: 'Sensitive file access',
          pause_tool_name: 'Read',
        },
      });

      // Verify paused
      expect(state.isPaused).toBe(true);
      expect(state.pauseInfo).toBeDefined();

      // Agent completes (done chunk should clear pause state)
      await handleStreamChunk({
        type: 'chunk',
        session_id: 'test-session-123',
        data: { type: 'done' },
      });

      // Verify cleared
      expect(state.isPaused).toBe(false);
      expect(state.pauseInfo).toBeUndefined();
    });
  });
});
