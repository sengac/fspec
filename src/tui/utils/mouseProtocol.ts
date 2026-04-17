/**
 * SGR Mouse Protocol utilities for ink 6.8.0+ compatibility.
 *
 * Ink 6.8.0 introduced a CSI input parser that splits X10 mouse protocol
 * sequences at the CSI final byte. SGR extended mouse protocol encodes all
 * data as standard CSI parameter bytes (0x30-0x3F) which survive the parser.
 *
 * @module mouseProtocol
 */

/**
 * Enable SGR extended mouse protocol.
 * Combines X10 button event tracking (?1000h) with SGR encoding (?1006h).
 */
export const MOUSE_ENABLE = '\x1b[?1000h\x1b[?1006h';

/**
 * Disable SGR extended mouse protocol.
 * Disables in reverse order: SGR encoding first (?1006l), then tracking (?1000l).
 */
export const MOUSE_DISABLE = '\x1b[?1006l\x1b[?1000l';

/**
 * SGR mouse event regex.
 * Matches the post-ESC-strip format delivered by ink's useInput handler.
 * Full format: ESC [ < button ; x ; y M/m
 * After ESC strip: [ < button ; x ; y M/m
 */
export const SGR_MOUSE_RE = /^\[<(\d+);(\d+);(\d+)([Mm])$/;

/** SGR mouse button codes */
export const SGR_BUTTON = {
  LEFT: 0,
  MIDDLE: 1,
  RIGHT: 2,
  SCROLL_UP: 64,
  SCROLL_DOWN: 65,
} as const;

/** Parsed SGR mouse event */
export interface SgrMouseEvent {
  /** Button code (0=left, 1=middle, 2=right, 64=scroll-up, 65=scroll-down) */
  button: number;
  /** X coordinate (1-based) */
  x: number;
  /** Y coordinate (1-based) */
  y: number;
  /** True if this is a release event (terminator 'm'), false for press ('M') */
  isRelease: boolean;
}

/**
 * Parse an SGR mouse event from input string (after ESC stripping by ink).
 *
 * @param input - The input string from ink's useInput handler (ESC already stripped)
 * @returns Parsed mouse event or null if input is not an SGR mouse sequence
 */
export function parseSgrMouse(input: string): SgrMouseEvent | null {
  const match = SGR_MOUSE_RE.exec(input);
  if (!match) {
    return null;
  }
  return {
    button: parseInt(match[1], 10),
    x: parseInt(match[2], 10),
    y: parseInt(match[3], 10),
    isRelease: match[4] === 'm',
  };
}
