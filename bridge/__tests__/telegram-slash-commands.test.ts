/**
 * Feature: spec/features/telegram-slash-commands-for-agent-control.feature
 *
 * Tests for Telegram slash commands that allow users to control the agent session.
 * Commands like /help, /status, /stop, /clear are intercepted before being sent
 * to the agent and handled directly by the bridge.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import type { SlashCommandState } from '../telegram-slash-commands';
import {
  createMockBot,
  createMockWebSocket,
  createMockState,
  assertBotMessageContains,
  getBotMessage,
  type MockBot,
  type MockWebSocket,
} from './fixtures/telegram-test-helpers';

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
      assertBotMessageContains(mockBot, 'Available commands');

      // @step And the message should include "/help", "/status", "/stop", and "/clear"
      const sentMessage = getBotMessage(mockBot);
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
      assertBotMessageContains(mockBot, 'idle');
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
      assertBotMessageContains(mockBot, 'thinking');
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
      assertBotMessageContains(mockBot, 'stopped');
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
      assertBotMessageContains(mockBot, 'Nothing to stop');
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
      assertBotMessageContains(mockBot, 'cleared');
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
      assertBotMessageContains(mockBot, 'Unknown command');

      // @step And the error message should list the available commands
      const sentMessage = getBotMessage(mockBot);
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
      assertBotMessageContains(mockBot, 'Available commands');

      // @step And the message should include "/help", "/status", "/stop", and "/clear"
      const sentMessage = getBotMessage(mockBot);
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
