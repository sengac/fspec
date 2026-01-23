/**
 * Token State Utilities Tests
 *
 * Tests for DRY/SOLID token state extraction and context fill calculation
 */

import { describe, it, expect, vi, beforeEach, type Mock } from 'vitest';

// Mock dependencies before importing
vi.mock('../../../utils/logger', () => ({
  logger: {
    error: vi.fn(),
  },
}));

vi.mock('@sengac/codelet-napi', () => ({
  sessionGetTokens: vi.fn(),
  persistenceSetSessionTokens: vi.fn(),
}));

import {
  extractTokenStateFromChunks,
  calculateContextFillPercentage,
  persistTokenState,
} from '../tokenStateUtils';
import { logger } from '../../../utils/logger';
import {
  sessionGetTokens,
  persistenceSetSessionTokens,
} from '@sengac/codelet-napi';

describe('tokenStateUtils', () => {
  describe('extractTokenStateFromChunks', () => {
    it('should return null values when no token/context chunks exist', () => {
      const chunks = [{ type: 'Text', text: 'Hello' }, { type: 'Done' }];

      const result = extractTokenStateFromChunks(chunks);

      expect(result.tokenUsage).toBeNull();
      expect(result.contextFillPercentage).toBeNull();
      expect(result.tokensPerSecond).toBeNull();
    });

    it('should extract the LAST TokenUpdate chunk', () => {
      const chunks = [
        { type: 'TokenUpdate', tokens: { inputTokens: 100, outputTokens: 50 } },
        { type: 'Text', text: 'Hello' },
        {
          type: 'TokenUpdate',
          tokens: { inputTokens: 200, outputTokens: 100 },
        },
        { type: 'Done' },
      ];

      const result = extractTokenStateFromChunks(chunks);

      expect(result.tokenUsage).toEqual({
        inputTokens: 200,
        outputTokens: 100,
      });
    });

    it('should extract the LAST ContextFillUpdate chunk', () => {
      const chunks = [
        { type: 'ContextFillUpdate', contextFill: { fillPercentage: 25 } },
        { type: 'Text', text: 'Hello' },
        { type: 'ContextFillUpdate', contextFill: { fillPercentage: 45 } },
        { type: 'Done' },
      ];

      const result = extractTokenStateFromChunks(chunks);

      expect(result.contextFillPercentage).toBe(45);
    });

    it('should extract tokensPerSecond when present', () => {
      const chunks = [
        {
          type: 'TokenUpdate',
          tokens: { inputTokens: 100, outputTokens: 50, tokensPerSecond: 42.5 },
        },
      ];

      const result = extractTokenStateFromChunks(chunks);

      expect(result.tokensPerSecond).toBe(42.5);
    });

    it('should return null tokensPerSecond when not present', () => {
      const chunks = [
        { type: 'TokenUpdate', tokens: { inputTokens: 100, outputTokens: 50 } },
      ];

      const result = extractTokenStateFromChunks(chunks);

      expect(result.tokensPerSecond).toBeNull();
    });

    it('should handle mixed chunk types', () => {
      const chunks = [
        { type: 'Text', text: 'Start' },
        { type: 'TokenUpdate', tokens: { inputTokens: 100, outputTokens: 50 } },
        { type: 'ContextFillUpdate', contextFill: { fillPercentage: 30 } },
        { type: 'ToolCall', toolCall: { id: '1', name: 'test' } },
        {
          type: 'TokenUpdate',
          tokens: { inputTokens: 200, outputTokens: 100, tokensPerSecond: 35 },
        },
        { type: 'Text', text: 'End' },
        { type: 'ContextFillUpdate', contextFill: { fillPercentage: 55 } },
      ];

      const result = extractTokenStateFromChunks(chunks);

      expect(result.tokenUsage).toEqual({
        inputTokens: 200,
        outputTokens: 100,
        tokensPerSecond: 35,
      });
      expect(result.contextFillPercentage).toBe(55);
      expect(result.tokensPerSecond).toBe(35);
    });

    it('should handle empty chunks array', () => {
      const result = extractTokenStateFromChunks([]);

      expect(result.tokenUsage).toBeNull();
      expect(result.contextFillPercentage).toBeNull();
      expect(result.tokensPerSecond).toBeNull();
    });
  });

  describe('calculateContextFillPercentage', () => {
    it('should calculate fill percentage correctly for Claude Sonnet 4', () => {
      // Claude Sonnet 4: 200k context, 16k max output
      // threshold = 200000 - min(16000, 32000) = 200000 - 16000 = 184000
      const result = calculateContextFillPercentage(92000, 200000, 16000);

      // 92000 / 184000 * 100 = 50%
      expect(result).toBe(50);
    });

    it('should cap max output reservation at 32000', () => {
      // Model with 64k max output should still only reserve 32k
      // threshold = 200000 - min(64000, 32000) = 200000 - 32000 = 168000
      const result = calculateContextFillPercentage(84000, 200000, 64000);

      // 84000 / 168000 * 100 = 50%
      expect(result).toBe(50);
    });

    it('should return 0 when threshold is 0 or negative', () => {
      // Edge case: very small context window
      const result = calculateContextFillPercentage(10000, 30000, 32000);

      // threshold = 30000 - 32000 = -2000 (negative)
      expect(result).toBe(0);
    });

    it('should handle 0 input tokens', () => {
      const result = calculateContextFillPercentage(0, 200000, 16000);

      expect(result).toBe(0);
    });

    it('should allow percentage over 100 near compaction', () => {
      // When context is very full, percentage can exceed 100
      // threshold = 200000 - 16000 = 184000
      const result = calculateContextFillPercentage(200000, 200000, 16000);

      // 200000 / 184000 * 100 ≈ 109%
      expect(result).toBe(109);
    });

    it('should round to nearest integer', () => {
      // Test rounding behavior
      // threshold = 200000 - 16000 = 184000
      // 46000 / 184000 * 100 = 25.0%
      expect(calculateContextFillPercentage(46000, 200000, 16000)).toBe(25);

      // 46500 / 184000 * 100 ≈ 25.27% → rounds to 25
      expect(calculateContextFillPercentage(46500, 200000, 16000)).toBe(25);

      // 47000 / 184000 * 100 ≈ 25.54% → rounds to 26
      expect(calculateContextFillPercentage(47000, 200000, 16000)).toBe(26);
    });

    it('should match Rust formula exactly', () => {
      // Verify against known Rust calculation
      // From codelet/cli/tests/autocompact_buffer_test.rs
      // Claude: 200000 context, 16000 max_output
      // Usable = 200000 - 16000 = 184000
      const usable = 200000 - Math.min(16000, 32000);
      expect(usable).toBe(184000);

      // At 50% fill: 92000 tokens
      const fillAt50 = calculateContextFillPercentage(92000, 200000, 16000);
      expect(fillAt50).toBe(50);

      // At 100% fill: 184000 tokens
      const fillAt100 = calculateContextFillPercentage(184000, 200000, 16000);
      expect(fillAt100).toBe(100);
    });
  });

  describe('persistTokenState', () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it('should do nothing when sessionId is null', () => {
      persistTokenState(null);

      expect(sessionGetTokens).not.toHaveBeenCalled();
      expect(persistenceSetSessionTokens).not.toHaveBeenCalled();
    });

    it('should persist token state for valid session', () => {
      (sessionGetTokens as Mock).mockReturnValue({
        inputTokens: 5000,
        outputTokens: 1000,
      });

      persistTokenState('session-123');

      expect(sessionGetTokens).toHaveBeenCalledWith('session-123');
      expect(persistenceSetSessionTokens).toHaveBeenCalledWith(
        'session-123',
        5000, // inputTokens
        1000, // outputTokens
        0, // cacheRead
        0, // cacheCreate
        5000, // cumulativeInput
        1000 // cumulativeOutput
      );
    });

    it('should log error when persistence fails', () => {
      const testError = new Error('Persistence failed');
      (sessionGetTokens as Mock).mockImplementation(() => {
        throw testError;
      });

      persistTokenState('session-456');

      expect(logger.error).toHaveBeenCalledWith(
        'Failed to persist token state for session session-456:',
        testError
      );
    });

    it('should not throw when persistence fails', () => {
      (sessionGetTokens as Mock).mockImplementation(() => {
        throw new Error('Test error');
      });

      // Should not throw
      expect(() => persistTokenState('session-789')).not.toThrow();
    });
  });
});
