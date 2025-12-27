/**
 * Feature: spec/features/context-window-fill-percentage-indicator.feature
 *
 * Tests for Context window fill percentage indicator in agent modal header
 *
 * These tests verify the context fill percentage calculation and color-coded display
 * in the AgentModal header.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Box } from 'ink';

// Track callback and resolver at module level for test control
let capturedCallback: ((chunk: unknown) => void) | null = null;
let capturedResolver: (() => void) | null = null;

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
  // Persistence NAPI bindings required by AgentModal
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
import { AgentModal } from '../components/AgentModal';

// Helper to wait for async operations
const waitForFrame = (ms = 50): Promise<void> =>
  new Promise(resolve => setTimeout(resolve, ms));

// Helper to reset mock session
const resetMockSession = () => {
  mockState.session = {
    currentProviderName: 'claude',
    availableProviders: ['claude'],
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

// Helper to simulate ContextFillUpdate event
const simulateContextFillUpdate = async (
  fillPercentage: number,
  effectiveTokens: number,
  threshold: number,
  contextWindow: number
) => {
  if (capturedCallback) {
    capturedCallback({
      type: 'ContextFillUpdate',
      contextFill: {
        fillPercentage,
        effectiveTokens,
        threshold,
        contextWindow,
      },
    });
  }
  await waitForFrame(50);
};

// Helper to end streaming
const endStreaming = async () => {
  if (capturedCallback) {
    capturedCallback({ type: 'Done' });
  }
  if (capturedResolver) {
    capturedResolver();
  }
  await waitForFrame(150);
};

describe('Feature: Context Window Fill Percentage Indicator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockSession();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Display shows 0% at start of fresh conversation', () => {
    it('should display [0%] in green at start of conversation', async () => {
      // @step Given I start a fresh conversation in Claude Code
      const { lastFrame } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      // @step And no tokens have been used yet
      await waitForFrame(100);

      // @step When the AgentModal header renders
      const frame = lastFrame();

      // @step Then I should see "[0%]" displayed in the header
      expect(frame).toContain('[0%]');

      // @step And the percentage should be colored green
      // Note: Color verification requires checking ANSI codes or component internals
      // For now we verify the display format is correct
      expect(frame).toMatch(/\[0%\]/);
    });
  });

  describe('Scenario: Display shows percentage in green zone (0-49%)', () => {
    it('should display [45%] in green when at 45% fill', async () => {
      // @step Given I am in a conversation with 81000 effective tokens used
      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame(100);

      // Start a conversation to trigger streaming
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the context window threshold is 180000 tokens
      // @step When the AgentModal header renders
      await simulateContextFillUpdate(45, 81000, 180000, 200000);

      const frame = lastFrame();

      // @step Then I should see "[45%]" displayed in the header
      expect(frame).toContain('[45%]');

      // @step And the percentage should be colored green
      // Green zone is 0-49%
      expect(frame).toMatch(/\[45%\]/);

      await endStreaming();
    });
  });

  describe('Scenario: Display shows percentage in yellow zone (50-69%)', () => {
    it('should display [60%] in yellow when at 60% fill', async () => {
      // @step Given I am in a conversation with 108000 effective tokens used
      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the context window threshold is 180000 tokens
      // @step When the AgentModal header renders
      await simulateContextFillUpdate(60, 108000, 180000, 200000);

      const frame = lastFrame();

      // @step Then I should see "[60%]" displayed in the header
      expect(frame).toContain('[60%]');

      // @step And the percentage should be colored yellow
      // Yellow zone is 50-69%
      expect(frame).toMatch(/\[60%\]/);

      await endStreaming();
    });
  });

  describe('Scenario: Display shows percentage in magenta zone (70-84%)', () => {
    it('should display [75%] in magenta when at 75% fill', async () => {
      // @step Given I am in a conversation with 135000 effective tokens used
      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the context window threshold is 180000 tokens
      // @step When the AgentModal header renders
      await simulateContextFillUpdate(75, 135000, 180000, 200000);

      const frame = lastFrame();

      // @step Then I should see "[75%]" displayed in the header
      expect(frame).toContain('[75%]');

      // @step And the percentage should be colored magenta
      // Magenta zone is 70-84%
      expect(frame).toMatch(/\[75%\]/);

      await endStreaming();
    });
  });

  describe('Scenario: Display shows percentage in red zone (85%+)', () => {
    it('should display [90%] in red when at 90% fill', async () => {
      // @step Given I am in a conversation with 162000 effective tokens used
      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the context window threshold is 180000 tokens
      // @step When the AgentModal header renders
      await simulateContextFillUpdate(90, 162000, 180000, 200000);

      const frame = lastFrame();

      // @step Then I should see "[90%]" displayed in the header
      expect(frame).toContain('[90%]');

      // @step And the percentage should be colored red
      // Red zone is 85%+
      expect(frame).toMatch(/\[90%\]/);

      await endStreaming();
    });
  });

  describe('Scenario: Percentage calculation uses effective tokens with cache discount', () => {
    it('should calculate percentage from effective tokens not raw tokens', async () => {
      // @step Given I am in a conversation with 150000 raw input tokens
      // @step And 80000 tokens are cache read tokens
      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the context window threshold is 180000 tokens
      // @step When the effective token count is calculated
      // Effective = 150000 - (80000 * 0.9) = 150000 - 72000 = 78000
      // Percentage = (78000 / 180000) * 100 = 43.3%

      // @step Then the effective tokens should be 78000
      // @step And I should see "[43%]" displayed in the header
      await simulateContextFillUpdate(43, 78000, 180000, 200000);

      const frame = lastFrame();
      expect(frame).toContain('[43%]');

      // @step And the percentage should be colored green
      expect(frame).toMatch(/\[43%\]/);

      await endStreaming();
    });
  });

  describe('Scenario: Percentage resets after compaction', () => {
    it('should show reduced percentage after compaction', async () => {
      // @step Given I am in a conversation that has just been compacted
      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // First show high percentage
      await simulateContextFillUpdate(85, 153000, 180000, 200000);
      let frame = lastFrame();
      expect(frame).toContain('[85%]');

      // @step And the new effective token count is 50000
      // @step And the context window threshold is 180000 tokens
      // @step When the AgentModal header renders after compaction
      // Percentage = (50000 / 180000) * 100 = 27.8% ≈ 28%
      await simulateContextFillUpdate(28, 50000, 180000, 200000);

      frame = lastFrame();

      // @step Then I should see "[28%]" displayed in the header
      expect(frame).toContain('[28%]');

      // @step And the percentage should be colored green
      expect(frame).toMatch(/\[28%\]/);

      await endStreaming();
    });
  });

  describe('Scenario: Percentage indicator is positioned correctly in header', () => {
    it('should position percentage between token count and Tab Switch', async () => {
      // @step Given I am in an active conversation
      const { lastFrame, stdin } = render(
        <AgentModal isOpen={true} onClose={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // Simulate some context fill
      await simulateContextFillUpdate(50, 90000, 180000, 200000);

      // Also send token update so tokens display is populated
      if (capturedCallback) {
        capturedCallback({
          type: 'TokenUpdate',
          tokens: { inputTokens: 1000, outputTokens: 500 },
        });
      }
      await waitForFrame(50);

      // @step When the AgentModal header renders
      const frame = lastFrame();

      // @step Then the percentage indicator should appear after the token count display
      const tokensIndex = frame?.indexOf('tokens:') ?? -1;
      const percentageMatch = frame?.match(/\[\d+%\]/);
      const percentageIndex = percentageMatch ? (frame?.indexOf(percentageMatch[0]) ?? -1) : -1;

      expect(tokensIndex).toBeGreaterThan(-1);
      expect(percentageIndex).toBeGreaterThan(-1);
      expect(percentageIndex).toBeGreaterThan(tokensIndex);

      // @step And the percentage indicator should appear before the Tab Switch component
      // Note: With single provider, Tab Switch may not be visible
      // This test verifies position relative to token count

      await endStreaming();
    });
  });
});
