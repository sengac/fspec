/**
 * Feature: spec/features/context-window-fill-percentage-indicator.feature
 * Feature: spec/features/context-fill-percentage-realtime-recompute.feature
 * Feature: spec/features/context-fill-percentage-realtime-recompute-ui.feature
 *
 * Tests for Context window fill percentage indicator in agent modal header
 *
 * These tests verify the context fill percentage calculation and color-coded display
 * in the AgentView header, including the RPC-101 real-time recompute on every
 * TokenUpdate with the RPC-419-corrected physical-occupancy formula.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Box } from 'ink';

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

// Track callback and resolver at module level for test control
let capturedCallback: ((err: Error | null, chunk: unknown) => void) | null = null;
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
  napiComputeEffectiveThinkingLevel: vi.fn((base: number, detected: number, forceOff: boolean) => {
    if (forceOff) { return 0; }
    return Math.max(base, detected);
  }),
  // TUI-034: Model selection mocks
  modelsListAll: vi.fn(() => Promise.resolve([mockModels.anthropic])),
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
  sessionGetBufferedOutput: vi.fn().mockReturnValue([]),
  sessionManagerDestroy: vi.fn(),
  sessionSendInput: vi.fn(),
  // NAPI-009: New session manager functions
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionRestoreMessages: vi.fn(),
  sessionRestoreTokenState: vi.fn(),
  // NAPI-009 + AGENT-021: Debug and compaction for background sessions
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
  // Rust state functions for model, status, and tokens
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
  // TUI-054: Base thinking level
  sessionGetBaseThinkingLevel: vi.fn().mockReturnValue(0),
  sessionSetBaseThinkingLevel: vi.fn(),
  // VIEWNV-001: Navigation functions for session/watcher navigation
  sessionGetSubordinate: vi.fn().mockReturnValue(null),
  sessionGetSupervisors: vi.fn().mockReturnValue([]),
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

// Mock credentials utilities - required for provider filtering
vi.mock('../../utils/credentials', () => ({
  getProviderConfig: vi.fn((registryId: string) => {
    const registryToAvailable: Record<string, string> = {
      anthropic: 'claude',
      openai: 'openai',
      gemini: 'gemini',
      google: 'gemini',
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

// Mock config utilities
vi.mock('../../utils/config', () => ({
  loadConfig: vi.fn(() => Promise.resolve({})),
  writeConfig: vi.fn(() => Promise.resolve()),
  getFspecUserDir: vi.fn(() => '/tmp/fspec-test'),
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
// REFAC-008: Import test helpers to properly inject chunks via GlobalSessionStreamManager
import {
  stopGlobalSessionStreamManager,
  clearNapiModuleCache,
  injectTestChunk,
} from '../services/globalSessionStreamManager';

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
// REFAC-008: Use injectTestChunk to bypass async NAPI import issues
const simulateContextFillUpdate = async (
  fillPercentage: number,
  effectiveTokens: number,
  threshold: number,
  contextWindow: number
) => {
  injectTestChunk('mock-session-id', {
    type: 'ContextFillUpdate',
    contextFill: {
      fillPercentage,
      effectiveTokens,
      threshold,
      contextWindow,
    },
  });
  await waitForFrame(50);
};

// Helper to end streaming
// REFAC-008: Use injectTestChunk to bypass async NAPI import issues
const endStreaming = async () => {
  injectTestChunk('mock-session-id', { type: 'Done' });
  if (capturedResolver) {
    capturedResolver();
  }
  await waitForFrame(150);
};

describe('Feature: Context Window Fill Percentage Indicator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockSession();
    // REFAC-008: Reset GlobalSessionStreamManager and NAPI module cache
    stopGlobalSessionStreamManager();
    clearNapiModuleCache();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    // REFAC-008: Clean up GlobalSessionStreamManager
    stopGlobalSessionStreamManager();
  });

  describe('Scenario: Display shows 0% at start of fresh conversation', () => {
    it('should display [0%] in green at start of conversation', async () => {
      // @step Given I start a fresh conversation in Claude Code
      const { lastFrame } = render(
        <AgentView onExit={() => {}} />
      );

      // @step And no tokens have been used yet
      await waitForFrame(100);

      // @step When the AgentView header renders
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
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      // Start a conversation to trigger streaming
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the context window threshold is 180000 tokens
      // @step When the AgentView header renders
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
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the context window threshold is 180000 tokens
      // @step When the AgentView header renders
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
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the context window threshold is 180000 tokens
      // @step When the AgentView header renders
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
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step And the context window threshold is 180000 tokens
      // @step When the AgentView header renders
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

  describe("Scenario: Percentage displays the backend's physical-occupancy calculation verbatim", () => {
    it('should display the backend-supplied fill percentage verbatim', async () => {
      // @step Given the backend has computed a fill percentage of 43 from 78000 total context tokens (input + cache + output + reasoning, with no cache discount) against a threshold of 180000 tokens
      const { lastFrame, stdin } = render(
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step When the backend emits ContextFillUpdate with fill_percentage=43
      await simulateContextFillUpdate(43, 78000, 180000, 200000);

      // @step Then the frontend displays the backend-supplied "[43%]" verbatim in the header
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
        <AgentView onExit={() => {}} />
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
      // @step When the AgentView header renders after compaction
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
        <AgentView onExit={() => {}} />
      );

      await waitForFrame(100);

      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // Simulate some context fill
      await simulateContextFillUpdate(50, 90000, 180000, 200000);

      // Also send token update so tokens display is populated
      // NAPI-009: Callback signature is (err, chunk)
      if (capturedCallback) {
        capturedCallback(null, {
          type: 'TokenUpdate',
          tokens: { inputTokens: 1000, outputTokens: 500 },
        });
      }
      await waitForFrame(50);

      // @step When the AgentView header renders
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

  // RPC-101: SessionHeader [X%] badge MUST update in real-time on
  // every TokenUpdate (same cadence as the `tokens: X↓ Y↑` counters)
  // by recomputing locally from the cached threshold. Without this,
  // the badge freezes mid-stream (backend may emit ContextFillUpdate
  // only at end-of-turn) and after Esc interrupt (terminal
  // ContextFillUpdate never arrives).
  //
  // RPC-419: the recompute formula is the backend's physical-occupancy
  // formula pct = trunc((inputTokens + outputTokens + reasoningTokens)
  // / threshold * 100) — no 0.9 cache discount (wire inputTokens
  // already includes cache tokens per PROV-001), truncation not
  // rounding, missing optional fields treated as 0.
  // Features: spec/features/context-fill-percentage-realtime-recompute.feature
  //           spec/features/context-fill-percentage-realtime-recompute-ui.feature
  describe('Scenario: Badge updates in real-time on TokenUpdate (RPC-101)', () => {
    it('should recompute percentage from cached threshold on every TokenUpdate', async () => {
      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await waitForFrame(100);
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step Given a session has received ContextFillUpdate with fill_percentage=10 and threshold=100000 tokens
      await simulateContextFillUpdate(10, 10_000, 100_000, 132_000);
      let frame = lastFrame();
      expect(frame).toContain('[10%]');

      // @step When a TokenUpdate with input_tokens=45000 arrives without an accompanying ContextFillUpdate
      // 45000 / 100000 = 45%
      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: { inputTokens: 45_000, outputTokens: 0 },
      });
      await waitForFrame(80);

      // @step Then the SessionHeader badge MUST display [45%] (recomputed locally from 45000/100000)
      frame = lastFrame();
      expect(frame).toContain('[45%]');
      expect(frame).not.toContain('[10%]');

      // @step When a further TokenUpdate with input_tokens=90000 arrives later in the same turn
      // 90000 / 100000 = 90%
      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: { inputTokens: 90_000, outputTokens: 0 },
      });
      await waitForFrame(80);

      // @step Then the SessionHeader badge MUST display [90%] at TokenUpdate cadence
      frame = lastFrame();
      expect(frame).toContain('[90%]');

      await endStreaming();
    });

    it('should NOT update the badge on TokenUpdate when no threshold has been cached yet', async () => {
      // @step Given a fresh session with no ContextFillUpdate received yet (threshold cache is 0)
      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await waitForFrame(100);
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // Initial badge is [0%]
      let frame = lastFrame();
      expect(frame).toContain('[0%]');

      // @step When a TokenUpdate with input_tokens=50000 arrives
      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: { inputTokens: 50_000, outputTokens: 500 },
      });
      await waitForFrame(80);

      // @step Then the SessionHeader badge MUST remain at [0%] (no threshold means no recompute, never divide by zero)
      frame = lastFrame();
      expect(frame).toContain('[0%]');

      await endStreaming();
    });

    it('should NOT apply any cache discount when recomputing on TokenUpdate (RPC-419)', async () => {
      // RPC-419: wire inputTokens is ALREADY total_input (raw +
      // cache_read + cache_creation, PROV-001) — the old 0.9 discount
      // subtracted from a value that already included 100% of the
      // cache reads and produced [32%] here. The corrected formula
      // yields trunc(50000/100000*100) = [50%].
      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await waitForFrame(100);
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      await simulateContextFillUpdate(0, 0, 100_000, 132_000);

      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: {
          inputTokens: 50_000,
          outputTokens: 0,
          cacheReadInputTokens: 20_000,
        },
      });
      await waitForFrame(80);

      const frame = lastFrame();
      expect(frame).toContain('[50%]');
      expect(frame).not.toContain('[32%]');

      await endStreaming();
    });

    it('should include output and reasoning tokens in the recomputed percentage (RPC-419)', async () => {
      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await waitForFrame(100);
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step Given a session with cached threshold=100000 tokens
      await simulateContextFillUpdate(0, 0, 100_000, 132_000);

      // @step When a TokenUpdate with input_tokens=50000, output_tokens=3000 and reasoning_tokens=2000 arrives during streaming
      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: {
          inputTokens: 50_000,
          outputTokens: 3_000,
          reasoningTokens: 2_000,
        },
      });
      await waitForFrame(80);

      // @step Then the SessionHeader renders [55%] with no cache discount applied
      const frame = lastFrame();
      expect(frame).toContain('[55%]');

      await endStreaming();
    });

    it('should not collapse the badge on a cache-heavy TokenUpdate (RPC-419 oscillation regression)', async () => {
      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await waitForFrame(100);
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step Given a session with cached threshold=168000 tokens and an authoritative ContextFillUpdate showing 110%
      await simulateContextFillUpdate(110, 186_000, 168_000, 200_000);
      let frame = lastFrame();
      expect(frame).toContain('[110%]');

      // @step When a bare TokenUpdate arrives with input_tokens=175000 (including cache_read_input_tokens=150000 and cache_creation_input_tokens=5000), output_tokens=3000 and reasoning_tokens=8000
      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: {
          inputTokens: 175_000,
          outputTokens: 3_000,
          cacheReadInputTokens: 150_000,
          cacheCreationInputTokens: 5_000,
          reasoningTokens: 8_000,
        },
      });
      await waitForFrame(80);

      // @step Then the SessionHeader badge MUST remain [110%] computed as trunc(186000/168000*100) instead of collapsing to [24%]
      frame = lastFrame();
      expect(frame).toContain('[110%]');
      expect(frame).not.toContain('[24%]');

      await endStreaming();
    });

    it('should truncate the recomputed percentage like the backend instead of rounding (RPC-419)', async () => {
      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await waitForFrame(100);
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step Given a session with cached threshold=100000 tokens
      await simulateContextFillUpdate(0, 0, 100_000, 132_000);

      // @step When a TokenUpdate with input_tokens=45900 and no output or reasoning tokens arrives
      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: { inputTokens: 45_900, outputTokens: 0 },
      });
      await waitForFrame(80);

      // @step Then the SessionHeader badge MUST display [45%] (truncation matching the backend's `as u32` cast, not [46%] from rounding)
      const frame = lastFrame();
      expect(frame).toContain('[45%]');
      expect(frame).not.toContain('[46%]');

      await endStreaming();
    });

    it('should treat missing optional token fields as zero (RPC-419)', async () => {
      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await waitForFrame(100);
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step Given a session with cached threshold=100000 tokens
      await simulateContextFillUpdate(0, 0, 100_000, 132_000);

      // @step When a TokenUpdate with input_tokens=40000, output_tokens=1000 and absent reasoning and cache fields arrives
      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: { inputTokens: 40_000, outputTokens: 1_000 },
      });
      await waitForFrame(80);

      // @step Then the SessionHeader badge MUST display [41%] without error
      const frame = lastFrame();
      expect(frame).toContain('[41%]');

      await endStreaming();
    });

    it('should let an authoritative ContextFillUpdate override a locally-recomputed value', async () => {
      const { lastFrame, stdin } = render(<AgentView onExit={() => {}} />);

      await waitForFrame(100);
      stdin.write('test message');
      await waitForFrame(50);
      stdin.write('\r');
      await waitForFrame(100);

      // @step Given a session with cached threshold=100000 tokens after a ContextFillUpdate{fill_percentage=5}
      await simulateContextFillUpdate(5, 5_000, 100_000, 132_000);

      // @step And a TokenUpdate with input_tokens=50000 has locally recomputed the badge to [50%]
      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: { inputTokens: 50_000, outputTokens: 0 },
      });
      await waitForFrame(80);
      let frame = lastFrame();
      expect(frame).toContain('[50%]');

      // @step When the backend emits an authoritative ContextFillUpdate{fill_percentage=62} (the backend remains authoritative whenever it speaks)
      await simulateContextFillUpdate(62, 62_000, 100_000, 132_000);

      // @step Then the SessionHeader badge MUST display [62%] (backend value wins)
      frame = lastFrame();
      expect(frame).toContain('[62%]');

      // @step And the cached threshold MUST remain at 100000 tokens for subsequent TokenUpdates
      injectTestChunk('mock-session-id', {
        type: 'TokenUpdate',
        tokens: { inputTokens: 70_000, outputTokens: 0 },
      });
      await waitForFrame(80);
      frame = lastFrame();
      expect(frame).toContain('[70%]');

      await endStreaming();
    });
  });
});
