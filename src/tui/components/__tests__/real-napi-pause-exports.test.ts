/**
 * PAUSE-001: Real NAPI Function Export Tests
 *
 * These tests verify the ACTUAL NAPI functions are exported from @sengac/codelet-napi.
 * Tests will FAIL until implementation is complete.
 *
 * Required exports:
 * 1. sessionGetPauseState(sessionId: string): PauseState | null
 * 2. sessionPauseResume(sessionId: string): void
 * 3. sessionPauseConfirm(sessionId: string, approved: boolean): void
 */

import { describe, it, expect } from 'vitest';

// Import the REAL module - these imports will fail if functions don't exist
import * as codeletNapi from '@sengac/codelet-napi';

describe('PAUSE-001: Real NAPI Function Exports', () => {
  // =============================================================================
  // sessionGetPauseState Tests
  // =============================================================================

  describe('sessionGetPauseState', () => {
    it('should be exported from @sengac/codelet-napi', () => {
      // This test FAILS if sessionGetPauseState is not exported
      expect(typeof codeletNapi.sessionGetPauseState).toBe('function');
    });

    it('should accept sessionId parameter', () => {
      // Verify function signature by calling it with valid UUID format
      // Should throw with "Session not found" (not invalid format)
      expect(() => {
        codeletNapi.sessionGetPauseState(
          '00000000-0000-0000-0000-000000000000'
        );
      }).toThrow(/session not found|Session not found/i);
    });

    it('should reject invalid session ID format', () => {
      // Invalid UUID should throw "Invalid session ID"
      expect(() => {
        codeletNapi.sessionGetPauseState('invalid-session-id');
      }).toThrow(/invalid/i);
    });
  });

  // =============================================================================
  // sessionPauseResume Tests
  // =============================================================================

  describe('sessionPauseResume', () => {
    it('should be exported from @sengac/codelet-napi', () => {
      // This test FAILS if sessionPauseResume is not exported
      expect(typeof codeletNapi.sessionPauseResume).toBe('function');
    });

    it('should accept sessionId parameter', () => {
      // Verify function signature by calling it with valid UUID format
      // Should throw with "Session not found" (not invalid format)
      expect(() => {
        codeletNapi.sessionPauseResume('00000000-0000-0000-0000-000000000000');
      }).toThrow(/session not found|Session not found/i);
    });
  });

  // =============================================================================
  // sessionPauseConfirm Tests
  // =============================================================================

  describe('sessionPauseConfirm', () => {
    it('should be exported from @sengac/codelet-napi', () => {
      // This test FAILS if sessionPauseConfirm is not exported
      expect(typeof codeletNapi.sessionPauseConfirm).toBe('function');
    });

    it('should accept sessionId and approved boolean parameters', () => {
      // Verify function signature by calling it with valid UUID format
      expect(() => {
        codeletNapi.sessionPauseConfirm(
          '00000000-0000-0000-0000-000000000000',
          true
        );
      }).toThrow(/session not found|Session not found/i);
    });
  });

  // =============================================================================
  // PauseState Type Tests
  // =============================================================================

  describe('PauseState type structure', () => {
    it('sessionGetPauseState should return null or object with correct shape', () => {
      // This documents the expected shape - will be verified when we have a real paused session
      // For now, we verify the function throws for non-existent session (proper UUID)
      expect(() => {
        codeletNapi.sessionGetPauseState(
          '00000000-0000-0000-0000-000000000000'
        );
      }).toThrow(/session not found|Session not found/i);
    });
  });
});
