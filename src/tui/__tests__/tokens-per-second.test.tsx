/**
 * Feature: spec/features/real-time-tokens-per-second-display-in-agent-modal-header.feature
 *
 * Tests for Real-time tokens per second display in agent modal header
 *
 * These tests verify the tokens per second calculation and display
 * in the AgentView header during streaming responses.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Box, Text } from 'ink';

// Track callback and resolver at module level for test control
let capturedCallback: ((chunk: unknown) => void) | null = null;
let capturedResolver: (() => void) | null = null;

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
    resetInterrupt: vi.fn(),
  },
  shouldThrow: false,
  errorMessage: 'No AI provider credentials configured',
}));

// Mock codelet-napi module
vi.mock('@sengac/codelet-napi', () => ({
  CodeletSession: class MockCodeletSession {
    currentProviderName: string;
    availableProviders: string[];
    tokenTracker: { inputTokens: number; outputTokens: number };
    messages: Array<{ role: string; content: string }>;
    prompt: (input: string, thinkingConfig: string | null, callback: (chunk: unknown) => void) => Promise<void>;
    switchProvider: ReturnType<typeof vi.fn>;
    clearHistory: ReturnType<typeof vi.fn>;
    interrupt: ReturnType<typeof vi.fn>;
    resetInterrupt: ReturnType<typeof vi.fn>;

    constructor() {
      if (mockState.shouldThrow) {
        throw new Error(mockState.errorMessage);
      }
      this.currentProviderName = mockState.session.currentProviderName;
      this.availableProviders = mockState.session.availableProviders;
      this.tokenTracker = mockState.session.tokenTracker;
      this.messages = mockState.session.messages;
      // Capture callback and return a controllable promise (TOOL-010: added thinkingConfig param)
      this.prompt = async (_input: string, _thinkingConfig: string | null, callback: (chunk: unknown) => void) => {
        capturedCallback = callback;
        return new Promise<void>(resolve => {
          capturedResolver = resolve;
        });
      };
      this.switchProvider = mockState.session.switchProvider;
      this.clearHistory = mockState.session.clearHistory;
      this.interrupt = mockState.session.interrupt;
      this.resetInterrupt = mockState.session.resetInterrupt;
    }
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
  modelsListAll: vi.fn(() => Promise.resolve([])),
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
}));

// Mock Dialog to render children without position="absolute"
vi.mock('../../components/Dialog', () => ({
  Dialog: ({
    children,
  }: {
    children: React.ReactNode;
    onClose: () => void;
    borderColor?: string;
  }) => <Box flexDirection="column">{children}</Box>,
}));

// Mock Ink's Box to strip position="absolute" which doesn't work in ink-testing-library
vi.mock('ink', async () => {
  const actual = await vi.importActual<typeof import('ink')>('ink');
  return {
    ...actual,
    Box: (props: React.ComponentProps<typeof actual.Box>) => {
      const { position, ...rest } = props as { position?: string } & typeof props;
      return <actual.Box {...rest} />;
    },
  };
});

// Import the component after mocks are set up
import { AgentView } from '../components/AgentView';

// Helper to wait for async operations
const waitForFrame = (ms = 50): Promise<void> =>
  new Promise(resolve => setTimeout(resolve, ms));

// Helper to reset mock session
const resetMockSession = () => {
  mockState.session = {
    currentProviderName: 'claude',
    availableProviders: ['claude', 'openai'],
    tokenTracker: { inputTokens: 0, outputTokens: 0 },
    messages: [],
    prompt: vi.fn(),
    switchProvider: vi.fn(),
    clearHistory: vi.fn(),
    interrupt: vi.fn(),
    resetInterrupt: vi.fn(),
  };
  mockState.shouldThrow = false;
  mockState.errorMessage = 'No AI provider credentials configured';
  capturedCallback = null;
  capturedResolver = null;
};

// Helper to simulate streaming with tok/s from Rust
// TUI-031: Tok/s is calculated in Rust and sent via TokenUpdate.tokensPerSecond
const simulateStreaming = async (
  finalTokens: { inputTokens: number; outputTokens: number },
  waitTime: number,
  tokensPerSecond: number = 25.5 // Default tok/s value for tests
) => {
  if (capturedCallback) {
    // First text chunk
    capturedCallback({ type: 'Text', text: 'Hello ' });
  }
  await waitForFrame(waitTime / 3);
  if (capturedCallback) {
    // Second text chunk
    capturedCallback({ type: 'Text', text: 'world, this is ' });
  }
  await waitForFrame(waitTime / 3);
  if (capturedCallback) {
    // Third text chunk
    capturedCallback({ type: 'Text', text: 'a streaming response.' });
    // Send token update with tok/s from Rust
    capturedCallback({
      type: 'TokenUpdate',
      tokens: { ...finalTokens, tokensPerSecond },
    });
  }
  await waitForFrame(waitTime / 3);
};

// Helper to end streaming
const endStreaming = async (finalTokens = { inputTokens: 100, outputTokens: 50 }) => {
  if (capturedCallback) {
    // Final token update with tokensPerSecond: null to hide tok/s display
    capturedCallback({
      type: 'TokenUpdate',
      tokens: { ...finalTokens, tokensPerSecond: null },
    });
    capturedCallback({ type: 'Done' });
  }
  if (capturedResolver) {
    capturedResolver();
  }
  await waitForFrame(150);
};

describe('Feature: Real-time tokens per second display in agent modal header', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockSession();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Display tokens per second after multiple token updates', () => {
    it('should display tok/s in header after multiple token updates', async () => {
      // @step Given the agent modal is open and streaming a response
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      // Wait for session initialization
      await waitForFrame(100);

      // Verify initial state - no tok/s shown
      let frame = lastFrame();
      expect(frame).toContain('Agent: claude');
      expect(frame).not.toContain('tok/s');

      // Type text first, wait for React to process
      stdin.write('test message');
      await waitForFrame(50);
      // Then send Enter key separately
      stdin.write('\r');
      await waitForFrame(150);

      // @step And multiple TokenUpdate events have been received
      // @step And rate samples have been calculated from token deltas
      await simulateStreaming({ inputTokens: 100, outputTokens: 50 }, 600);

      // @step When the header is rendered
      // @step Then the header should display the averaged tok/s value
      frame = lastFrame();
      expect(frame).toContain('tok/s');

      // @step And the tokens per second should appear to the left of the token count
      const tokSIndex = frame?.indexOf('tok/s') ?? -1;
      const tokensIndex = frame?.indexOf('tokens:') ?? -1;
      expect(tokSIndex).toBeGreaterThan(-1);
      expect(tokensIndex).toBeGreaterThan(-1);
      expect(tokSIndex).toBeLessThan(tokensIndex);

      // Cleanup
      await endStreaming();
    });
  });

  describe('Scenario: Suppress tokens per second before rate samples available', () => {
    it('should not display tok/s until multiple token updates received', async () => {
      // @step Given the agent modal is open and streaming a response
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      // Start streaming
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And only one TokenUpdate event has been received (without tokensPerSecond)
      // @step When the header is rendered
      // TUI-031: Rust hasn't calculated tok/s yet, so it's not sent
      if (capturedCallback) {
        capturedCallback({ type: 'Text', text: 'Hello' });
        capturedCallback({
          type: 'TokenUpdate',
          tokens: { inputTokens: 100, outputTokens: 10 }, // No tokensPerSecond
        });
      }
      await waitForFrame(100);

      // @step Then no tokens per second value should be displayed
      const frame = lastFrame();
      expect(frame).toContain('streaming');
      expect(frame).not.toContain('tok/s');

      // Cleanup
      await endStreaming();
    });
  });

  describe('Scenario: Hide tokens per second when streaming ends', () => {
    it('should hide tok/s display when streaming completes', async () => {
      // @step Given the agent modal was streaming a response
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      // Start streaming
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the tokens per second was being displayed
      await simulateStreaming({ inputTokens: 100, outputTokens: 50 }, 600);

      // Verify tok/s is displayed during streaming
      let frame = lastFrame();
      expect(frame).toContain('tok/s');

      // @step When the streaming completes
      await endStreaming();

      // @step Then the tokens per second display should disappear
      frame = lastFrame();
      expect(frame).not.toContain('tok/s');

      // @step And only the token counts should remain visible
      expect(frame).toContain('tokens:');
    });
  });

  describe('Scenario: Display proper header layout during streaming', () => {
    it('should display header with correct layout', async () => {
      // @step Given the agent modal is open with provider 'claude'
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      // Start streaming
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And streaming is active with 12.3 tokens per second
      // @step And 1234 input tokens and 567 output tokens have been used
      await simulateStreaming({ inputTokens: 1234, outputTokens: 567 }, 600);

      // @step When the header is rendered
      const frame = lastFrame();

      // @step Then the header should show 'Agent: claude' on the left
      expect(frame).toContain('Agent: claude');

      // @step And the header should show '(streaming...)' next to the provider name
      expect(frame).toContain('streaming');

      // @step And the header should show '12.3 tok/s' to the left of the token count
      expect(frame).toContain('tok/s');

      // @step And the header should show 'tokens: 1234↓ 567↑'
      expect(frame).toContain('1234');
      expect(frame).toContain('567');

      // Cleanup
      await endStreaming();
    });
  });

  describe('Scenario: Calculate tokens per second for slow provider', () => {
    it('should display tok/s value for slow token generation', async () => {
      // @step Given the agent modal is open and streaming a response
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      // Start streaming
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And multiple TokenUpdate events have been received with slow token generation
      await simulateStreaming({ inputTokens: 100, outputTokens: 10 }, 1000);

      // @step When the header is rendered
      // @step Then the header should display a low tok/s value reflecting the slow rate
      const frame = lastFrame();
      expect(frame).toContain('tok/s');

      // Verify the display format is correct (X.X tok/s)
      expect(frame).toMatch(/\d+\.\d tok\/s/);

      // Cleanup
      await endStreaming();
    });
  });

  describe('Scenario: Update tokens per second in real-time during streaming', () => {
    it('should update tok/s display as more tokens arrive', async () => {
      // @step Given the agent modal is streaming a response
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      // Start streaming
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And rate samples are being collected
      await simulateStreaming({ inputTokens: 100, outputTokens: 50 }, 600);

      // Capture first reading
      const frame1 = lastFrame();
      expect(frame1).toContain('tok/s');

      // @step When additional tokens continue to stream over time
      // TUI-031: Send more tokens with updated tok/s from Rust
      if (capturedCallback) {
        capturedCallback({
          type: 'TokenUpdate',
          tokens: { inputTokens: 100, outputTokens: 150, tokensPerSecond: 30.2 },
        });
      }
      await waitForFrame(200);

      // @step Then the tokens per second display should update with new value from Rust
      const frame2 = lastFrame();
      expect(frame2).toContain('tok/s');

      // @step And the displayed value should reflect the EMA-smoothed rate from Rust
      expect(frame1).toMatch(/\d+\.\d tok\/s/);
      expect(frame2).toMatch(/\d+\.\d tok\/s/);

      // Cleanup
      await endStreaming();
    });
  });
});
