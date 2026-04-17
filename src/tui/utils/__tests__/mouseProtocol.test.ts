/**
 * Feature: spec/features/sgr-mouse-protocol.feature
 *
 * Tests for SGR mouse protocol migration (BUG-131).
 * Validates that the shared mouseProtocol utility correctly parses SGR mouse events
 * and that escape sequences use the SGR extended protocol format.
 */

import { describe, it, expect } from 'vitest';

// NOTE: These imports will fail until the module is created in implementing phase.
// This is intentional — red phase (tests must fail before implementation).
import {
  MOUSE_ENABLE,
  MOUSE_DISABLE,
  SGR_BUTTON,
  parseSgrMouse,
} from '../mouseProtocol';

describe('Feature: SGR mouse protocol for ink 6.8.0 compatibility', () => {
  // ========================================
  // PARSER TESTS
  // ========================================

  describe('Scenario: Parse SGR scroll-up mouse event', () => {
    it('should parse scroll-up event with correct button, coordinates, and press indicator', () => {
      // @step Given the SGR mouse parser receives input "[<64;10;20M"
      const input = '[<64;10;20M';

      // @step When the parser processes the input
      const result = parseSgrMouse(input);

      // @step Then it should return button code 64
      expect(result).not.toBeNull();
      expect(result!.button).toBe(64);

      // @step And it should return x coordinate 10
      expect(result!.x).toBe(10);

      // @step And it should return y coordinate 20
      expect(result!.y).toBe(20);

      // @step And it should indicate a press event
      expect(result!.isRelease).toBe(false);
    });
  });

  describe('Scenario: Parse SGR scroll-down mouse event', () => {
    it('should parse scroll-down event with correct button, coordinates, and press indicator', () => {
      // @step Given the SGR mouse parser receives input "[<65;5;15M"
      const input = '[<65;5;15M';

      // @step When the parser processes the input
      const result = parseSgrMouse(input);

      // @step Then it should return button code 65
      expect(result).not.toBeNull();
      expect(result!.button).toBe(65);

      // @step And it should return x coordinate 5
      expect(result!.x).toBe(5);

      // @step And it should return y coordinate 15
      expect(result!.y).toBe(15);

      // @step And it should indicate a press event
      expect(result!.isRelease).toBe(false);
    });
  });

  describe('Scenario: Parse SGR left-click press event', () => {
    it('should parse left-click press with correct button and press indicator', () => {
      // @step Given the SGR mouse parser receives input "[<0;5;10M"
      const input = '[<0;5;10M';

      // @step When the parser processes the input
      const result = parseSgrMouse(input);

      // @step Then it should return button code 0
      expect(result).not.toBeNull();
      expect(result!.button).toBe(0);

      // @step And it should indicate a press event
      expect(result!.isRelease).toBe(false);
    });
  });

  describe('Scenario: Parse SGR left-click release event', () => {
    it('should parse left-click release with correct button and release indicator', () => {
      // @step Given the SGR mouse parser receives input "[<0;5;10m"
      const input = '[<0;5;10m';

      // @step When the parser processes the input
      const result = parseSgrMouse(input);

      // @step Then it should return button code 0
      expect(result).not.toBeNull();
      expect(result!.button).toBe(0);

      // @step And it should indicate a release event
      expect(result!.isRelease).toBe(true);
    });
  });

  describe('Scenario: Reject non-mouse input', () => {
    it('should return null for non-mouse input', () => {
      // @step Given the SGR mouse parser receives input "j"
      const input = 'j';

      // @step When the parser processes the input
      const result = parseSgrMouse(input);

      // @step Then it should return null
      expect(result).toBeNull();
    });

    it('should return null for old X10 mouse format', () => {
      // The old format should not match SGR regex
      const input =
        '[M' +
        String.fromCharCode(96) +
        String.fromCharCode(33) +
        String.fromCharCode(33);
      const result = parseSgrMouse(input);
      expect(result).toBeNull();
    });

    it('should return null for empty string', () => {
      const result = parseSgrMouse('');
      expect(result).toBeNull();
    });

    it('should return null for partial SGR sequence', () => {
      const result = parseSgrMouse('[<64;10');
      expect(result).toBeNull();
    });
  });

  // ========================================
  // CONSTANTS TESTS
  // ========================================

  describe('Scenario: Mouse enable sequence uses SGR protocol', () => {
    it('should contain both X10 tracking and SGR encoding enable codes', () => {
      // @step When a component enables mouse tracking
      // @step Then the output should contain the escape sequence "\x1b[?1000h\x1b[?1006h"
      expect(MOUSE_ENABLE).toBe('\x1b[?1000h\x1b[?1006h');
    });
  });

  describe('Scenario: Mouse disable sequence uses reverse order', () => {
    it('should contain SGR disable before X10 tracking disable', () => {
      // @step When a component disables mouse tracking
      // @step Then the output should contain the escape sequence "\x1b[?1006l\x1b[?1000l"
      expect(MOUSE_DISABLE).toBe('\x1b[?1006l\x1b[?1000l');
    });
  });

  // ========================================
  // SGR BUTTON CODE TESTS
  // ========================================

  describe('SGR button codes', () => {
    it('should define correct button codes', () => {
      expect(SGR_BUTTON.LEFT).toBe(0);
      expect(SGR_BUTTON.MIDDLE).toBe(1);
      expect(SGR_BUTTON.RIGHT).toBe(2);
      expect(SGR_BUTTON.SCROLL_UP).toBe(64);
      expect(SGR_BUTTON.SCROLL_DOWN).toBe(65);
    });
  });

  // ========================================
  // EDGE CASES
  // ========================================

  describe('Edge cases', () => {
    it('should parse middle-click press', () => {
      const result = parseSgrMouse('[<1;20;30M');
      expect(result).not.toBeNull();
      expect(result!.button).toBe(1);
      expect(result!.x).toBe(20);
      expect(result!.y).toBe(30);
      expect(result!.isRelease).toBe(false);
    });

    it('should parse right-click release', () => {
      const result = parseSgrMouse('[<2;1;1m');
      expect(result).not.toBeNull();
      expect(result!.button).toBe(2);
      expect(result!.isRelease).toBe(true);
    });

    it('should parse large coordinate values', () => {
      const result = parseSgrMouse('[<0;999;500M');
      expect(result).not.toBeNull();
      expect(result!.x).toBe(999);
      expect(result!.y).toBe(500);
    });

    it('should parse coordinate (1,1) correctly', () => {
      const result = parseSgrMouse('[<64;1;1M');
      expect(result).not.toBeNull();
      expect(result!.button).toBe(64);
      expect(result!.x).toBe(1);
      expect(result!.y).toBe(1);
    });
  });
});
