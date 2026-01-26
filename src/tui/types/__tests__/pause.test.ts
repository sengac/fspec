/**
 * PAUSE-001: Pause Type Tests
 *
 * Tests for the pause type utilities in src/tui/types/pause.ts
 */

import { describe, it, expect } from 'vitest';
import {
  isValidPauseKind,
  parsePauseInfo,
  pauseInfoEqual,
  type PauseInfo,
  type PauseKind,
} from '../pause';

describe('PAUSE-001: Pause Types', () => {
  describe('isValidPauseKind', () => {
    it('should return true for "continue"', () => {
      expect(isValidPauseKind('continue')).toBe(true);
    });

    it('should return true for "confirm"', () => {
      expect(isValidPauseKind('confirm')).toBe(true);
    });

    it('should return false for invalid strings', () => {
      expect(isValidPauseKind('invalid')).toBe(false);
      expect(isValidPauseKind('')).toBe(false);
      expect(isValidPauseKind('CONTINUE')).toBe(false);
      expect(isValidPauseKind('CONFIRM')).toBe(false);
    });
  });

  describe('parsePauseInfo', () => {
    it('should return null for null input', () => {
      expect(parsePauseInfo(null)).toBeNull();
    });

    it('should return null for undefined input', () => {
      expect(parsePauseInfo(undefined)).toBeNull();
    });

    it('should return null for invalid kind', () => {
      expect(
        parsePauseInfo({
          kind: 'invalid',
          toolName: 'WebSearch',
          message: 'Page loaded',
        })
      ).toBeNull();
    });

    it('should parse valid continue pause', () => {
      const result = parsePauseInfo({
        kind: 'continue',
        toolName: 'WebSearch',
        message: 'Page loaded at https://example.com',
        details: null,
      });

      expect(result).not.toBeNull();
      expect(result?.kind).toBe('continue');
      expect(result?.toolName).toBe('WebSearch');
      expect(result?.message).toBe('Page loaded at https://example.com');
      expect(result?.details).toBeUndefined();
    });

    it('should parse valid confirm pause with details', () => {
      const result = parsePauseInfo({
        kind: 'confirm',
        toolName: 'Bash',
        message: 'Confirm dangerous command',
        details: 'rm -rf /tmp/*',
      });

      expect(result).not.toBeNull();
      expect(result?.kind).toBe('confirm');
      expect(result?.toolName).toBe('Bash');
      expect(result?.message).toBe('Confirm dangerous command');
      expect(result?.details).toBe('rm -rf /tmp/*');
    });

    it('should convert null details to undefined', () => {
      const result = parsePauseInfo({
        kind: 'continue',
        toolName: 'Test',
        message: 'Test message',
        details: null,
      });

      expect(result?.details).toBeUndefined();
    });
  });

  describe('pauseInfoEqual', () => {
    const pauseA: PauseInfo = {
      kind: 'continue',
      toolName: 'WebSearch',
      message: 'Page loaded',
      details: undefined,
    };

    const pauseB: PauseInfo = {
      kind: 'continue',
      toolName: 'WebSearch',
      message: 'Page loaded',
      details: undefined,
    };

    const pauseC: PauseInfo = {
      kind: 'confirm',
      toolName: 'Bash',
      message: 'Confirm command',
      details: 'rm -rf',
    };

    it('should return true for both null', () => {
      expect(pauseInfoEqual(null, null)).toBe(true);
    });

    it('should return false for null vs non-null', () => {
      expect(pauseInfoEqual(null, pauseA)).toBe(false);
      expect(pauseInfoEqual(pauseA, null)).toBe(false);
    });

    it('should return true for identical objects', () => {
      expect(pauseInfoEqual(pauseA, pauseA)).toBe(true);
    });

    it('should return true for equal objects', () => {
      expect(pauseInfoEqual(pauseA, pauseB)).toBe(true);
    });

    it('should return false for different kinds', () => {
      expect(pauseInfoEqual(pauseA, pauseC)).toBe(false);
    });

    it('should return false for different tool names', () => {
      const different: PauseInfo = { ...pauseA, toolName: 'Different' };
      expect(pauseInfoEqual(pauseA, different)).toBe(false);
    });

    it('should return false for different messages', () => {
      const different: PauseInfo = { ...pauseA, message: 'Different' };
      expect(pauseInfoEqual(pauseA, different)).toBe(false);
    });

    it('should return false for different details', () => {
      const withDetails: PauseInfo = { ...pauseA, details: 'some details' };
      expect(pauseInfoEqual(pauseA, withDetails)).toBe(false);
    });

    it('should return true for both undefined details', () => {
      const a: PauseInfo = { ...pauseA, details: undefined };
      const b: PauseInfo = { ...pauseB, details: undefined };
      expect(pauseInfoEqual(a, b)).toBe(true);
    });
  });
});
