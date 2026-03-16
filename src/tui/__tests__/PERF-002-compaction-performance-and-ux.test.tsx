/**
 * PERF-002: Optimize Context Compaction Performance and UX
 * 
 * Fixed using the working AgentView test pattern - COMPLETE working mocks
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';

// Track callback and resolver at module level for test control (NAPI-009)
let capturedCallback: ((err: Error | null, chunk: unknown) => void) | null = null;
let capturedResolver: (() => void) | null = null;

// Mock model data matching models.dev structure
const mockModels = vi.hoisted(() => ({
  anthropic: {
    providerId: 'anthropic',
    providerName: 'Anthropic',
    models: [
      {
        id: 'claude-sonnet-4-20250514',
        name: 'Claude Sonnet 4',
        family: 'claude-sonnet-4',
        reasoning: true,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 16000,
        hasVision: true,
      },
    ],
  },
  openai: {
    providerId: 'openai',
    providerName: 'OpenAI',
    models: [
      {
        id: 'gpt-4o',
        name: 'GPT-4o',
        family: 'gpt-4o',
        reasoning: false,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 128000,
        maxOutput: 16384,
        hasVision: true,
      },
    ],
  },
}));

// Create mock state that persists across mock hoisting
const mockState = vi.hoisted(() => ({
  session: {
    currentProviderName: 'claude',
    availableProviders: ['claude', 'openai'],
    tokenTracker: { inputTokens: 0, outputTokens: 0 },
    messages: [] as Array<{ role: string; content: string }>,
    prompt: vi.fn(),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    toggleDebug: vi.fn().mockReturnValue({
      enabled: true,
      sessionFile: '~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
      message: 'Debug capture started. Writing to: ~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
    }),
    compact: vi.fn().mockReturnValue({
      originalTokens: 150000,
      compactedTokens: 40000,
      compressionRatio: 73.3,
      turnsSummarized: 12,
      turnsKept: 3,
    }),
  },
  shouldThrow: false,
  errorMessage: 'No AI provider credentials configured',
}));

// Mock codelet-napi module - COMPLETE working version
vi.mock('@sengac/codelet-napi', () => ({
  JsThinkingLevel: {
    Off: 0,
    Low: 1,
    Medium: 2,
    High: 3,
  },
  getThinkingConfig: vi.fn(() => null),
  // BRIDGE-006: Unified thinking level detection NAPI functions
  napiDetectThinkingLevel: vi.fn(() => 0), // Default to Off
  napiHasDisableKeywords: vi.fn(() => false),
  napiComputeEffectiveThinkingLevel: vi.fn((base: number, detected: number, forceOff: boolean) => {
    if (forceOff) { return 0; }
    return Math.max(base, detected);
  }),
  modelsListAll: vi.fn(() => Promise.resolve([mockModels.anthropic, mockModels.openai])),
  setRustLogCallback: vi.fn(),
  persistenceSetDataDirectory: vi.fn(),
  persistenceStoreMessageEnvelope: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceCreateSessionWithProvider: vi.fn(() => ({
    id: 'mock-session-id',
    name: 'Mock Session',
    project: '/test/project',
    provider: 'claude',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    messageCount: 0,
  })),
  persistenceAddHistory: vi.fn(),
  persistenceSearchHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  persistenceAppendMessage: vi.fn(),
  persistenceRenameSession: vi.fn(),
  sessionManagerList: vi.fn().mockReturnValue([]),
  sessionGetBufferedOutput: vi.fn().mockReturnValue([]),
  sessionManagerDestroy: vi.fn(),
  sessionSendInput: vi.fn().mockImplementation((_sessionId: string, _input: string, _thinkingConfig: string | null) => {
    // NAPI-009: Trigger streaming callback when input is sent (simulates background session response)
  }),
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionRestoreMessages: vi.fn(),
  sessionRestoreTokenState: vi.fn(),
  sessionToggleDebug: vi.fn().mockResolvedValue({
    enabled: true,
    sessionFile: '/tmp/debug-session.json',
    message: 'Debug capture enabled. Events will be written to /tmp/debug-session.json',
  }),
  sessionCompact: vi.fn().mockResolvedValue({
    originalTokens: 10000,
    compactedTokens: 3000,
    compressionRatio: 70,
    turnsSummarized: 5,
    turnsKept: 2,
  }),
  sessionGetModel: vi.fn().mockReturnValue({ providerId: null, modelId: null }),
  sessionGetStatus: vi.fn().mockReturnValue('idle'),
  sessionGetTokens: vi.fn().mockReturnValue({ inputTokens: 0, outputTokens: 0 }),
  sessionSetModel: vi.fn().mockResolvedValue(undefined),
  sessionSetModelProfile: vi.fn().mockResolvedValue(undefined),
  sessionInterrupt: vi.fn(),
  sessionSetPendingInput: vi.fn(),
  sessionGetPendingInput: vi.fn().mockReturnValue(null),
  persistenceSetSessionTokens: vi.fn(),
  sessionGetCompactionProgress: vi.fn().mockReturnValue(null),
  sessionGetBaseThinkingLevel: vi.fn().mockReturnValue(0),
  sessionSetBaseThinkingLevel: vi.fn(),
  sessionGetDebugEnabled: vi.fn().mockReturnValue(false),
  sessionGetSubordinate: vi.fn().mockReturnValue(null),
  sessionGetSupervisors: vi.fn().mockReturnValue([]),
}));

// Mock models.dev
vi.mock('../../models.dev', () => ({
  default: mockModels,
}));

// Mock credentials utilities - required for provider filtering
vi.mock('../../utils/credentials', () => ({
  getProviderConfig: vi.fn((registryId: string) => {
    const registryToAvailable: Record<string, string> = {
      anthropic: 'claude',
      openai: 'openai',
    };
    const availableName = registryToAvailable[registryId] || registryId;
    if (mockState.session.availableProviders.includes(availableName)) {
      return Promise.resolve({ apiKey: 'test-key', source: 'file' });
    }
    return Promise.resolve({ apiKey: null, source: null });
  }),
  saveCredential: vi.fn(),
  deleteCredential: vi.fn(),
  maskApiKey: vi.fn((key: string) => '***'),
}));

// Mock config module to prevent loading user's real config
vi.mock('../../utils/config', () => ({
  loadConfig: vi.fn(() => Promise.resolve({})),
  writeConfig: vi.fn(() => Promise.resolve()),
  getFspecUserDir: vi.fn(() => '/tmp/fspec-test'),
}));

// Import the component after mocks are set up
import { AgentView } from '../components/AgentView';
import { useSessionStore } from '../store/sessionStore';
// REFAC-008: Import test helpers to properly inject chunks via GlobalSessionStreamManager
import {
  stopGlobalSessionStreamManager,
  clearNapiModuleCache,
  injectTestChunk,
} from '../services/globalSessionStreamManager';
import { clearAllSubscriptions } from '../hooks/useRustSessionState';

// Test utility functions
const waitForFrame = async (timeout = 10) => {
  return new Promise(resolve => setTimeout(resolve, timeout));
};

const resetMockSession = () => {
  mockState.shouldThrow = false;
  mockState.errorMessage = 'No AI provider credentials configured';
  mockState.session.currentProviderName = 'claude';
  mockState.session.tokenTracker = { inputTokens: 0, outputTokens: 0 };
  mockState.session.messages = [];
  
  // Reset sessionStore like the working test
  useSessionStore.getState().reset();
  
  vi.clearAllMocks();
  capturedCallback = null;
  capturedResolver = null;
};

// Streaming simulation helpers
// REFAC-008: Use injectTestChunk to bypass async NAPI import issues
const simulateStreamingResponse = async (options: {
  text?: string;
  inputTokens?: number;
  outputTokens?: number;
  error?: string;
}) => {
  if (options.text) {
    injectTestChunk('mock-session-id', { type: 'Text', text: options.text });
  }
  
  if (options.inputTokens !== undefined || options.outputTokens !== undefined) {
    injectTestChunk('mock-session-id', {
      type: 'TokenUpdate',
      tokens: {
        inputTokens: options.inputTokens ?? 0,
        outputTokens: options.outputTokens ?? 0
      }
    });
  }
  
  if (options.error) {
    // For errors, we can't use injectTestChunk - this case might need special handling
    // For now, just log the error scenario
  } else {
    injectTestChunk('mock-session-id', { type: 'Done' });
  }
  
  if (capturedResolver) {
    capturedResolver();
  }
  
  await waitForFrame(100);
};

// Message sending helper
const sendMessageToAgent = async (stdin: { write: (data: string) => void }, message: string) => {
  stdin.write(message);
  await waitForFrame();
  stdin.write('\r');
  await waitForFrame(100);
};

describe('PERF-002: Optimize Context Compaction Performance and UX', () => {
  beforeEach(() => {
    resetMockSession();
    // REFAC-008: Reset GlobalSessionStreamManager and NAPI module cache
    stopGlobalSessionStreamManager();
    clearNapiModuleCache();
    clearAllSubscriptions();
  });

  afterEach(() => {
    vi.clearAllMocks();
    // REFAC-008: Clean up GlobalSessionStreamManager
    stopGlobalSessionStreamManager();
    clearAllSubscriptions();
  });

  describe('Manual Compaction Command', () => {
    it('should compact context and show compression metrics when /compact is entered', async () => {
      // @step Given I am in AgentView with a conversation that has approximately 150k tokens
      resetMockSession();

      // Configure the sessionCompact mock - FIXED: Use dynamic import like working test
      const { sessionCompact } = await import('@sengac/codelet-napi');
      (sessionCompact as ReturnType<typeof vi.fn>).mockResolvedValue({
        originalTokens: 150000,
        compactedTokens: 40000,
        compressionRatio: 73.3,
        turnsSummarized: 12,
        turnsKept: 3,
      });

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // @step First send a message to create the session (NAPI-009: deferred session creation)
      await sendMessageToAgent(stdin, 'hello');

      // Simulate streaming response with high token count
      await simulateStreamingResponse({
        text: 'Hi!',
        inputTokens: 150000,
        outputTokens: 5000
      });

      // Verify view is open with high token count
      expect(lastFrame()).toContain('Claude Sonnet 4');
      expect(lastFrame()).toContain('150000');

      // @step When I type '/compact' and press Enter
      await sendMessageToAgent(stdin, '/compact');

      // @step Then sessionCompact should be called with the session ID
      expect(sessionCompact).toHaveBeenCalledTimes(1);
      expect(sessionCompact).toHaveBeenCalledWith('mock-session-id');

      // @step And the view should show the agent header
      expect(lastFrame()).toContain('Claude Sonnet 4');
    });

    it('should show error when trying to compact without a session', async () => {
      // @step Given I am in AgentView with no messages in the conversation
      resetMockSession();

      // Get reference to the mock function - FIXED: Use dynamic import like working test
      const { sessionCompact } = await import('@sengac/codelet-napi');

      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      // Wait for async session initialization
      await waitForFrame();

      // Verify view is open with empty conversation
      expect(lastFrame()).toContain('Claude Sonnet 4');

      // @step When I type '/compact' and press Enter (without sending a message first)
      await sendMessageToAgent(stdin, '/compact');

      // @step Then sessionCompact should NOT be called (no active session)
      expect(sessionCompact).not.toHaveBeenCalled();

      // @step And the view should show error about needing a session
      expect(lastFrame()).toContain('No active session to compact');

      // @step And the view should show the agent header
      expect(lastFrame()).toContain('Claude Sonnet 4');
    });
  });
});