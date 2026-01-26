/**
 * Rust State Source Interface
 *
 * Provides an abstraction layer for fetching session state from Rust via NAPI.
 * This interface allows for dependency injection, making the hook testable
 * without actual NAPI calls.
 *
 * SOLID: Interface Segregation - Minimal interface for state fetching
 * Composable: Injectable for testing with mock implementations
 */

import {
  sessionGetStatus,
  sessionGetModel,
  sessionGetTokens,
  sessionGetDebugEnabled,
  sessionGetPauseState,
  type SessionModel,
  type SessionTokens,
} from '@sengac/codelet-napi';
import { type PauseInfo, parsePauseInfo } from '../types/pause';

// Re-export types for convenience
export type { SessionModel, SessionTokens };

/**
 * Default token values when session doesn't exist or throws
 */
export const DEFAULT_TOKENS: SessionTokens = Object.freeze({
  inputTokens: 0,
  outputTokens: 0,
});

/**
 * Interface for fetching Rust session state.
 * Implementations can be injected for testing.
 */
export interface RustStateSource {
  getStatus(sessionId: string): string;
  getModel(sessionId: string): SessionModel | null;
  getTokens(sessionId: string): SessionTokens;
  getDebugEnabled(sessionId: string): boolean;
  getPauseState(sessionId: string): PauseInfo | null;
}

/**
 * Default implementation using actual NAPI calls.
 * Each method catches errors and returns safe defaults.
 */
export const defaultRustStateSource: RustStateSource = {
  getStatus(sessionId: string): string {
    try {
      return sessionGetStatus(sessionId);
    } catch {
      return 'idle';
    }
  },

  getModel(sessionId: string): SessionModel | null {
    try {
      return sessionGetModel(sessionId);
    } catch {
      return null;
    }
  },

  getTokens(sessionId: string): SessionTokens {
    try {
      return sessionGetTokens(sessionId);
    } catch {
      return DEFAULT_TOKENS;
    }
  },

  getDebugEnabled(sessionId: string): boolean {
    try {
      return sessionGetDebugEnabled(sessionId);
    } catch {
      return false;
    }
  },

  getPauseState(sessionId: string): PauseInfo | null {
    try {
      const state = sessionGetPauseState(sessionId);
      return parsePauseInfo(state);
    } catch {
      return null;
    }
  },
};

// Injectable state source for testing
let rustStateSource: RustStateSource = defaultRustStateSource;

/**
 * Get the current Rust state source
 */
export function getRustStateSource(): RustStateSource {
  return rustStateSource;
}

/**
 * Inject a custom state source (for testing)
 */
export function setRustStateSource(source: RustStateSource): void {
  rustStateSource = source;
}

/**
 * Reset to default NAPI state source
 */
export function resetRustStateSource(): void {
  rustStateSource = defaultRustStateSource;
}
