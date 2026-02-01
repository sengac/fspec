/**
 * Reusable test fixture for AgentView component testing
 * Extracted from working AgentView.test.tsx pattern
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

// Track callback and resolver at module level for test control (NAPI-009)
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

// Complete NAPI module mock - both class and individual functions
export const createNapiMock = () => ({
  CodeletSession: class MockCodeletSession {
    currentProviderName: string;
    availableProviders: string[];
    tokenTracker: { inputTokens: number; outputTokens: number };
    messages: Array<{ role: string; content: string }>;
    prompt: ReturnType<typeof vi.fn>;
    switchProvider: ReturnType<typeof vi.fn>;
    clearHistory: ReturnType<typeof vi.fn>;
    interrupt: ReturnType<typeof vi.fn>;
    toggleDebug: ReturnType<typeof vi.fn>;
    compact: ReturnType<typeof vi.fn>; // NAPI-005

    constructor() {
      if (mockState.shouldThrow) {
        throw new Error(mockState.errorMessage);
      }
      this.currentProviderName = mockState.session.currentProviderName;
      this.availableProviders = mockState.session.availableProviders;
      this.tokenTracker = mockState.session.tokenTracker;
      this.messages = mockState.session.messages;
      this.prompt = mockState.session.prompt;
      this.switchProvider = mockState.session.switchProvider;
      this.clearHistory = mockState.session.clearHistory;
      this.interrupt = mockState.session.interrupt;
      this.toggleDebug = mockState.session.toggleDebug;
      this.compact = mockState.session.compact; // NAPI-005
    }
  },
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
  // TUI-034: Model selection mocks
  modelsSetCacheDirectory: vi.fn(),
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
  // TUI-047: Session management for background sessions
  sessionManagerList: vi.fn().mockReturnValue([]),
  sessionAttach: vi
    .fn()
    .mockImplementation(
      (
        _sessionId: string,
        callback: (err: Error | null, chunk: unknown) => void
      ) => {
        capturedCallback = callback;
      }
    ),
  sessionGetBufferedOutput: vi.fn().mockReturnValue([]),
  sessionManagerDestroy: vi.fn(),
  sessionDetach: vi.fn(),
  sessionSendInput: vi
    .fn()
    .mockImplementation(
      (_sessionId: string, _input: string, _thinkingConfig: string | null) => {
        // NAPI-009: Trigger streaming callback when input is sent (simulates background session response)
        // Note: Tests should call capturedCallback directly to control streaming responses
      }
    ),
  // NAPI-009: New session manager functions
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionRestoreMessages: vi.fn(),
  sessionRestoreTokenState: vi.fn(),
  // NAPI-009 + AGENT-021: Debug and compaction for background sessions
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
  sessionInterrupt: vi.fn(),
  // TUI-054: Base thinking level
  sessionGetBaseThinkingLevel: vi.fn().mockReturnValue(0),
  sessionSetBaseThinkingLevel: vi.fn(),
  // AGENT-021: Debug enabled state from Rust
  sessionGetDebugEnabled: vi.fn().mockReturnValue(false),
  // VIEWNV-001: Navigation functions for session/watcher navigation
  sessionGetParent: vi.fn().mockReturnValue(null),
  sessionGetWatchers: vi.fn().mockReturnValue([]),
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
