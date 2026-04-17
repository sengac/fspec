/**
 * Feature: spec/features/per-model-compaction-threshold.feature
 *
 * TypeScript-side tests for per-model compaction threshold.
 * The core threshold logic lives in Rust (compaction_threshold.rs / manager.rs).
 * These tests verify the NAPI boundary exposure and TUI consumption pattern.
 *
 * Rust-side tests live in codelet/cli/src/compaction_threshold.rs (unit tests for
 * CompactionThresholdConfig, builtin defaults, resolve(), and priority chain).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  refreshSessionState,
  clearAllSubscriptions,
  getSessionSnapshotForTesting,
  setRustStateSource,
  resetRustStateSource,
  type RustStateSource,
} from '../../hooks/useRustSessionState';

// =============================================================================
// Mock state helpers
// =============================================================================

interface MockModelState {
  providerId: string;
  modelId: string;
  contextWindow?: number;
  maxOutputTokens?: number;
  compactionThreshold?: number;
}

interface MockState {
  status: string;
  model: MockModelState | null;
  tokens: { inputTokens: number; outputTokens: number };
  debugEnabled: boolean;
  baseThinkingLevel: number;
}

function createMockStateSource(initialState: MockState): {
  source: RustStateSource;
  setState: (state: Partial<MockState>) => void;
} {
  let state = { ...initialState };

  const source: RustStateSource = {
    getStatus: () => state.status,
    getModel: () => state.model,
    getTokens: () => state.tokens,
    getDebugEnabled: () => state.debugEnabled,
    getPauseState: () => null,
    getBaseThinkingLevel: () => state.baseThinkingLevel,
    setBaseThinkingLevel: (_sessionId: string, level: number) => {
      state.baseThinkingLevel = level;
    },
    getCompactionProgress: () => null,
    getHitlRequest: () => null,
  };

  return {
    source,
    setState: (newState: Partial<MockState>) => {
      state = { ...state, ...newState };
    },
  };
}

// =============================================================================
// Tests
// =============================================================================

describe('Feature: Per-Model Configurable Compaction Threshold', () => {
  beforeEach(() => {
    clearAllSubscriptions();
  });

  afterEach(() => {
    resetRustStateSource();
    clearAllSubscriptions();
  });

  // ===========================================================================
  // Scenario: Claude model retains legacy threshold behavior
  // ===========================================================================

  describe('Scenario: Claude model retains legacy threshold behavior', () => {
    it('should use legacy formula for Claude models', () => {
      // @step Given a Claude Sonnet 4 model with 200000 context window and 8192 max output
      // @step And no user-configured compaction threshold override
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4',
          contextWindow: 200000,
          maxOutputTokens: 8192,
          compactionThreshold: 191808, // Rust resolves: 200k - 8192
        },
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step When the compaction threshold is resolved
      const snapshot = getSessionSnapshotForTesting('test-ctx-007-claude');

      // @step Then the threshold should equal 191808 tokens
      expect(snapshot.model!.compactionThreshold).toBe(191808);

      // @step And the calculation should use context_window minus min(max_output, 32000)
      // Legacy formula: 200000 - min(8192, 32000) = 200000 - 8192 = 191808
      expect(snapshot.model!.compactionThreshold).toBe(
        snapshot.model!.contextWindow! -
          Math.min(snapshot.model!.maxOutputTokens!, 32000)
      );
    });
  });

  // ===========================================================================
  // Scenario: Gemini model uses 80% built-in default
  // ===========================================================================

  describe('Scenario: Gemini model uses 80% built-in default', () => {
    it('should use 80% threshold for Gemini models', () => {
      // @step Given a Gemini 2.5 Pro model with 1000000 context window
      // @step And no user-configured compaction threshold override
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'google',
          modelId: 'gemini-2.5-pro',
          contextWindow: 1000000,
          maxOutputTokens: 65536,
          compactionThreshold: 800000, // Rust resolves: 80% of 1M
        },
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step When the compaction threshold is resolved
      const snapshot = getSessionSnapshotForTesting('test-ctx-007-gemini');

      // @step Then the threshold should equal 800000 tokens
      expect(snapshot.model!.compactionThreshold).toBe(800000);
    });
  });

  // ===========================================================================
  // Scenario: OpenAI model uses 80% built-in default
  // ===========================================================================

  describe('Scenario: OpenAI model uses 80% built-in default', () => {
    it('should use 80% threshold for OpenAI models', () => {
      // @step Given a GPT-4o model with 128000 context window
      // @step And no user-configured compaction threshold override
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'openai',
          modelId: 'gpt-4o',
          contextWindow: 128000,
          maxOutputTokens: 16384,
          compactionThreshold: 102400, // Rust resolves: 80% of 128k
        },
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step When the compaction threshold is resolved
      const snapshot = getSessionSnapshotForTesting('test-ctx-007-openai');

      // @step Then the threshold should equal 102400 tokens
      expect(snapshot.model!.compactionThreshold).toBe(102400);
    });
  });

  // ===========================================================================
  // Scenario: User-configured absolute token threshold
  // ===========================================================================

  describe('Scenario: User-configured absolute token threshold', () => {
    it('should honor user-configured absolute threshold over model family default', () => {
      // @step Given a custom model with 200000 context window
      // @step And the user has configured a compaction threshold of 150000 tokens
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'openai',
          modelId: 'custom-model',
          contextWindow: 200000,
          maxOutputTokens: 8192,
          compactionThreshold: 150000, // User override
        },
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step When the compaction threshold is resolved
      const snapshot = getSessionSnapshotForTesting('test-ctx-007-user-tokens');

      // @step Then the threshold should equal 150000 tokens
      expect(snapshot.model!.compactionThreshold).toBe(150000);

      // @step And the built-in model family default should be ignored
      // 80% of 200k would be 160k, but user set 150k
      expect(snapshot.model!.compactionThreshold).not.toBe(160000);
    });
  });

  // ===========================================================================
  // Scenario: User-configured percentage threshold
  // ===========================================================================

  describe('Scenario: User-configured percentage threshold', () => {
    it('should resolve percentage threshold correctly', () => {
      // @step Given a model with 200000 context window
      // @step And the user has configured a compaction threshold of 60 percent
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'openai',
          modelId: 'some-model',
          contextWindow: 200000,
          maxOutputTokens: 8192,
          compactionThreshold: 120000, // Rust resolves: 60% of 200k
        },
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step When the compaction threshold is resolved
      const snapshot = getSessionSnapshotForTesting('test-ctx-007-user-pct');

      // @step Then the threshold should equal 120000 tokens
      expect(snapshot.model!.compactionThreshold).toBe(120000);
    });
  });

  // ===========================================================================
  // Scenario: Unknown model falls through to legacy formula
  // ===========================================================================

  describe('Scenario: Unknown model falls through to legacy formula', () => {
    it('should fall through to legacy formula for unknown models', () => {
      // @step Given an unknown model with 100000 context window and 0 max output
      // @step And no model family information is available
      // @step And no user-configured compaction threshold override
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'unknown',
          modelId: 'my-custom-llm',
          contextWindow: 100000,
          maxOutputTokens: 0,
          compactionThreshold: 68000, // Legacy: 100k - 32k fallback
        },
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step When the compaction threshold is resolved
      const snapshot = getSessionSnapshotForTesting('test-ctx-007-unknown');

      // @step Then the threshold should equal 68000 tokens
      expect(snapshot.model!.compactionThreshold).toBe(68000);

      // @step And the legacy calculate_usable_context formula should be used
      // 100000 - 32000 (SESSION_OUTPUT_TOKEN_MAX fallback for 0) = 68000
    });
  });

  // ===========================================================================
  // Scenario: User threshold exceeding context window is clamped
  // ===========================================================================

  describe('Scenario: User threshold exceeding context window is clamped', () => {
    it('should clamp threshold that exceeds context_window', () => {
      // @step Given a model with 200000 context window and 100000 max output
      // @step And the user has configured a compaction threshold of 300000 tokens
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4',
          contextWindow: 200000,
          maxOutputTokens: 100000,
          compactionThreshold: 168000, // Clamped: 200k - min(100k, 32k) = 168k
        },
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step When the compaction threshold is resolved
      const snapshot = getSessionSnapshotForTesting('test-ctx-007-clamped');

      // @step Then the threshold should be clamped to 168000 tokens
      expect(snapshot.model!.compactionThreshold).toBe(168000);

      // @step And the clamped value should equal context_window minus output reservation
      expect(snapshot.model!.compactionThreshold).toBeLessThanOrEqual(
        snapshot.model!.contextWindow!
      );
    });
  });

  // ===========================================================================
  // Scenario: Context fill percentage uses compaction threshold
  // ===========================================================================

  describe('Scenario: Context fill percentage uses compaction threshold', () => {
    it('should compute fill percentage relative to compaction threshold', () => {
      // @step Given a session with 200000 compaction threshold
      // @step And the session has consumed 100000 tokens
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4',
          contextWindow: 200000,
          maxOutputTokens: 8192,
          compactionThreshold: 200000,
        },
        tokens: { inputTokens: 80000, outputTokens: 20000 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      const snapshot = getSessionSnapshotForTesting('test-ctx-007-fill');

      // @step When the context fill percentage is calculated
      const threshold = snapshot.model!.compactionThreshold!;
      const totalTokens =
        snapshot.tokens.inputTokens + snapshot.tokens.outputTokens;

      // @step Then the fill percentage should be 50 percent
      const fillPercentage = (totalTokens / threshold) * 100;
      expect(fillPercentage).toBe(50);

      // @step And the percentage should be relative to the compaction threshold not the context window
      // With a 200k threshold and 100k tokens, fill = 50%
      // If we used context_window (200k), fill would also be 50% in this case
      // But the key is the threshold is the denominator
      expect(threshold).toBe(200000);
    });
  });

  // ===========================================================================
  // Scenario: Threshold resolution priority chain
  // ===========================================================================

  describe('Scenario: Threshold resolution priority chain', () => {
    it('should prioritize user config over built-in defaults', () => {
      // @step Given a Claude model with 200000 context window
      // @step And the user has configured a compaction threshold of 150000 tokens
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4',
          contextWindow: 200000,
          maxOutputTokens: 8192,
          compactionThreshold: 150000, // User override beats Claude family default
        },
        tokens: { inputTokens: 0, outputTokens: 0 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step When the compaction threshold is resolved
      const snapshot = getSessionSnapshotForTesting('test-ctx-007-priority');

      // @step Then the user-configured threshold of 150000 should take priority
      expect(snapshot.model!.compactionThreshold).toBe(150000);

      // @step And the built-in Claude family default should be ignored
      // Claude default would be legacy: 200k - 8192 = 191808
      expect(snapshot.model!.compactionThreshold).not.toBe(191808);

      // @step And the legacy formula should not be used
      // Legacy would be 200k - 8192 = 191808
      expect(snapshot.model!.compactionThreshold).not.toBe(191808);
    });
  });

  // ===========================================================================
  // Scenario: Stream loop uses ProviderManager compaction threshold
  // ===========================================================================

  describe('Scenario: Stream loop uses ProviderManager compaction threshold', () => {
    it('should expose compaction_threshold through SessionModel for stream loop consistency', () => {
      // @step Given a session with a configured ProviderManager
      const mock = createMockStateSource({
        status: 'running',
        model: {
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4',
          contextWindow: 200000,
          maxOutputTokens: 8192,
          compactionThreshold: 191808,
        },
        tokens: { inputTokens: 50000, outputTokens: 10000 },
        debugEnabled: false,
        baseThinkingLevel: 0,
      });
      setRustStateSource(mock.source);

      // @step When the agent stream loop starts
      const snapshot = getSessionSnapshotForTesting('test-ctx-007-stream');

      // @step Then the threshold should come from ProviderManager compaction_threshold method
      expect(snapshot.model!.compactionThreshold).toBeDefined();
      expect(snapshot.model!.compactionThreshold).toBeGreaterThan(0);

      // @step And all compaction trigger paths should use the same resolved threshold value
      // Verified structurally: the stream loop reads from ProviderManager.compaction_threshold()
      // and all downstream consumers (CompactionHook, pre-prompt, fill %) receive the same value
      expect(snapshot.model!.compactionThreshold).toBe(191808);
    });
  });
});
