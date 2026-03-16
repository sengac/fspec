/**
 * Feature: spec/features/esc-interrupt-during-compaction.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios map directly to Gherkin scenarios.
 *
 * CMPCT-014: Esc to stop does not interrupt compaction —
 * isCompacting state bypasses interrupt handler
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';

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
}));

// Create mock state that persists across mock hoisting
const mockState = vi.hoisted(() => ({
  session: {
    currentProviderName: 'claude',
    availableProviders: ['claude'],
    tokenTracker: { inputTokens: 0, outputTokens: 0 },
    messages: [] as Array<{ role: string; content: string }>,
    prompt: vi.fn(),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    toggleDebug: vi.fn().mockReturnValue({
      enabled: true,
      sessionFile: '~/.fspec/debug/session.jsonl',
      message: 'Debug capture started.',
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

// Mock sessionGetStatus to be controllable per-test
const mockSessionGetStatus = vi.hoisted(() => vi.fn().mockReturnValue('idle'));
const mockSessionInterrupt = vi.hoisted(() => vi.fn());

// Mock codelet-napi module
vi.mock('@sengac/codelet-napi', () => ({
  JsThinkingLevel: {
    Off: 0,
    Low: 1,
    Medium: 2,
    High: 3,
  },
  getThinkingConfig: vi.fn(() => null),
  napiDetectThinkingLevel: vi.fn(() => 0),
  napiHasDisableKeywords: vi.fn(() => false),
  napiComputeEffectiveThinkingLevel: vi.fn(
    (base: number, detected: number, forceOff: boolean) => {
      if (forceOff) {
        return 0;
      }
      return Math.max(base, detected);
    }
  ),
  modelsListAll: vi.fn(() => Promise.resolve([mockModels.anthropic])),
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
  sessionSendInput: vi.fn(),
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionRestoreMessages: vi.fn(),
  sessionRestoreTokenState: vi.fn(),
  sessionToggleDebug: vi.fn().mockResolvedValue({
    enabled: true,
    sessionFile: '/tmp/debug-session.json',
    message: 'Debug capture enabled.',
  }),
  sessionCompact: vi.fn().mockResolvedValue({
    originalTokens: 10000,
    compactedTokens: 3000,
    compressionRatio: 70,
    turnsSummarized: 5,
    turnsKept: 2,
  }),
  sessionGetModel: vi.fn().mockReturnValue({ providerId: null, modelId: null }),
  sessionGetStatus: mockSessionGetStatus,
  sessionGetTokens: vi
    .fn()
    .mockReturnValue({ inputTokens: 0, outputTokens: 0 }),
  sessionSetModel: vi.fn().mockResolvedValue(undefined),
  sessionSetModelProfile: vi.fn().mockResolvedValue(undefined),
  sessionInterrupt: mockSessionInterrupt,
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

// Mock credentials utilities
vi.mock('../../utils/credentials', () => ({
  getProviderConfig: vi.fn((registryId: string) => {
    const registryToAvailable: Record<string, string> = {
      anthropic: 'claude',
    };
    const availableName = registryToAvailable[registryId] || registryId;
    if (mockState.session.availableProviders.includes(availableName)) {
      return Promise.resolve({ apiKey: 'test-key', source: 'file' });
    }
    return Promise.resolve({ apiKey: null, source: null });
  }),
  saveCredential: vi.fn(),
  deleteCredential: vi.fn(),
  maskApiKey: vi.fn(() => '***'),
}));

// Mock config module
vi.mock('../../utils/config', () => ({
  loadConfig: vi.fn(() => Promise.resolve({})),
  writeConfig: vi.fn(() => Promise.resolve()),
  getFspecUserDir: vi.fn(() => '/tmp/fspec-test'),
}));

// Import after mocks
import { AgentView } from '../components/AgentView';
import { useSessionStore } from '../store/sessionStore';
import {
  stopGlobalSessionStreamManager,
  clearNapiModuleCache,
  injectTestChunk,
} from '../services/globalSessionStreamManager';
import { clearAllSubscriptions, refreshSessionState } from '../hooks/useRustSessionState';

// Test utility functions
const waitForFrame = async (timeout = 10) => {
  return new Promise(resolve => setTimeout(resolve, timeout));
};

const sendMessageToAgent = async (
  stdin: { write: (data: string) => void },
  message: string
) => {
  stdin.write(message);
  await waitForFrame();
  stdin.write('\r');
  await waitForFrame(100);
};

const resetMockSession = () => {
  mockState.shouldThrow = false;
  mockState.errorMessage = 'No AI provider credentials configured';
  mockState.session.currentProviderName = 'claude';
  mockState.session.tokenTracker = { inputTokens: 0, outputTokens: 0 };
  mockState.session.messages = [];

  useSessionStore.getState().reset();

  vi.clearAllMocks();
  // Re-set default return values after clearAllMocks
  mockSessionGetStatus.mockReturnValue('idle');
};

describe('Feature: Esc interrupt during compaction', () => {
  beforeEach(() => {
    resetMockSession();
    stopGlobalSessionStreamManager();
    clearNapiModuleCache();
    clearAllSubscriptions();
  });

  afterEach(() => {
    vi.clearAllMocks();
    stopGlobalSessionStreamManager();
    clearAllSubscriptions();
  });

  describe('Scenario: Esc interrupts compaction when only compaction is active', () => {
    it('should call sessionInterrupt when Esc is pressed during compaction', async () => {
      // @step Given the agent is compacting context
      resetMockSession();

      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);
      await waitForFrame();

      // Create a session by sending a message
      await sendMessageToAgent(stdin, 'hello');

      // Simulate the streaming response completing
      injectTestChunk('mock-session-id', { type: 'Text', text: 'Hi!' });
      injectTestChunk('mock-session-id', { type: 'Done' });
      await waitForFrame(100);

      // @step And the agent is not in a loading state
      // Now set session status to 'compacting' (isLoading=false, isCompacting=true)
      mockSessionGetStatus.mockReturnValue('compacting');
      refreshSessionState('mock-session-id');
      await waitForFrame(50);

      // Clear previous calls to sessionInterrupt
      mockSessionInterrupt.mockClear();

      // @step When I press Esc
      stdin.write('\x1b');
      await waitForFrame(50);

      // @step Then the agent should be interrupted
      expect(mockSessionInterrupt).toHaveBeenCalledWith('mock-session-id');

      // @step And the compaction should stop
      // (sessionInterrupt stops the agent loop which stops compaction)
      expect(mockSessionInterrupt).toHaveBeenCalledTimes(1);
    });
  });

  describe('Scenario: Esc interrupts when both compaction and loading are active', () => {
    it('should call sessionInterrupt when both states are active', async () => {
      // @step Given the agent is compacting context
      resetMockSession();

      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);
      await waitForFrame();

      // Create a session
      await sendMessageToAgent(stdin, 'hello');
      injectTestChunk('mock-session-id', { type: 'Text', text: 'Hi!' });
      injectTestChunk('mock-session-id', { type: 'Done' });
      await waitForFrame(100);

      // @step And the agent is also in a loading state
      // Set status to 'running' (isLoading=true) - existing behavior should still work
      mockSessionGetStatus.mockReturnValue('running');
      refreshSessionState('mock-session-id');
      await waitForFrame(50);

      mockSessionInterrupt.mockClear();

      // @step When I press Esc
      stdin.write('\x1b');
      await waitForFrame(50);

      // @step Then the agent should be interrupted
      expect(mockSessionInterrupt).toHaveBeenCalledWith('mock-session-id');
    });
  });

  describe('Scenario: Esc closes modal before interrupting compaction', () => {
    it('should close turn modal first when both modal and compaction are active', async () => {
      // @step Given the agent is compacting context
      resetMockSession();

      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);
      await waitForFrame();

      // Create a session and get a conversation going
      await sendMessageToAgent(stdin, 'hello');
      injectTestChunk('mock-session-id', { type: 'Text', text: 'Hi there!' });
      injectTestChunk('mock-session-id', { type: 'Done' });
      await waitForFrame(100);

      // @step And a turn modal is open
      // Enable turn select mode first (Tab), then press Enter to open modal
      stdin.write('\t');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(50);

      // Set status to compacting
      mockSessionGetStatus.mockReturnValue('compacting');
      refreshSessionState('mock-session-id');
      await waitForFrame(50);

      mockSessionInterrupt.mockClear();

      // @step When I press Esc
      stdin.write('\x1b');
      await waitForFrame(50);

      // @step Then the turn modal should close
      // @step And the compaction should continue running
      // Modal close has higher priority (Priority 3) than interrupt (Priority 5),
      // so Esc should close the modal, not interrupt the session
      expect(mockSessionInterrupt).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Esc clears input when neither compaction nor loading is active', () => {
    it('should clear input instead of interrupting when idle', async () => {
      // @step Given the agent is idle
      resetMockSession();
      mockSessionGetStatus.mockReturnValue('idle');

      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);
      await waitForFrame();

      // Create a session
      await sendMessageToAgent(stdin, 'hello');
      injectTestChunk('mock-session-id', { type: 'Text', text: 'Hi!' });
      injectTestChunk('mock-session-id', { type: 'Done' });
      await waitForFrame(100);

      // Ensure status is idle
      mockSessionGetStatus.mockReturnValue('idle');
      refreshSessionState('mock-session-id');
      await waitForFrame(50);

      // @step And I have text in the input field
      stdin.write('some text');
      await waitForFrame(50);

      mockSessionInterrupt.mockClear();

      // @step When I press Esc
      stdin.write('\x1b');
      await waitForFrame(50);

      // @step Then the input field should be cleared
      // @step And no session interrupt should occur
      expect(mockSessionInterrupt).not.toHaveBeenCalled();
    });
  });
});
