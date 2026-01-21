/**
 * Ink Renderer Configuration
 *
 * Single source of truth for Ink render options.
 * This ensures animation timing stays synchronized with the render loop.
 *
 * @module inkConfig
 */

/**
 * Target frames per second for the Ink renderer.
 *
 * This value is used by:
 * - The main render() call in src/index.ts
 * - Animation timing calculations in animationConstants.ts
 *
 * Changing this value automatically adjusts animation speeds
 * to stay synchronized with the render throttle.
 *
 * Recommended values:
 * - 60: Smooth animations, higher CPU usage
 * - 30: Lower CPU usage, still acceptable for most UIs
 */
export const INK_MAX_FPS = 60;

/**
 * Frame time in milliseconds based on INK_MAX_FPS.
 * This is the minimum interval between render updates.
 */
export const INK_FRAME_TIME_MS = Math.ceil(1000 / INK_MAX_FPS);

/**
 * Ink render options.
 * Use this when calling render() to ensure consistent configuration.
 */
export const INK_RENDER_OPTIONS = {
  /** Enable incremental rendering to reduce flickering */
  incrementalRendering: true,
  /** Target FPS for render throttling */
  maxFps: INK_MAX_FPS,
  /** Debug mode - set via environment variable if needed */
  debug: false,
  /** Patch console for cleaner output */
  patchConsole: false,
} as const;

export type InkRenderOptions = typeof INK_RENDER_OPTIONS;
