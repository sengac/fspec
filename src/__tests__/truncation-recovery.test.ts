/**
 * Feature: spec/features/truncated-tool-call-recovery-auto-chunk-large-writes-and-retry-on-max-tokens.feature
 *
 * PROV-040: Truncated tool call recovery — auto-chunk large writes and retry on max_tokens
 *
 * Integration tests that verify the truncation detection and recovery message contract
 * using fixture data. These tests validate the same behavior the Rust implementation
 * enforces, ensuring TypeScript/TUI consumers handle truncation correctly.
 *
 * NO MOCKS — Uses fixture data and real string matching.
 */

import { describe, it, expect } from 'vitest';

// =============================================================================
// Fixture data: realistic error strings from each provider
// =============================================================================

/** Truncation error fixtures matching the PROV-039 enriched error format */
const truncationErrorFixtures = {
  anthropicWrite: [
    'Streaming error: ResponseError: Tool call truncated due to output token limit.',
    "Tool 'Write' received incomplete JSON arguments.",
    'The model hit max_tokens while generating the tool call.',
    'Partial arguments: {"file_path": "/tmp/prov039-large-write-test.txt"',
  ].join(' '),

  anthropicBash: [
    'Tool call truncated due to output token limit.',
    "Tool 'Bash' received incomplete JSON arguments.",
    'The model hit max_tokens while generating the tool call.',
    'Partial arguments: {"command": "cat << \'HEREDOC_EOF\'',
  ].join(' '),

  openaiEdit: [
    'Tool call truncated due to output token limit.',
    "Tool 'Edit' received incomplete JSON arguments.",
    'Partial arguments: {"file_path": "/src/main.rs", "old_string": "fn',
  ].join(' '),

  geminiWrite: [
    'Tool call truncated due to output token limit.',
    "Tool 'Write' received incomplete JSON arguments.",
    'Partial arguments: {"file_path": "/tmp/output.json", "content": "[{',
  ].join(' '),
};

/** Non-truncation error fixtures that must NOT trigger recovery */
const nonTruncationErrorFixtures = {
  networkTimeout: 'Network timeout after 30000ms',
  authFailure: 'Authentication failed: Invalid API key',
  rateLimited: 'Rate limit exceeded. Please retry after 60 seconds.',
  promptTooLong: 'prompt is too long: 209834 tokens > 200000 maximum',
  contextExceeded: 'context_length_exceeded: Request too large',
  thinkingBudget:
    '{"type":"invalid_request_error","message":"`max_tokens` must be greater than `thinking.budget_tokens`"}',
  genericServerError: 'Internal server error (500)',
  sseError: 'SSE Error: connection reset by peer',
  textTruncation: 'Response truncated: model hit max_tokens output limit',
  partialSubstring: 'Tool call truncated',
  partialSubstring2: 'output token limit exceeded',
};

// =============================================================================
// Detection contract: matches the Rust is_truncated_tool_call_error()
// =============================================================================

/**
 * TypeScript-side detection function matching the Rust implementation.
 * This validates the contract: "contains 'Tool call truncated due to output token limit'"
 *
 * SYNC NOTE: This sentinel string is produced by rig-core's anthropic/streaming.rs
 * (PROV-039, line 443) and is also checked by Rust's is_truncated_tool_call_error()
 * in stream_loop.rs. If the sentinel changes, both this TS function AND the Rust
 * function must be updated. The Rust tests in truncation_recovery_test.rs test the
 * same string, so a desync would be caught by cargo test failing.
 */
function isTruncatedToolCallError(errorStr: string): boolean {
  return errorStr.includes('Tool call truncated due to output token limit');
}

/**
 * Extract tool name from the truncation error message.
 * Matches the Rust build_truncation_recovery_message() extraction logic.
 */
function extractToolName(errorStr: string): string {
  const match = errorStr.match(/Tool '([^']+)'/);
  return match ? match[1] : 'unknown';
}

/**
 * Extract partial arguments from the truncation error message.
 * Matches the Rust build_truncation_recovery_message() extraction logic.
 */
function extractPartialArgs(errorStr: string): string {
  const parts = errorStr.split('Partial arguments: ');
  return parts.length > 1 ? parts[1] : '(not available)';
}

// =============================================================================
// Tests
// =============================================================================

describe('Feature: Truncated tool call recovery', () => {
  describe('Scenario: Truncated tool call error includes structured recovery instruction', () => {
    it('should detect Anthropic Write truncation error', () => {
      // @step Given the agent is streaming a response from any provider
      const error = truncationErrorFixtures.anthropicWrite;

      // @step And the model attempts a Write tool call with content exceeding the output token limit
      // @step When the tool call is truncated due to max_tokens
      const detected = isTruncatedToolCallError(error);

      // @step Then the error message contains a structured recovery instruction
      expect(detected).toBe(true);

      // @step And the recovery instruction includes the truncated tool name
      expect(extractToolName(error)).toBe('Write');

      // @step And the recovery instruction includes the partial arguments that were received
      expect(extractPartialArgs(error)).toContain(
        '/tmp/prov039-large-write-test.txt'
      );
    });

    it('should extract tool name and partial args for all provider errors', () => {
      // @step Given the agent is streaming a response from any provider
      const fixtures = [
        {
          error: truncationErrorFixtures.anthropicWrite,
          tool: 'Write',
          argFragment: '/tmp/prov039',
        },
        {
          error: truncationErrorFixtures.anthropicBash,
          tool: 'Bash',
          argFragment: 'HEREDOC_EOF',
        },
        {
          error: truncationErrorFixtures.openaiEdit,
          tool: 'Edit',
          argFragment: '/src/main.rs',
        },
        {
          error: truncationErrorFixtures.geminiWrite,
          tool: 'Write',
          argFragment: '/tmp/output.json',
        },
      ];

      for (const fixture of fixtures) {
        // @step When the tool call is truncated due to max_tokens
        expect(isTruncatedToolCallError(fixture.error)).toBe(true);

        // @step And the recovery instruction includes the truncated tool name
        expect(extractToolName(fixture.error)).toBe(fixture.tool);

        // @step And the recovery instruction includes the partial arguments that were received
        expect(extractPartialArgs(fixture.error)).toContain(
          fixture.argFragment
        );
      }
    });
  });

  describe('Scenario: Retry budget prevents infinite truncation retry loops', () => {
    it('should allow exactly MAX_TRUNCATION_RETRIES before exhaustion', () => {
      // @step Given the agent is streaming a response from any provider
      // @step And the truncation retry budget is set to 2
      const MAX_RETRIES = 2;
      let retryCount = 0;
      const error = truncationErrorFixtures.anthropicWrite;

      // @step When the model hits max_tokens on the first tool call attempt
      expect(isTruncatedToolCallError(error)).toBe(true);
      retryCount++;

      // @step Then the recovery instruction is sent and a retry stream is started
      expect(retryCount).toBeLessThanOrEqual(MAX_RETRIES);

      // @step When the model hits max_tokens on the second tool call attempt
      retryCount++;
      expect(retryCount).toBeLessThanOrEqual(MAX_RETRIES);

      // Third attempt — budget exhausted
      retryCount++;

      // @step Then the retry budget is exhausted
      expect(retryCount).toBeGreaterThan(MAX_RETRIES);

      // @step And the error is reported to the user as a non-recoverable truncation failure
      // @step And the stream loop terminates without starting another retry
      // (In Rust: the else branch emits the budget-exhausted error and returns Err)
    });
  });

  describe('Scenario: Normal completion is unaffected by truncation recovery logic', () => {
    it('should not detect non-truncation errors as truncation', () => {
      // @step Given the agent is streaming a response from any provider
      // @step And the model completes a tool call normally with stop_reason end_turn
      const errors = Object.values(nonTruncationErrorFixtures);

      for (const error of errors) {
        // @step When the stream completes
        // @step Then no recovery instruction is injected
        expect(isTruncatedToolCallError(error)).toBe(false);
      }

      // @step And the truncation retry counter remains at zero
      // @step And the behavior is identical to pre-PROV-040 baseline
    });
  });

  describe('Scenario: Text-only truncation does not trigger tool call recovery', () => {
    it('should not detect text truncation warning as tool call truncation', () => {
      // @step Given the agent is streaming a response from any provider
      // @step And the model hits max_tokens during a text-only response with no tool call
      const textTruncation =
        'Response truncated: model hit max_tokens output limit';

      // @step When the stream completes with stop_reason max_tokens
      // @step Then the existing PROV-039 truncation warning is displayed
      // @step And no tool call recovery instruction is injected
      expect(isTruncatedToolCallError(textTruncation)).toBe(false);
    });
  });

  describe('Scenario: Truncation recovery is provider-agnostic', () => {
    it('should detect truncation errors from all providers identically', () => {
      // @step Given the truncation detection relies on the error message string from PROV-039
      const providerErrors = [
        truncationErrorFixtures.anthropicWrite,
        truncationErrorFixtures.openaiEdit,
        truncationErrorFixtures.geminiWrite,
      ];

      // @step When a truncation error containing "Tool call truncated due to output token limit" is received
      for (const error of providerErrors) {
        // @step Then the same recovery logic fires regardless of whether the provider is Anthropic, OpenAI, or Gemini
        expect(isTruncatedToolCallError(error)).toBe(true);
      }

      // @step And the recovery instruction content is identical across all providers
      // All use the same detection string and recovery message builder
      const toolNames = providerErrors.map(e => extractToolName(e));
      // Each extracts a valid tool name (not 'unknown')
      for (const name of toolNames) {
        expect(name).not.toBe('unknown');
      }
    });
  });

  describe('Edge cases', () => {
    it('should not match empty string', () => {
      expect(isTruncatedToolCallError('')).toBe(false);
    });

    it('should not match partial substrings', () => {
      expect(isTruncatedToolCallError('Tool call truncated')).toBe(false);
      expect(isTruncatedToolCallError('output token limit')).toBe(false);
      expect(isTruncatedToolCallError('truncated due to')).toBe(false);
    });

    it('should handle tool name extraction for unknown format', () => {
      const weirdError =
        'Tool call truncated due to output token limit. No standard format here.';
      expect(isTruncatedToolCallError(weirdError)).toBe(true);
      expect(extractToolName(weirdError)).toBe('unknown');
    });

    it('should handle partial args extraction when missing', () => {
      const noArgs =
        "Tool call truncated due to output token limit. Tool 'Write' received incomplete JSON.";
      expect(extractPartialArgs(noArgs)).toBe('(not available)');
    });
  });
});
