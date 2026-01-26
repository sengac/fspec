/**
 * Session Attachment Helpers
 *
 * Utility functions for managing session attachment lifecycle.
 * Extracted for separation of concerns - these are side-effect operations
 * that don't need to be in the main hook file.
 *
 * SOLID: Single Responsibility - Only handles attachment operations
 */

import {
  sessionAttach,
  sessionDetach,
  sessionGetMergedOutput,
  type StreamChunk,
} from '@sengac/codelet-napi';

/**
 * Attach to a session with error handling
 */
export function manualAttach(
  sessionId: string,
  onChunk: (err: Error | null, chunk: StreamChunk) => void
): void {
  try {
    sessionAttach(sessionId, onChunk);
  } catch {
    // Ignore attach errors
  }
}

/**
 * Detach from a session with error handling
 */
export function manualDetach(sessionId: string): void {
  try {
    sessionDetach(sessionId);
  } catch {
    // Ignore detach errors
  }
}

/**
 * Get buffered output chunks from a session
 */
export function getSessionChunks(sessionId: string): StreamChunk[] {
  try {
    return sessionGetMergedOutput(sessionId);
  } catch {
    return [];
  }
}
