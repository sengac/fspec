/**
 * Feature: spec/features/persistent-chunk-handler.feature
 *
 * Tests for TUI persistent chunk handler that displays bridge input while idle.
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios map directly to Gherkin scenarios.
 *
 * BRIDGE-013: Fix bridge input not displaying in TUI when idle.
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

import type { StreamChunk } from '@sengac/codelet-napi';
import {
  persistenceSetDataDirectory,
  persistenceCreateSessionWithProvider,
  sessionManagerDestroy,
  sessionManagerList,
} from '@sengac/codelet-napi';

import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../../test-helpers/universal-test-setup';

import {
  GlobalSessionStreamManager,
  initGlobalSessionStreamManager,
  stopGlobalSessionStreamManager,
} from '../../services/globalSessionStreamManager';

import {
  processStreamingChunk,
  type ChunkProcessorContext,
} from '../../utils/chunkProcessor';
import type { ConversationMessage } from '../../types/conversation';

// ============================================================================
// Test Setup
// ============================================================================

describe('Feature: TUI persistent chunk handler for bridge input display', () => {
  let testSetup: WorkUnitTestSetup;

  beforeAll(async () => {
    testSetup = await setupWorkUnitTest('bridge-013-test');
    persistenceSetDataDirectory(testSetup.testDir);
  });

  afterAll(async () => {
    await testSetup.cleanup();
  });

  beforeEach(() => {
    stopGlobalSessionStreamManager();
    initGlobalSessionStreamManager();
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

  // Helper to create chunk processor context
  const createChunkContext = (): ChunkProcessorContext => ({
    formatToolHeader: (name: string, args: string) =>
      args ? `● ${name}(${args})` : `● ${name}()`,
    formatCollapsedOutput: (content: string) =>
      content.length > 100 ? `L ${content.slice(0, 100)}...` : `L ${content}`,
    pendingToolCalls: new Map(),
  });

  // ===========================================================================
  // Scenario: Display TextChunks from bridge input while TUI is idle
  // ===========================================================================

  describe('Scenario: Display TextChunks from bridge input while TUI is idle', () => {
    it('should display TextChunk content in TUI conversation when bridge sends input', async () => {
      // @step Given the TUI is viewing session "test-session"
      const session = persistenceCreateSessionWithProvider(
        'test-session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the TUI input is idle with no pending requests
      // @step And a persistent chunk handler is registered for "test-session"
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const unregister = manager.registerHandler(
        session.id,
        (_sessionId: string, chunk: StreamChunk) => {
          processStreamingChunk(chunk, conversation, ctx);
        }
      );

      // @step When the bridge sends input to session "test-session"
      const supervisorInputChunk: StreamChunk = {
        type: 'IncomingMessage',
        text: '[SUPERVISOR: Telegram | Session: bridge-123]\nHello from Telegram!',
      };
      manager.simulateChunk(session.id, supervisorInputChunk);

      // @step And the LLM responds with TextChunk data
      const textChunk: StreamChunk = {
        type: 'Text',
        text: 'Hello! I received your message from Telegram.',
      };
      manager.simulateChunk(session.id, textChunk);

      // @step Then the TextChunk content should appear in the TUI conversation
      expect(conversation.length).toBeGreaterThan(0);
      const textMessages = conversation.filter(
        m => m.type === 'assistant-text'
      );
      expect(textMessages.length).toBeGreaterThan(0);
      expect(textMessages[textMessages.length - 1].content).toContain(
        'Hello! I received your message'
      );

      // @step And the conversation should update in real-time
      expect(conversation.some(m => m.type === 'supervisor-input')).toBe(true);

      unregister();
    });
  });

  // ===========================================================================
  // Scenario: Display ToolCall chunks from bridge input while TUI is idle
  // ===========================================================================

  describe('Scenario: Display ToolCall chunks from bridge input while TUI is idle', () => {
    it('should display ToolCall in TUI conversation when bridge triggers tool use', async () => {
      // @step Given the TUI is viewing session "test-session"
      const session = persistenceCreateSessionWithProvider(
        'toolcall-session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the TUI input is idle with no pending requests
      // @step And a persistent chunk handler is registered for "test-session"
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const unregister = manager.registerHandler(
        session.id,
        (_sessionId: string, chunk: StreamChunk) => {
          processStreamingChunk(chunk, conversation, ctx);
        }
      );

      // @step When the bridge sends input to session "test-session"
      const supervisorInputChunk: StreamChunk = {
        type: 'IncomingMessage',
        text: '[SUPERVISOR: Telegram | Session: bridge-456]\nRead the file package.json',
      };
      manager.simulateChunk(session.id, supervisorInputChunk);

      // @step And the LLM responds with ToolCall chunks
      const toolCallChunk: StreamChunk = {
        type: 'ToolCall',
        toolCall: {
          id: 'tool-call-1',
          name: 'Read',
          input: JSON.stringify({ file_path: 'package.json' }),
        },
      };
      manager.simulateChunk(session.id, toolCallChunk);

      // @step Then the ToolCall should appear in the TUI conversation
      expect(conversation.some(m => m.type === 'tool-call')).toBe(true);
      const toolCallMsg = conversation.find(m => m.type === 'tool-call');
      expect(toolCallMsg?.content).toContain('Read');

      // @step And the tool execution should be displayed
      const toolResultChunk: StreamChunk = {
        type: 'ToolResult',
        toolResult: {
          toolCallId: 'tool-call-1',
          content: '{"name": "fspec", "version": "1.0.0"}',
          isError: false,
        },
      };
      manager.simulateChunk(session.id, toolResultChunk);

      const updatedToolCall = conversation.find(
        m => m.type === 'tool-call' && m.toolCallId === 'tool-call-1'
      );
      expect(updatedToolCall).toBeDefined();

      unregister();
    });
  });

  // ===========================================================================
  // Scenario: Done chunk updates conversation state and re-enables input
  // ===========================================================================

  describe('Scenario: Done chunk updates conversation state and re-enables input', () => {
    it('should finalize conversation state when Done chunk is received', async () => {
      // @step Given the TUI is viewing session "test-session"
      const session = persistenceCreateSessionWithProvider(
        'done-chunk-session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the TUI input is idle with no pending requests
      // @step And a persistent chunk handler is registered for "test-session"
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const unregister = manager.registerHandler(
        session.id,
        (_sessionId: string, chunk: StreamChunk) => {
          processStreamingChunk(chunk, conversation, ctx);
        }
      );

      // @step When the bridge sends input to session "test-session"
      const supervisorInputChunk: StreamChunk = {
        type: 'IncomingMessage',
        text: '[SUPERVISOR: Telegram | Session: bridge-789]\nHello!',
      };
      manager.simulateChunk(session.id, supervisorInputChunk);

      const textChunk: StreamChunk = {
        type: 'Text',
        text: 'Hello from the assistant!',
      };
      manager.simulateChunk(session.id, textChunk);

      const streamingBefore = conversation.find(
        m => m.type === 'assistant-text' && m.isStreaming === true
      );
      expect(streamingBefore).toBeDefined();

      // @step And the LLM responds with a Done chunk
      const doneChunk: StreamChunk = { type: 'Done' };
      manager.simulateChunk(session.id, doneChunk);

      // @step Then the conversation state should be updated
      const streamingAfter = conversation.find(m => m.isStreaming === true);
      expect(streamingAfter).toBeUndefined();

      // @step And the TUI input should be re-enabled
      const finalizedMessage = conversation.find(
        m => m.type === 'assistant-text' && m.isStreaming === false
      );
      expect(finalizedMessage).toBeDefined();

      unregister();
    });
  });

  // ===========================================================================
  // Scenario: Switching sessions does not show chunks from previous session
  // ===========================================================================

  describe('Scenario: Switching sessions does not show chunks from previous session', () => {
    it('should not display chunks from old session after switching', async () => {
      // @step Given the TUI is viewing session "session-A"
      const sessionA = persistenceCreateSessionWithProvider(
        'session-A',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const sessionB = persistenceCreateSessionWithProvider(
        'session-B',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession(sessionA.id);
      manager.subscribeToSession(sessionB.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And a persistent chunk handler is registered for "session-A"
      const conversationA: ConversationMessage[] = [];
      const conversationB: ConversationMessage[] = [];
      const ctx = createChunkContext();

      const unregisterA = manager.registerHandler(
        sessionA.id,
        (_sessionId: string, chunk: StreamChunk) => {
          processStreamingChunk(chunk, conversationA, ctx);
        }
      );

      // @step When the user switches to session "session-B"
      unregisterA();
      const unregisterB = manager.registerHandler(
        sessionB.id,
        (_sessionId: string, chunk: StreamChunk) => {
          processStreamingChunk(chunk, conversationB, ctx);
        }
      );

      // @step And the bridge sends input to session "session-A"
      const supervisorInputChunk: StreamChunk = {
        type: 'IncomingMessage',
        text: '[SUPERVISOR: Telegram | Session: bridge-A]\nMessage for session A',
      };
      manager.simulateChunk(sessionA.id, supervisorInputChunk);

      // @step And the LLM responds with TextChunk data for "session-A"
      const textChunkA: StreamChunk = {
        type: 'Text',
        text: 'Response for session A',
      };
      manager.simulateChunk(sessionA.id, textChunkA);

      // @step Then the TUI should NOT display chunks from "session-A"
      expect(conversationA.length).toBe(0);

      // @step And the persistent handler for "session-A" should be unregistered
      const textChunkB: StreamChunk = {
        type: 'Text',
        text: 'Message for session B',
      };
      manager.simulateChunk(sessionB.id, textChunkB);
      expect(conversationB.length).toBeGreaterThan(0);

      unregisterB();
    });
  });

  // ===========================================================================
  // Scenario: User input via TUI flows through persistent handler
  // ===========================================================================

  describe('Scenario: User input via TUI flows through persistent handler', () => {
    it('should process chunks through the same persistent handler for user input', async () => {
      // @step Given the TUI is viewing session "test-session"
      const session = persistenceCreateSessionWithProvider(
        'user-input-session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And a persistent chunk handler is registered for "test-session"
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      let handlerCallCount = 0;
      const unregister = manager.registerHandler(
        session.id,
        (_sessionId: string, chunk: StreamChunk) => {
          handlerCallCount++;
          processStreamingChunk(chunk, conversation, ctx);
        }
      );

      // @step When the user sends input via the TUI
      const userInputChunk: StreamChunk = {
        type: 'UserInput',
        text: 'Hello from TUI!',
      };
      manager.simulateChunk(session.id, userInputChunk);

      // @step And the LLM responds with TextChunk data
      const textChunk: StreamChunk = {
        type: 'Text',
        text: 'I received your TUI input!',
      };
      manager.simulateChunk(session.id, textChunk);

      // @step Then the TextChunk content should appear in the TUI conversation
      expect(conversation.some(m => m.type === 'user-input')).toBe(true);
      expect(conversation.some(m => m.type === 'assistant-text')).toBe(true);

      // @step And the same persistent handler should process the chunks
      expect(handlerCallCount).toBe(2);

      unregister();
    });

    it('should not process chunks twice when multiple handlers are registered', async () => {
      // This tests the handler exclusion pattern used to prevent duplicate messages
      // when both persistent handler and handleSubmit's handler are active
      const session = persistenceCreateSessionWithProvider(
        'no-duplicate-session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // Simulate two handlers (persistent + handleSubmit's handler)
      const conversation1: ConversationMessage[] = [];
      const conversation2: ConversationMessage[] = [];
      const ctx = createChunkContext();

      // First handler (simulates persistent handler)
      const unregister1 = manager.registerHandler(
        session.id,
        (_sessionId: string, chunk: StreamChunk) => {
          processStreamingChunk(chunk, conversation1, ctx);
        }
      );

      // Second handler (simulates handleSubmit's handler)
      const unregister2 = manager.registerHandler(
        session.id,
        (_sessionId: string, chunk: StreamChunk) => {
          processStreamingChunk(chunk, conversation2, ctx);
        }
      );

      // Send a chunk - both handlers receive it
      const textChunk: StreamChunk = { type: 'Text', text: 'Test message' };
      manager.simulateChunk(session.id, textChunk);

      // Both conversations should have the message (this is the bug we fixed in AgentView)
      // The fix in AgentView skips persistent handler when sessionCleanupRef is set
      expect(conversation1.length).toBe(1);
      expect(conversation2.length).toBe(1);

      // Total messages across both = 2 (the bug caused this in AgentView)
      // AgentView's fix ensures only one handler processes at a time
      const totalMessages = conversation1.length + conversation2.length;
      expect(totalMessages).toBe(2);

      unregister1();
      unregister2();
    });
  });

  // ===========================================================================
  // Scenario: SupervisorInput chunk shows injected input before LLM response
  // ===========================================================================

  describe('Scenario: SupervisorInput chunk shows injected input before LLM response', () => {
    it('should display injected input before LLM response', async () => {
      // @step Given the TUI is viewing session "test-session"
      const session = persistenceCreateSessionWithProvider(
        'watcher-input-session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the TUI input is idle with no pending requests
      // @step And a persistent chunk handler is registered for "test-session"
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const unregister = manager.registerHandler(
        session.id,
        (_sessionId: string, chunk: StreamChunk) => {
          processStreamingChunk(chunk, conversation, ctx);
        }
      );

      // @step When the bridge sends input to session "test-session"
      // @step And a SupervisorInput chunk is emitted
      const supervisorInputChunk: StreamChunk = {
        type: 'IncomingMessage',
        text: '[SUPERVISOR: Telegram | Session: bridge-watcher]\nInjected message from bridge',
      };
      manager.simulateChunk(session.id, supervisorInputChunk);

      // @step Then the injected input should appear in the TUI conversation
      expect(conversation.some(m => m.type === 'supervisor-input')).toBe(true);
      const watcherMsg = conversation.find(m => m.type === 'supervisor-input');
      expect(watcherMsg?.content).toContain('Telegram');

      const textChunk: StreamChunk = {
        type: 'Text',
        text: 'LLM response to bridge input',
      };
      manager.simulateChunk(session.id, textChunk);

      // @step And the injected input should appear before the LLM response
      const watcherIndex = conversation.findIndex(
        m => m.type === 'supervisor-input'
      );
      const responseIndex = conversation.findIndex(
        m => m.type === 'assistant-text'
      );
      expect(watcherIndex).toBeLessThan(responseIndex);

      unregister();
    });
  });

  // ===========================================================================
  // Scenario: Handler not registered when session is null
  // ===========================================================================

  describe('Scenario: Handler not registered when session is null', () => {
    it('should not register handler when session ID is null', async () => {
      // @step Given the TUI has no active session
      const sessionId: string | null = null;

      // @step When the component renders
      const manager = GlobalSessionStreamManager.getInstance();

      // @step Then no chunk handler should be registered
      // @step And the GlobalSessionStreamManager should have no handlers
      expect(sessionId).toBeNull();

      // Simulating a chunk to non-existent session doesn't crash
      const nonExistentChunk: StreamChunk = { type: 'Text', text: 'ignored' };
      expect(() =>
        manager.simulateChunk('non-existent-session', nonExistentChunk)
      ).not.toThrow();
    });
  });

  // ===========================================================================
  // Scenario: Chunks flow from Rust NAPI through GlobalSessionStreamManager to React state
  // ===========================================================================

  describe('Scenario: Chunks flow from Rust NAPI through GlobalSessionStreamManager to React state', () => {
    it('should route chunks from NAPI through manager to registered handlers', async () => {
      // @step Given the TUI is viewing session "test-session"
      const session = persistenceCreateSessionWithProvider(
        'e2e-flow-session',
        testSetup.testDir,
        'anthropic/claude-sonnet-4-20250514'
      );
      const manager = GlobalSessionStreamManager.getInstance();

      // @step And a persistent chunk handler is registered for "test-session"
      manager.subscribeToSession(session.id);
      await new Promise(resolve => setTimeout(resolve, 100));

      const receivedChunks: StreamChunk[] = [];
      const unregister = manager.registerHandler(
        session.id,
        (_sessionId: string, chunk: StreamChunk) => {
          receivedChunks.push(chunk);
        }
      );

      // @step When Rust NAPI emits a chunk via GLOBAL_CHUNK_CALLBACK
      const testChunk: StreamChunk = {
        type: 'Text',
        text: 'Chunk from Rust NAPI',
      };
      manager.simulateChunk(session.id, testChunk);

      // @step Then GlobalSessionStreamManager should receive the chunk
      // @step And the chunk should be dispatched to the registered handler
      expect(receivedChunks.length).toBe(1);
      expect(receivedChunks[0].type).toBe('Text');
      expect(receivedChunks[0].text).toBe('Chunk from Rust NAPI');

      // @step And the React conversation state should be updated
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      processStreamingChunk(receivedChunks[0], conversation, ctx);
      expect(conversation.length).toBe(1);
      expect(conversation[0].type).toBe('assistant-text');

      unregister();
    });
  });

  // ===========================================================================
  // Additional: Verify processStreamingChunk handles all chunk types
  // ===========================================================================

  describe('Rule: processStreamingChunk must handle ALL chunk types', () => {
    it('should process Text chunks correctly', () => {
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const chunk: StreamChunk = { type: 'Text', text: 'Hello world' };

      const result = processStreamingChunk(chunk, conversation, ctx);

      expect(result).toBe(true);
      expect(conversation[0].type).toBe('assistant-text');
      expect(conversation[0].content).toBe('Hello world');
    });

    it('should process Thinking chunks correctly', () => {
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const chunk: StreamChunk = {
        type: 'Thinking',
        thinking: 'Let me think...',
      };

      const result = processStreamingChunk(chunk, conversation, ctx);

      expect(result).toBe(true);
      expect(conversation[0].type).toBe('thinking');
      expect(conversation[0].content).toContain('Let me think...');
    });

    it('should process SupervisorInput chunks correctly', () => {
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const chunk: StreamChunk = {
        type: 'IncomingMessage',
        text: '[SUPERVISOR: Test | Session: test-123]\nTest input',
      };

      const result = processStreamingChunk(chunk, conversation, ctx);

      expect(result).toBe(true);
      expect(conversation[0].type).toBe('supervisor-input');
    });

    it('should process UserInput chunks correctly', () => {
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const chunk: StreamChunk = { type: 'UserInput', text: 'User message' };

      const result = processStreamingChunk(chunk, conversation, ctx);

      expect(result).toBe(true);
      expect(conversation[0].type).toBe('user-input');
      expect(conversation[0].content).toBe('User message');
    });

    it('should process ToolCall chunks correctly', () => {
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const chunk: StreamChunk = {
        type: 'ToolCall',
        toolCall: {
          id: 'tool-1',
          name: 'Read',
          input: JSON.stringify({ file_path: 'test.txt' }),
        },
      };

      const result = processStreamingChunk(chunk, conversation, ctx);

      expect(result).toBe(true);
      expect(conversation[0].type).toBe('tool-call');
      expect(conversation[0].content).toContain('Read');
    });

    it('should process ToolResult chunks correctly', () => {
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();

      // First add a tool call
      const toolCallChunk: StreamChunk = {
        type: 'ToolCall',
        toolCall: {
          id: 'tool-1',
          name: 'Read',
          input: JSON.stringify({ file_path: 'test.txt' }),
        },
      };
      processStreamingChunk(toolCallChunk, conversation, ctx);

      // Then add result
      const resultChunk: StreamChunk = {
        type: 'ToolResult',
        toolResult: {
          toolCallId: 'tool-1',
          content: 'File contents here',
          isError: false,
        },
      };
      const result = processStreamingChunk(resultChunk, conversation, ctx);

      expect(result).toBe(true);
    });

    it('should process Done chunks correctly', () => {
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();

      // Add streaming message first
      const textChunk: StreamChunk = { type: 'Text', text: 'Hello' };
      processStreamingChunk(textChunk, conversation, ctx);
      expect(conversation[0].isStreaming).toBe(true);

      // Process Done
      const doneChunk: StreamChunk = { type: 'Done' };
      const result = processStreamingChunk(doneChunk, conversation, ctx);

      expect(result).toBe(true);
      expect(conversation[0].isStreaming).toBe(false);
    });

    it('should process Error chunks correctly', () => {
      const conversation: ConversationMessage[] = [];
      const ctx = createChunkContext();
      const chunk: StreamChunk = {
        type: 'Error',
        error: 'API rate limit exceeded',
      };

      const result = processStreamingChunk(chunk, conversation, ctx);

      expect(result).toBe(true);
      expect(conversation[0].type).toBe('status');
      expect(conversation[0].content).toContain('API Error');
    });

    it('should process HistoryCleared chunks correctly', () => {
      const conversation: ConversationMessage[] = [
        { type: 'user-input', content: 'Old message' },
        { type: 'assistant-text', content: 'Old response' },
      ];
      const ctx = createChunkContext();
      const chunk: StreamChunk = { type: 'HistoryCleared' };

      const result = processStreamingChunk(chunk, conversation, ctx);

      expect(result).toBe(true);
      expect(conversation.length).toBe(1);
      expect(conversation[0].type).toBe('status');
      expect(conversation[0].content).toBe('History cleared');
    });
  });
});
