/**
 * Reusable test fixture for AgentView component testing
 * Extracted from working AgentView.test.tsx pattern
 *
 * Session streaming uses GlobalSessionStreamManager with global chunk callback.
 */

import { vi } from 'vitest';

// Mock model data matching models.dev structure
export const mockModels = {
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
};

// Track callback and resolver at module level for test control
export let capturedCallback:
  | ((err: Error | null, chunk: unknown) => void)
  | null = null;
export let capturedResolver: (() => void) | null = null;

// Create mock state that persists across mock hoisting
export const mockState = {
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
      message:
        'Debug capture started. Writing to: ~/.fspec/debug/session-2025-01-01T00-00-00.jsonl',
    }),
    // NAPI-005: Manual compaction command
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
};

// Type for fspecStore state (matches FspecState in fspecStore.ts)
// Only includes properties commonly used in AgentView tests
export interface MockFspecState {
  cwd: string;
  workUnits: Array<{
    id: string;
    title: string;
    status: string;
    type: string;
    description?: string;
  }>;
  selectedWorkUnitId: string | null;
  setWorkUnits: ReturnType<typeof vi.fn>;
  loadData: ReturnType<typeof vi.fn>;
  getWorkUnitBySession: ReturnType<typeof vi.fn>;
  detachSession: ReturnType<typeof vi.fn>;
  getAttachedSession: ReturnType<typeof vi.fn>;
  setCurrentWorkUnitId: ReturnType<typeof vi.fn>;
}

// Default fspecStore state for tests
// Used by useWorkUnitsWatcher hook which requires cwd
export const createFspecStoreMock = (
  overrides: Partial<MockFspecState> = {}
) => {
  const defaultState: MockFspecState = {
    cwd: '/tmp/fspec-test-project',
    workUnits: [],
    selectedWorkUnitId: null,
    setWorkUnits: vi.fn(),
    loadData: vi.fn(),
    getWorkUnitBySession: vi.fn().mockReturnValue(undefined),
    detachSession: vi.fn(),
    getAttachedSession: vi.fn().mockReturnValue(null),
    setCurrentWorkUnitId: vi.fn(),
    ...overrides,
  };

  return (selector?: (state: MockFspecState) => unknown) => {
    return selector ? selector(defaultState) : defaultState;
  };
};

/**
 * Set the captured callback for streaming simulation
 * Used by GlobalSessionStreamManager tests
 */
export function setStreamingCallback(
  callback: (err: Error | null, chunk: unknown) => void
) {
  capturedCallback = callback;
}

// Complete NAPI module mock - individual functions only
export const createNapiMock = () => ({
  ChunkType: {
    Text: 'Text',
    Thinking: 'Thinking', // TOOL-010
    ToolCall: 'ToolCall',
    ToolResult: 'ToolResult',
    Done: 'Done',
    Error: 'Error',
  },
  // TOOL-010: Thinking level detection exports
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
  napiComputeEffectiveThinkingLevel: vi.fn(
    (base: number, detected: number, forceOff: boolean) => {
      if (forceOff) {
        return 0;
      }
      return Math.max(base, detected);
    }
  ),
  modelsListAll: vi.fn(() =>
    Promise.resolve([mockModels.anthropic, mockModels.openai])
  ),
  setRustLogCallback: vi.fn(),
  // Persistence NAPI bindings required by AgentView
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
  persistenceSetSessionTokens: vi.fn(),
  // Session management
  sessionManagerList: vi.fn().mockReturnValue([]),
  sessionGetBufferedOutput: vi.fn().mockReturnValue([]),
  sessionManagerDestroy: vi.fn(),
  sessionSendInput: vi
    .fn()
    .mockImplementation(
      (_sessionId: string, _input: string, _thinkingConfig: string | null) => {
        // Streaming responses come via GlobalSessionStreamManager global callback
        // Tests should use setStreamingCallback and simulateStreamingResponse
      }
    ),
  // Session manager functions
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionRestoreMessages: vi.fn(),
  sessionRestoreTokenState: vi.fn(),
  // Debug and compaction for background sessions
  sessionToggleDebug: vi.fn().mockResolvedValue({
    enabled: true,
    sessionFile: '/tmp/debug-session.json',
    message:
      'Debug capture enabled. Events will be written to /tmp/debug-session.json',
  }),
  sessionCompact: vi.fn().mockResolvedValue({
    originalTokens: 10000,
    compactedTokens: 3000,
    compressionRatio: 70,
    turnsSummarized: 5,
    turnsKept: 2,
  }),
  // Rust state functions for model, status, and tokens
  sessionGetModel: vi.fn().mockReturnValue({ providerId: null, modelId: null }),
  sessionGetStatus: vi.fn().mockReturnValue('idle'),
  sessionGetTokens: vi
    .fn()
    .mockReturnValue({ inputTokens: 0, outputTokens: 0 }),
  sessionSetModel: vi.fn().mockResolvedValue(undefined),
  sessionSetModelProfile: vi.fn().mockResolvedValue(undefined),
  sessionInterrupt: vi.fn(),
  sessionSetPendingInput: vi.fn(),
  sessionGetPendingInput: vi.fn().mockReturnValue(null),
  // TUI-054: Base thinking level
  sessionGetBaseThinkingLevel: vi.fn().mockReturnValue(0),
  sessionSetBaseThinkingLevel: vi.fn(),
  // Debug enabled state from Rust
  sessionGetDebugEnabled: vi.fn().mockReturnValue(false),
  // Navigation functions for session/watcher navigation
  sessionGetParent: vi.fn().mockReturnValue(null),
  sessionGetWatchers: vi.fn().mockReturnValue([]),
  // UX-002: Compaction progress
  sessionGetCompactionProgress: vi.fn().mockReturnValue(null),
});

// Test utility functions
export const waitForFrame = async (timeout = 10) => {
  return new Promise(resolve => setTimeout(resolve, timeout));
};

export const resetMockSession = () => {
  mockState.shouldThrow = false;
  mockState.errorMessage = 'No AI provider credentials configured';
  mockState.session.currentProviderName = 'claude';
  mockState.session.tokenTracker = { inputTokens: 0, outputTokens: 0 };
  mockState.session.messages = [];

  // Reset all mock functions
  vi.clearAllMocks();
  capturedCallback = null;
  capturedResolver = null;
};

// Streaming simulation helpers
export const simulateStreamingResponse = async (options: {
  text?: string;
  inputTokens?: number;
  outputTokens?: number;
  error?: string;
}) => {
  if (!capturedCallback) return;

  if (options.text) {
    capturedCallback(null, { type: 'Text', text: options.text });
  }

  if (options.inputTokens !== undefined || options.outputTokens !== undefined) {
    capturedCallback(null, {
      type: 'TokenUpdate',
      tokens: {
        inputTokens: options.inputTokens ?? 0,
        outputTokens: options.outputTokens ?? 0,
      },
    });
  }

  if (options.error) {
    capturedCallback(new Error(options.error), null);
  } else {
    capturedCallback(null, { type: 'Done' });
  }

  if (capturedResolver) {
    capturedResolver();
  }

  await waitForFrame(100);
};

// Message sending helper
export const sendMessageToAgent = async (
  stdin: { write: (data: string) => void },
  message: string
) => {
  stdin.write(message);
  await waitForFrame();
  stdin.write('\r');
  await waitForFrame(100);
};
