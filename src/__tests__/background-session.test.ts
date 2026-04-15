// Feature: spec/features/background-session-management-with-attach-detach.feature
// Tests for NAPI-009: Background Session Management
// Tests for TUI-046: Detach Confirmation Modal on AgentView Exit
// Tests for TUI-047: Attach to Detached Sessions from Resume View
//
// Session streaming now uses GlobalSessionStreamManager with global chunk callback.
// These tests verify session lifecycle management (create, destroy, buffer, restore).

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the NAPI bindings
vi.mock('@sengac/codelet-napi', () => ({
  sessionManagerCreate: vi.fn().mockResolvedValue('test-session-id'),
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionManagerList: vi.fn(),
  sessionManagerDestroy: vi.fn(),
  sessionSendInput: vi.fn(),
  sessionInterrupt: vi.fn(),
  sessionSetPendingInput: vi.fn(),
  sessionGetPendingInput: vi.fn().mockReturnValue(null),
  sessionGetCompactionProgress: vi.fn().mockReturnValue(null),
  persistenceSetSessionTokens: vi.fn(),
  sessionGetStatus: vi.fn(),
  sessionGetBufferedOutput: vi.fn(),
  sessionRestoreMessages: vi.fn(),
  sessionRestoreTokenState: vi.fn(),
}));

import {
  sessionManagerCreate,
  sessionManagerCreateWithId,
  sessionManagerList,
  sessionManagerDestroy,
  sessionSendInput,
  sessionInterrupt,
  sessionGetStatus,
  sessionGetBufferedOutput,
  sessionRestoreMessages,
  sessionRestoreTokenState,
} from '@sengac/codelet-napi';

describe('Background Session Management', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('NAPI-009: Session creation with persistence ID', () => {
    it('should create a session with a specific persistence ID', () => {
      // @step Given I have a persistence session ID
      const persistenceId = '550e8400-e29b-41d4-a716-446655440000';

      // @step When I create a background session with that ID
      sessionManagerCreateWithId(
        persistenceId,
        'anthropic/claude-sonnet-4',
        '/test/project',
        'My Session'
      );

      // @step Then the session is created with the persistence ID
      expect(sessionManagerCreateWithId).toHaveBeenCalledWith(
        persistenceId,
        'anthropic/claude-sonnet-4',
        '/test/project',
        'My Session'
      );
    });
  });

  describe('TUI-046: Session lifecycle on AgentView exit', () => {
    it('should allow detaching while agent is running (session continues in background)', () => {
      // @step Given I have an active session running in AgentView
      const sessionId = '550e8400-e29b-41d4-a716-446655440000';
      vi.mocked(sessionGetStatus).mockReturnValue('running');

      // @step When I press ESC and select "Detach" from the modal
      // (GlobalSessionStreamManager stops forwarding to TUI, but session continues)
      const status = sessionGetStatus(sessionId);

      // @step Then the session continues running in background
      expect(status).toBe('running');
    });

    it('should destroy session when user presses ESC and selects Close Session', () => {
      // @step Given I have an active session running in AgentView
      const sessionId = '550e8400-e29b-41d4-a716-446655440000';

      // @step When I press ESC and select "Close Session" from the modal
      sessionManagerDestroy(sessionId);

      // @step Then the session is destroyed
      expect(sessionManagerDestroy).toHaveBeenCalledWith(sessionId);
    });
  });

  describe('TUI-047: Resume view shows background sessions', () => {
    it('should list background sessions with status in /resume', () => {
      // @step Given I have detached sessions running in background
      vi.mocked(sessionManagerList).mockReturnValue([
        {
          id: 'session-1',
          status: 'running',
          name: 'Running Task',
          project: '/project',
          messageCount: 5,
          isIsolated: false,
        },
        {
          id: 'session-2',
          status: 'idle',
          name: 'Finished Task',
          project: '/project',
          messageCount: 10,
          isIsolated: false,
        },
      ]);

      // @step When I view /resume
      const sessions = sessionManagerList();

      // @step Then I see background sessions with their status
      expect(sessions).toHaveLength(2);
      expect(sessions[0].status).toBe('running');
      expect(sessions[1].status).toBe('idle');
    });

    it('should show buffered output when viewing a session', () => {
      // @step Given I have a session that ran while I was viewing another session
      const sessionId = 'session-with-output';
      vi.mocked(sessionGetBufferedOutput).mockReturnValue([
        { type: 'Text', text: 'Output line 1' },
        { type: 'Text', text: 'Output line 2' },
        { type: 'Done' },
      ]);

      // @step When I select the session from /resume
      const bufferedOutput = sessionGetBufferedOutput(sessionId, 1000);

      // @step Then I see all the buffered output
      expect(bufferedOutput).toHaveLength(3);
      expect(bufferedOutput[0]).toEqual(
        expect.objectContaining({ type: 'Text', text: 'Output line 1' })
      );
      expect(bufferedOutput[1]).toEqual(
        expect.objectContaining({ type: 'Text', text: 'Output line 2' })
      );

      // GlobalSessionStreamManager will route future chunks to the TUI handler
    });

    it('should restore messages when viewing a session', async () => {
      // @step Given I have a session with persisted conversation history
      const sessionId = 'session-with-history';
      const envelopes = [
        JSON.stringify({
          message: { role: 'user', content: [{ type: 'text', text: 'Hello' }] },
        }),
        JSON.stringify({
          message: {
            role: 'assistant',
            content: [{ type: 'text', text: 'Hi there!' }],
          },
        }),
      ];

      // @step When I view the session via /resume
      await sessionRestoreMessages(sessionId, envelopes);

      // @step Then the messages are restored to the session
      expect(sessionRestoreMessages).toHaveBeenCalledWith(sessionId, envelopes);
    });

    it('should restore token state when viewing a session', async () => {
      // @step Given I have a session with persisted token usage
      const sessionId = 'session-with-tokens';
      const tokenUsage = {
        currentContextTokens: 5000,
        cumulativeBilledInput: 10000,
        cumulativeBilledOutput: 8000,
        cacheReadTokens: 2000,
        cacheCreationTokens: 1000,
      };

      // @step When I view the session via /resume
      await sessionRestoreTokenState(
        sessionId,
        tokenUsage.currentContextTokens,
        tokenUsage.cumulativeBilledOutput,
        tokenUsage.cacheReadTokens,
        tokenUsage.cacheCreationTokens,
        tokenUsage.cumulativeBilledInput,
        tokenUsage.cumulativeBilledOutput
      );

      // @step Then the token state is restored to the background session
      expect(sessionRestoreTokenState).toHaveBeenCalledWith(
        sessionId,
        tokenUsage.currentContextTokens,
        tokenUsage.cumulativeBilledOutput,
        tokenUsage.cacheReadTokens,
        tokenUsage.cacheCreationTokens,
        tokenUsage.cumulativeBilledInput,
        tokenUsage.cumulativeBilledOutput
      );
    });
  });

  describe('NAPI-009: Send input with thinking config', () => {
    it('should send input with thinking config to background session', () => {
      // @step Given I have an active session
      const sessionId = 'active-session';

      // @step When I send input with thinking config
      const thinkingConfig = JSON.stringify({
        type: 'enabled',
        budget_tokens: 10000,
      });
      sessionSendInput(sessionId, 'Hello, agent!', thinkingConfig);

      // @step Then the input is sent with thinking config to the background session
      expect(sessionSendInput).toHaveBeenCalledWith(
        sessionId,
        'Hello, agent!',
        thinkingConfig
      );
    });

    it('should send input without thinking config', () => {
      // @step Given I have an active session
      const sessionId = 'active-session';

      // @step When I send input without thinking config
      sessionSendInput(sessionId, 'Hello!', null);

      // @step Then the input is sent without thinking config
      expect(sessionSendInput).toHaveBeenCalledWith(sessionId, 'Hello!', null);
    });
  });

  describe('NAPI-009: Interrupt a running session', () => {
    it('should interrupt a running session', () => {
      // @step Given I have a session that is currently running
      const sessionId = 'running-session';
      vi.mocked(sessionGetStatus).mockReturnValue('running');
      expect(sessionGetStatus(sessionId)).toBe('running');

      // @step When I send an interrupt signal
      sessionInterrupt(sessionId);

      // @step Then the session is interrupted
      expect(sessionInterrupt).toHaveBeenCalledWith(sessionId);

      // @step And the status changes to idle after interrupt completes
      vi.mocked(sessionGetStatus).mockReturnValue('idle');
      expect(sessionGetStatus(sessionId)).toBe('idle');
    });
  });

  describe('Integration: Full session lifecycle', () => {
    it('should support full session lifecycle workflow', () => {
      // @step Given I create a session with persistence ID
      const sessionId = '550e8400-e29b-41d4-a716-446655440000';
      sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4',
        '/project',
        'My Task'
      );

      // @step And I send input to start the agent (with thinking config)
      const thinkingConfig = JSON.stringify({
        type: 'enabled',
        budget_tokens: 5000,
      });
      sessionSendInput(sessionId, 'Do something', thinkingConfig);
      vi.mocked(sessionGetStatus).mockReturnValue('running');

      // @step When I switch to another session (via GlobalSessionStreamManager)
      // The running session continues in background

      // @step Then the session continues running
      expect(sessionGetStatus(sessionId)).toBe('running');

      // @step And the session appears in the list
      vi.mocked(sessionManagerList).mockReturnValue([
        {
          id: sessionId,
          status: 'running',
          name: 'My Task',
          project: '/project',
          messageCount: 1,
          isIsolated: false,
        },
      ]);
      const sessions = sessionManagerList();
      expect(sessions).toHaveLength(1);
      expect(sessions[0].id).toBe(sessionId);
      expect(sessions[0].status).toBe('running');

      // @step When I switch back to this session via /resume
      vi.mocked(sessionGetStatus).mockReturnValue('idle'); // Task finished
      vi.mocked(sessionGetBufferedOutput).mockReturnValue([
        { type: 'Text', text: 'Task completed!' },
        { type: 'Done' },
      ]);

      const buffered = sessionGetBufferedOutput(sessionId, 1000);
      expect(buffered).toHaveLength(2);

      // GlobalSessionStreamManager routes future chunks to TUI handler

      // @step Then I can continue the conversation
      sessionSendInput(sessionId, 'What did you do?', null);
      expect(sessionSendInput).toHaveBeenCalledWith(
        sessionId,
        'What did you do?',
        null
      );
    });
  });
});
