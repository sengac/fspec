/**
 * Real Session Fixture - Minimal mocking, maximum integration testing
 *
 * This fixture creates REAL sessions that work with the actual session store,
 * React hooks, and component infrastructure. Only the NAPI boundary is mocked.
 *
 * Session streaming uses GlobalSessionStreamManager with global chunk callback.
 */

import { vi } from 'vitest';

// Track callback and resolver at module level for test control
let capturedCallback: ((err: Error | null, chunk: unknown) => void) | null =
  null;
let capturedResolver: (() => void) | null = null;

// Mock state that persists across mock hoisting (following working pattern)
const mockState = vi.hoisted(() => ({
  sessionId: 'test-session-id',
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
    compact: vi.fn(),
  },
  shouldThrow: false,
  errorMessage: 'Mock error',
}));

/**
 * Reset session fixture between tests
 */
export function resetSessionFixture() {
  mockState.sessionId = `test-session-${Date.now()}`;
  mockState.shouldThrow = false;
  mockState.session.tokenTracker = { inputTokens: 0, outputTokens: 0 };
  mockState.session.messages = [];
  capturedCallback = null;
  capturedResolver = null;
  vi.clearAllMocks();
}

/**
 * Get the current session ID from fixture
 */
export function getFixtureSessionId(): string {
  return mockState.sessionId;
}

/**
 * Access captured streaming callback for test control
 */
export function getStreamingCallback():
  | ((err: Error | null, chunk: unknown) => void)
  | null {
  return capturedCallback;
}

/**
 * Access captured resolver for test control
 */
export function getStreamingResolver(): (() => void) | null {
  return capturedResolver;
}

/**
 * Configure session fixture for error scenarios
 */
export function configureFixtureError(
  shouldThrow: boolean,
  errorMessage = 'Mock error'
) {
  mockState.shouldThrow = shouldThrow;
  mockState.errorMessage = errorMessage;
}

/**
 * Get the hoisted mock state for advanced configuration
 */
export function getFixtureState() {
  return mockState;
}

/**
 * Set the captured callback for streaming simulation
 * Used by GlobalSessionStreamManager tests
 */
export function setStreamingCallback(
  callback: (err: Error | null, chunk: unknown) => void
) {
  capturedCallback = callback;
}

/**
 * Standard NAPI mocks that create real sessions
 */
export const createSessionNAPIMocks = () => ({
  // Core session management - creates REAL sessions
  persistenceCreateSessionWithProvider: vi.fn(() => ({
    id: mockState.sessionId,
    name: 'Test Session',
    project: '/test/project',
    provider: 'claude',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    messageCount: 0,
  })),

  // Session input - streaming responses come via global callback
  sessionSendInput: vi
    .fn()
    .mockImplementation(
      (_sessionId: string, _input: string, _thinkingConfig: string | null) => {
        return new Promise(resolve => {
          capturedResolver = resolve;
          // Note: Tests should use setStreamingCallback and simulateStreamingResponse
        });
      }
    ),

  // PERF-002: Compaction functionality
  sessionCompact: vi.fn().mockResolvedValue({
    originalTokens: 8500,
    compactedTokens: 3200,
    compressionRatio: 62.4,
    turnsSummarized: 25,
    turnsKept: 15,
  }),

  // Session state functions
  sessionGetModel: vi
    .fn()
    .mockReturnValue({ providerId: 'anthropic', modelId: 'claude-sonnet-4' }),
  sessionGetStatus: vi.fn().mockReturnValue('Ready'),
  sessionGetTokens: vi
    .fn()
    .mockReturnValue({ inputTokens: 0, outputTokens: 0 }),
  sessionGetDebugEnabled: vi.fn().mockReturnValue(false),

  // Session lifecycle
  sessionManagerList: vi.fn().mockReturnValue([]),
  sessionGetBufferedOutput: vi.fn().mockReturnValue([]),
  sessionManagerDestroy: vi.fn(),
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionRestoreMessages: vi.fn(),
  sessionRestoreTokenState: vi.fn(),
  sessionToggleDebug: vi.fn(),
  sessionSetModel: vi.fn().mockResolvedValue(undefined),
  sessionInterrupt: vi.fn(),
  sessionGetBaseThinkingLevel: vi.fn().mockReturnValue(0),
  sessionSetBaseThinkingLevel: vi.fn(),
  sessionGetParent: vi.fn().mockReturnValue(null),
  sessionGetWatchers: vi.fn().mockReturnValue([]),

  // Required exports for compatibility
  ChunkType: {
    Text: 'Text',
    Thinking: 'Thinking',
    ToolCall: 'ToolCall',
    ToolResult: 'ToolResult',
    Done: 'Done',
    Error: 'Error',
  },
  JsThinkingLevel: {
    Off: 0,
    Low: 1,
    Medium: 2,
    High: 3,
  },
  getThinkingConfig: vi.fn(() => null),
});

/**
 * Simulate streaming response with proper callback handling
 */
export function simulateStreamingResponse(
  chunks: Array<{
    type: string;
    text?: string;
    tokens?: { inputTokens: number; outputTokens: number };
    error?: string;
  }>
) {
  if (!capturedCallback) {
    throw new Error('No callback captured - call setStreamingCallback first');
  }

  chunks.forEach(chunk => {
    if (capturedCallback) {
      if (chunk.error) {
        capturedCallback(new Error(chunk.error), null);
      } else {
        capturedCallback(null, chunk);
      }
    }
  });

  if (capturedResolver) {
    capturedResolver();
  }
}

/**
 * Standard wait helper for tests
 */
export const waitForFrame = (ms = 50) =>
  new Promise(resolve => setTimeout(resolve, ms));
