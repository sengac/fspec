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
  sessionGetBaseThinkingLevel,
  sessionSetBaseThinkingLevel,
  sessionGetCompactionProgress,
  sessionGetHitlRequest,
  type SessionModel,
  type SessionTokens,
  type CompactionProgress,
} from '@sengac/codelet-napi';
import { type PauseInfo, parsePauseInfo } from '../types/pause';
import {
  type HitlRequestInfo,
  parseHitlRequestInfo,
} from '../types/hitlRequest';

// Re-export types for convenience
export type { SessionModel, SessionTokens, CompactionProgress };

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
  /** TUI-054: Get the base thinking level (0=Off, 1=Low, 2=Medium, 3=High) */
  getBaseThinkingLevel(sessionId: string): number;
  /** TUI-054: Set the base thinking level (0=Off, 1=Low, 2=Medium, 3=High) */
  setBaseThinkingLevel(sessionId: string, level: number): void;
  /** PERF-002: Get compaction progress when session is compacting */
  getCompactionProgress(sessionId: string): CompactionProgress | null;
  /** BUG-118: Get HITL request when session is paused for user input */
  getHitlRequest(sessionId: string): HitlRequestInfo | null;
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

  /** TUI-054: Get the base thinking level (0=Off, 1=Low, 2=Medium, 3=High) */
  getBaseThinkingLevel(sessionId: string): number {
    try {
      return sessionGetBaseThinkingLevel(sessionId);
    } catch {
      return 0; // Default to Off
    }
  },

  /** TUI-054: Set the base thinking level (0=Off, 1=Low, 2=Medium, 3=High) */
  setBaseThinkingLevel(sessionId: string, level: number): void {
    try {
      sessionSetBaseThinkingLevel(sessionId, level);
    } catch {
      // Silently fail - state will be stale but won't crash
    }
  },

  /** PERF-002: Get compaction progress when session is compacting */
  getCompactionProgress(sessionId: string): CompactionProgress | null {
    try {
      return sessionGetCompactionProgress(sessionId);
    } catch {
      return null;
    }
  },

  /** BUG-118: Get HITL request when session is paused for user input */
  getHitlRequest(sessionId: string): HitlRequestInfo | null {
    try {
      const state = sessionGetHitlRequest(sessionId);
      return parseHitlRequestInfo(state);
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
