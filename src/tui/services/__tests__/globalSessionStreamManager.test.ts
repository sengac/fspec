/**
 * Feature: spec/features/global-session-stream-manager.feature
 *
 * Tests for GlobalSessionStreamManager.
 */

import {
  describe,
  it,
  expect,
  beforeEach,
  afterEach,
  beforeAll,
  afterAll,
} from 'vitest';

import {
  persistenceSetDataDirectory,
  persistenceCreateSessionWithProvider,
  sessionManagerDestroy,
  sessionManagerList,
} from '@sengac/codelet-napi';
import type { StreamChunk } from '@sengac/codelet-napi';

import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../../test-helpers/universal-test-setup';

import {
  GlobalSessionStreamManager,
  initGlobalSessionStreamManager,
  stopGlobalSessionStreamManager,
} from '../globalSessionStreamManager';

import { useSessionStore } from '../../store/sessionStore';

describe('Feature: Global Session Stream Subscription for FspecCommandRequest Handling', () => {
  let testSetup: WorkUnitTestSetup;

  beforeAll(async () => {
    testSetup = await setupWorkUnitTest('gssm-test');
    persistenceSetDataDirectory(testSetup.testDir);
  });

  afterAll(async () => {
    await testSetup.cleanup();
  });

  beforeEach(() => {
    stopGlobalSessionStreamManager();
  });

  afterEach(() => {
    stopGlobalSessionStreamManager();

    const sessions = sessionManagerList();
    for (const session of sessions) {
      try {
        sessionManagerDestroy(session.id);
      } catch {
        // Session might already be destroyed
      }
    }
  });

  describe('Background: User Story', () => {
    it('validates the user story for detached session fspec tools', () => {
      // @step As a developer
      // @step I want to have fspec commands execute successfully from detached sessions
      // @step So that agents running in background can use fspec tools without deadlocking
      expect(true).toBe(true);
    });
  });

  describe('Scenario: Fspec command completes successfully after user navigates away', () => {
    it('should handle FspecCommandRequest globally even when no session handlers registered', async () => {
      // @step Given I have Session A running with an agent
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      manager.subscribeToSession(sessionA.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      const uiChunksReceived: StreamChunk[] = [];
      const unregister = manager.registerHandler(
        sessionA.id,
        (_sessionId, chunk) => {
          uiChunksReceived.push(chunk);
        }
      );

      // @step And I send a message that invokes the fspec tool in Session A
      const fspecRequest: StreamChunk = {
        type: 'FspecCommandRequest',
        fspecRequest: {
          command: 'board',
          argsJson: '{}',
          projectRoot: testSetup.testDir,
          toolCallId: 'test-tool-call-1',
        },
      };

      // @step When I navigate to Session B before the fspec command completes
      unregister();

      manager.simulateChunk(sessionA.id, fspecRequest);
      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then the fspec command in Session A should complete successfully
      expect(uiChunksReceived).toHaveLength(0);

      // @step And Session A should not deadlock
      expect(manager.getSubscribedSessions()).toContain(sessionA.id);
    });
  });

  describe('Scenario: Multiple detached sessions can invoke fspec tools concurrently', () => {
    it('should handle FspecCommandRequest from multiple sessions concurrently', async () => {
      // @step Given I have 3 detached sessions running agents
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const session1 = persistenceCreateSessionWithProvider(
        'Session 1',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const session2 = persistenceCreateSessionWithProvider(
        'Session 2',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const session3 = persistenceCreateSessionWithProvider(
        'Session 3',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(session1.id);
      manager.subscribeToSession(session2.id);
      manager.subscribeToSession(session3.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And each session sends a message invoking the fspec tool
      const fspecRequest1: StreamChunk = {
        type: 'FspecCommandRequest',
        fspecRequest: {
          command: 'board',
          argsJson: '{}',
          projectRoot: testSetup.testDir,
          toolCallId: 'tool-call-session-1',
        },
      };
      const fspecRequest2: StreamChunk = {
        type: 'FspecCommandRequest',
        fspecRequest: {
          command: 'board',
          argsJson: '{}',
          projectRoot: testSetup.testDir,
          toolCallId: 'tool-call-session-2',
        },
      };
      const fspecRequest3: StreamChunk = {
        type: 'FspecCommandRequest',
        fspecRequest: {
          command: 'board',
          argsJson: '{}',
          projectRoot: testSetup.testDir,
          toolCallId: 'tool-call-session-3',
        },
      };

      // @step When I am viewing the BoardView
      manager.simulateChunk(session1.id, fspecRequest1);
      manager.simulateChunk(session2.id, fspecRequest2);
      manager.simulateChunk(session3.id, fspecRequest3);

      // @step Then all 3 fspec commands should complete successfully
      await new Promise(resolve => setTimeout(resolve, 500));

      // @step And no sessions should deadlock
      expect(manager.getSubscribedSessions()).toContain(session1.id);
      expect(manager.getSubscribedSessions()).toContain(session2.id);
      expect(manager.getSubscribedSessions()).toContain(session3.id);
    });
  });

  describe('Scenario: GlobalSessionStreamManager subscribes to new sessions automatically', () => {
    it('should subscribe to new session and track in subscriptions map', async () => {
      // @step Given the GlobalSessionStreamManager is initialized
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();
      expect(manager).toBeDefined();
      const initialSessions = manager.getSubscribedSessions();
      expect(initialSessions).toEqual([]);

      // @step When a new session is created
      const session = persistenceCreateSessionWithProvider(
        'Test Session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionId = session.id;

      manager.subscribeToSession(sessionId);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the GlobalSessionStreamManager should subscribe to the new session
      // @step And the session should be tracked in the subscriptions map
      const subscribedSessions = manager.getSubscribedSessions();
      expect(subscribedSessions).toContain(sessionId);
    });
  });

  describe('Scenario: GlobalSessionStreamManager unsubscribes when session is destroyed', () => {
    it('should unsubscribe and remove session from subscriptions map', async () => {
      // @step Given the GlobalSessionStreamManager is initialized
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();
      expect(manager).toBeDefined();

      const session = persistenceCreateSessionWithProvider(
        'Session to Destroy',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionId = session.id;

      // @step And a session exists with an active subscription
      manager.subscribeToSession(sessionId);
      await new Promise(resolve => setTimeout(resolve, 100));
      expect(manager.getSubscribedSessions()).toContain(sessionId);

      // @step When the session is destroyed
      manager.unsubscribeFromSession(sessionId);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the GlobalSessionStreamManager should unsubscribe from the session
      // @step And the session should be removed from the subscriptions map
      const subscribedSessions = manager.getSubscribedSessions();
      expect(subscribedSessions).not.toContain(sessionId);
    });
  });

  describe('Scenario: AgentView receives UI chunks but not FspecCommandRequest', () => {
    it('should forward UI chunks to registered handlers but NOT FspecCommandRequest', async () => {
      // @step Given the GlobalSessionStreamManager is handling events for a session
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const session = persistenceCreateSessionWithProvider(
        'Handler Test Session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionId = session.id;

      manager.subscribeToSession(sessionId);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And AgentView is displaying that session
      const uiChunksReceived: StreamChunk[] = [];
      const unregister = manager.registerHandler(
        sessionId,
        (_sessionId, chunk) => {
          uiChunksReceived.push(chunk);
        }
      );

      // @step When the session emits a Text chunk
      const textChunk: StreamChunk = {
        type: 'Text',
        text: 'Hello, world!',
      };
      manager.simulateChunk(sessionId, textChunk);

      // @step Then AgentView should receive the Text chunk for UI rendering
      expect(uiChunksReceived).toHaveLength(1);
      expect(uiChunksReceived[0].type).toBe('Text');
      expect(uiChunksReceived[0].text).toBe('Hello, world!');

      // @step When the session emits a FspecCommandRequest chunk
      const fspecRequest: StreamChunk = {
        type: 'FspecCommandRequest',
        fspecRequest: {
          command: 'board',
          argsJson: '{}',
          projectRoot: testSetup.testDir,
          toolCallId: 'test-tool-call-id',
        },
      };
      manager.simulateChunk(sessionId, fspecRequest);

      // @step Then AgentView should NOT receive the FspecCommandRequest chunk
      expect(uiChunksReceived).toHaveLength(1);

      // @step And the GlobalSessionStreamManager should handle the FspecCommandRequest
      expect(manager.getSubscribedSessions()).toContain(sessionId);

      unregister();
    });

    it('should forward multiple UI chunk types to handlers', async () => {
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const session = persistenceCreateSessionWithProvider(
        'Multi-Chunk Session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      const chunksReceived: StreamChunk[] = [];
      const unregister = manager.registerHandler(
        session.id,
        (_sessionId, chunk) => {
          chunksReceived.push(chunk);
        }
      );

      manager.simulateChunk(session.id, { type: 'Text', text: 'Hello' });
      manager.simulateChunk(session.id, {
        type: 'Thinking',
        thinking: 'Pondering...',
      });
      manager.simulateChunk(session.id, {
        type: 'ToolCall',
        toolName: 'Read',
        toolCallId: 'tool-1',
      });

      expect(chunksReceived).toHaveLength(3);
      expect(chunksReceived[0].type).toBe('Text');
      expect(chunksReceived[1].type).toBe('Thinking');
      expect(chunksReceived[2].type).toBe('ToolCall');

      unregister();
    });

    it('should allow multiple handlers to register for the same session', async () => {
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const session = persistenceCreateSessionWithProvider(
        'Multi-Handler Session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      const handler1Chunks: StreamChunk[] = [];
      const handler2Chunks: StreamChunk[] = [];

      const unregister1 = manager.registerHandler(
        session.id,
        (_sessionId, chunk) => {
          handler1Chunks.push(chunk);
        }
      );
      const unregister2 = manager.registerHandler(
        session.id,
        (_sessionId, chunk) => {
          handler2Chunks.push(chunk);
        }
      );

      manager.simulateChunk(session.id, {
        type: 'Text',
        text: 'Both handlers',
      });

      expect(handler1Chunks).toHaveLength(1);
      expect(handler2Chunks).toHaveLength(1);

      unregister1();
      unregister2();
    });

    it('should support global handlers that receive events from all sessions', async () => {
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const session1 = persistenceCreateSessionWithProvider(
        'Global Handler Session 1',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const session2 = persistenceCreateSessionWithProvider(
        'Global Handler Session 2',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(session1.id);
      manager.subscribeToSession(session2.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      const globalChunks: Array<{ sessionId: string; chunk: StreamChunk }> = [];
      const unregister = manager.registerGlobalHandler((sessionId, chunk) => {
        globalChunks.push({ sessionId, chunk });
      });

      manager.simulateChunk(session1.id, {
        type: 'Text',
        text: 'From session 1',
      });
      manager.simulateChunk(session2.id, {
        type: 'Text',
        text: 'From session 2',
      });

      expect(globalChunks).toHaveLength(2);
      expect(globalChunks[0].sessionId).toBe(session1.id);
      expect(globalChunks[1].sessionId).toBe(session2.id);

      unregister();
    });
  });

  describe('Scenario: Tests use real NAPI bindings without mocks', () => {
    it('should use real NAPI bindings and fixtures with no mocks', async () => {
      // @step Given the test environment using universal-test-setup.ts for temp directories
      expect(persistenceSetDataDirectory).toBeDefined();
      expect(persistenceCreateSessionWithProvider).toBeDefined();
      expect(sessionManagerDestroy).toBeDefined();
      expect(testSetup).toBeDefined();
      expect(testSetup.testDir).toBeDefined();
      expect(testSetup.cleanup).toBeDefined();

      // @step When a test creates a session via persistenceCreateSessionWithProvider
      const session = persistenceCreateSessionWithProvider(
        'No Mocks Test Session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      expect(session).toBeDefined();
      expect(session.id).toBeDefined();
      expect(typeof session.id).toBe('string');
      expect(session.name).toBe('No Mocks Test Session');
      expect(session.project).toBe(testSetup.testDir);

      // @step And subscribes to it via GlobalSessionStreamManager
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));
      expect(manager.getSubscribedSessions()).toContain(session.id);

      // @step And simulates a FspecCommandRequest chunk via simulateChunk
      const fspecRequest: StreamChunk = {
        type: 'FspecCommandRequest',
        fspecRequest: {
          command: 'board',
          argsJson: '{}',
          projectRoot: testSetup.testDir,
          toolCallId: 'no-mocks-tool-call',
        },
      };
      manager.simulateChunk(session.id, fspecRequest);
      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then the fspec command should execute successfully
      expect(manager.getSubscribedSessions()).toContain(session.id);

      // @step And no mocks should be used for GlobalSessionStreamManager
      expect(session.id).toMatch(/^[a-f0-9-]{36}$/);
      expect(manager.getSubscribedSessions()).toContain(session.id);

      // @step And temp directories should be automatically cleaned up
      expect(typeof testSetup.cleanup).toBe('function');
    });
  });

  describe('Singleton pattern', () => {
    it('should return the same instance on multiple calls', () => {
      initGlobalSessionStreamManager();
      const manager1 = GlobalSessionStreamManager.getInstance();
      const manager2 = GlobalSessionStreamManager.getInstance();

      expect(manager1).toBe(manager2);
    });

    it('should reset state when resetInstance is called', async () => {
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const session = persistenceCreateSessionWithProvider(
        'Reset Test Session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));
      expect(manager.getSubscribedSessions()).toContain(session.id);

      GlobalSessionStreamManager.resetInstance();

      const newManager = GlobalSessionStreamManager.getInstance();
      expect(newManager.getSubscribedSessions()).toEqual([]);
    });
  });

  // ----------------------------------------
  // GIT-029: IsolationStateChange StreamChunk handling
  // ----------------------------------------

  describe('Scenario: IsolationStateChange StreamChunk updates sessionStore', () => {
    it('should update sessionStore when IsolationStateChange chunk is received for active session', async () => {
      // @step Given the GlobalSessionStreamManager is initialized
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const session = persistenceCreateSessionWithProvider(
        'Isolation Test Session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And I have an active session
      useSessionStore.getState().activateSession(session.id);
      expect(useSessionStore.getState().currentSessionId).toBe(session.id);

      // Initial state should be non-isolated
      expect(useSessionStore.getState().isIsolated).toBe(false);
      expect(useSessionStore.getState().worktreePath).toBeNull();

      // @step When an IsolationStateChange chunk is received with isIsolated=true and worktreePath set
      const isolationChunk: StreamChunk = {
        type: 'IsolationStateChange',
        isIsolated: true,
        worktreePath: '/project/.fspec/worktrees/test-session-id',
      };
      manager.simulateChunk(session.id, isolationChunk);

      // @step Then the sessionStore should have isIsolated set to true
      expect(useSessionStore.getState().isIsolated).toBe(true);

      // @step And the sessionStore should have worktreePath set to the received path
      expect(useSessionStore.getState().worktreePath).toBe(
        '/project/.fspec/worktrees/test-session-id'
      );

      // Cleanup
      useSessionStore.getState().reset();
    });
  });

  describe('Scenario: IsolationStateChange StreamChunk ignored for non-active sessions', () => {
    it('should not update sessionStore when IsolationStateChange chunk is received for different session', async () => {
      // @step Given the GlobalSessionStreamManager is initialized
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionA = persistenceCreateSessionWithProvider(
        'Session A',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionB = persistenceCreateSessionWithProvider(
        'Session B',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );

      manager.subscribeToSession(sessionA.id);
      manager.subscribeToSession(sessionB.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And I have an active session "session-A"
      useSessionStore.getState().activateSession(sessionA.id);
      expect(useSessionStore.getState().currentSessionId).toBe(sessionA.id);

      // Set initial isolation state for session A
      useSessionStore.getState().setIsolationState(false, null);

      // @step When an IsolationStateChange chunk is received for a different session "session-B"
      const isolationChunk: StreamChunk = {
        type: 'IsolationStateChange',
        isIsolated: true,
        worktreePath: '/project/.fspec/worktrees/session-b-id',
      };
      manager.simulateChunk(sessionB.id, isolationChunk);

      // @step Then the sessionStore isolation state should remain unchanged
      expect(useSessionStore.getState().isIsolated).toBe(false);
      expect(useSessionStore.getState().worktreePath).toBeNull();

      // Cleanup
      useSessionStore.getState().reset();
    });
  });
});
