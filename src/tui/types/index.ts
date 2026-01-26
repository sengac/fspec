/**
 * TUI Type exports
 *
 * Central export point for all TUI-related types.
 * Provides clean imports for consumers.
 */

export type { PauseKind, PauseInfo } from './pause';
export { isValidPauseKind, parsePauseInfo, pauseInfoEqual } from './pause';
