/**
 * Animation Constants
 *
 * Centralized timing constants for all TUI animations.
 * Single source of truth for animation speeds - ensures consistency
 * and makes it easy to tune animation feel across the entire app.
 *
 * IMPORTANT: Animation intervals are derived from INK_FRAME_TIME_MS
 * to ensure synchronization with Ink's render throttle. This prevents
 * inconsistent animation speeds caused by timer batching.
 *
 * @module animationConstants
 */

import { INK_MAX_FPS, INK_FRAME_TIME_MS } from '../config/inkConfig';

// Re-export for convenience
export { INK_MAX_FPS, INK_FRAME_TIME_MS };

/**
 * Number of characters to animate per frame.
 * Higher = faster animation, lower = smoother but slower.
 *
 * At 60fps with 5 chars/frame:
 * - ~300 characters per second
 * - "Thinking..." (40 chars) hides in ~0.13s
 * - Placeholder (95 chars) reveals in ~0.32s
 * - Total: ~0.5s (snappy)
 */
export const CHARS_PER_FRAME = 5;

/**
 * Character-by-character animation speed (milliseconds per character).
 * Used for text reveal/hide animations like input transitions.
 *
 * Derived from frame time to ensure each update is visible.
 * Animation moves CHARS_PER_FRAME characters each frame.
 */
export const CHAR_ANIMATION_INTERVAL_MS = INK_FRAME_TIME_MS;

/**
 * Delay between animation phases (milliseconds).
 * Used to create a brief pause between hide and show phases
 * for a cleaner visual separation.
 *
 * Set to ~2 frames for a quick pause.
 */
export const ANIMATION_PHASE_DELAY_MS = INK_FRAME_TIME_MS * 2;

/**
 * Animation timing configuration object.
 * Provides a structured way to access all timing constants.
 */
export const ANIMATION_TIMING = {
  /** Target FPS from Ink config */
  targetFps: INK_MAX_FPS,
  /** Frame time in ms */
  frameTime: INK_FRAME_TIME_MS,
  /** Characters per frame */
  charsPerFrame: CHARS_PER_FRAME,
  /** Character animation interval in ms */
  charInterval: CHAR_ANIMATION_INTERVAL_MS,
  /** Phase transition delay in ms */
  phaseDelay: ANIMATION_PHASE_DELAY_MS,
} as const;

export type AnimationTiming = typeof ANIMATION_TIMING;
