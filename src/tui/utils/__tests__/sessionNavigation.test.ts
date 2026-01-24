/**
 * Tests for sessionNavigation.ts - Simple Rust wrapper functions
 *
 * VIEWNV-001: Unified Shift+Arrow Navigation
 *
 * These functions just wrap Rust NAPI calls. Rust is the source of truth.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock NAPI functions
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetNext: vi.fn(),
  sessionGetPrev: vi.fn(),
  sessionGetFirst: vi.fn(),
  sessionClearActive: vi.fn(),
}));

import {
  sessionGetNext,
  sessionGetPrev,
  sessionGetFirst,
  sessionClearActive,
} from '@sengac/codelet-napi';

import {
  navigateRight,
  navigateLeft,
  clearActiveSession,
  getFirstSession,
} from '../sessionNavigation';

describe('sessionNavigation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('navigateRight', () => {
    it('returns session when Rust returns next session', () => {
      vi.mocked(sessionGetNext).mockReturnValue('session-2');

      const result = navigateRight();

      expect(result).toEqual({ type: 'session', sessionId: 'session-2' });
      expect(sessionGetNext).toHaveBeenCalled();
    });

    it('returns create-dialog when Rust returns null (no next session)', () => {
      vi.mocked(sessionGetNext).mockReturnValue(null);

      const result = navigateRight();

      expect(result).toEqual({ type: 'create-dialog' });
    });
  });

  describe('navigateLeft', () => {
    it('returns session when Rust returns previous session', () => {
      vi.mocked(sessionGetPrev).mockReturnValue('session-1');

      const result = navigateLeft();

      expect(result).toEqual({ type: 'session', sessionId: 'session-1' });
      expect(sessionGetPrev).toHaveBeenCalled();
    });

    it('returns board when Rust returns null (at first session or no session)', () => {
      vi.mocked(sessionGetPrev).mockReturnValue(null);

      const result = navigateLeft();

      expect(result).toEqual({ type: 'board' });
    });
  });

  describe('clearActiveSession', () => {
    it('calls Rust sessionClearActive', () => {
      clearActiveSession();

      expect(sessionClearActive).toHaveBeenCalled();
    });
  });

  describe('getFirstSession', () => {
    it('returns session ID from Rust', () => {
      vi.mocked(sessionGetFirst).mockReturnValue('session-1');

      const result = getFirstSession();

      expect(result).toBe('session-1');
      expect(sessionGetFirst).toHaveBeenCalled();
    });

    it('returns null when no sessions exist', () => {
      vi.mocked(sessionGetFirst).mockReturnValue(null);

      const result = getFirstSession();

      expect(result).toBeNull();
    });
  });
});
