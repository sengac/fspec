/**
 * useRustSessionState - Hook for subscribing to Rust session state using useSyncExternalStore
 *
 * This hook provides React components with a reactive connection to Rust session state.
 * It uses React 18's useSyncExternalStore to properly subscribe to external state changes.
 *
 * CRITICAL: getSnapshot MUST return a cached reference when version hasn't changed.
 * React may call getSnapshot multiple times during a single render, and returning
 * different object references will cause infinite loops.
 *
 * Architecture:
 * - Types: src/tui/types/pause.ts (PauseInfo, PauseKind)
 * - Subscriptions: ./sessionSubscription.ts (pub/sub management)
 * - State Source: ./rustStateSource.ts (NAPI abstraction)
 * - Attachment: ./sessionAttachment.ts (attach/detach helpers)
 */

import { useSyncExternalStore, useCallback } from 'react';
import type { SessionModel, SessionTokens } from '@sengac/codelet-napi';
import { type PauseInfo, pauseInfoEqual } from '../types/pause';
import {
  getOrCreateSubscription,
  invalidateCache,
  refreshSessionState,
  clearAllSubscriptions,
  getSubscriptionForTesting,
  type SessionSubscription,
} from './sessionSubscription';
import {
  getRustStateSource,
  setRustStateSource,
  resetRustStateSource,
  DEFAULT_TOKENS,
  type RustStateSource,
} from './rustStateSource';
import { logger } from '../../utils/logger';

// Re-exports for backwards compatibility
export type { PauseInfo } from '../types/pause';
export type { RustStateSource } from './rustStateSource';
export { setRustStateSource, resetRustStateSource } from './rustStateSource';
export {
  refreshSessionState,
  clearAllSubscriptions,
  getSubscriptionForTesting,
} from './sessionSubscription';
export {
  manualAttach,
  manualDetach,
  getSessionChunks,
} from './sessionAttachment';

// =============================================================================
// Types
// =============================================================================

export interface RustSessionSnapshot {
  status: string;
  isLoading: boolean;
  /** PAUSE-001: True when session status is "paused" */
  isPaused: boolean;
  /** PAUSE-001: Pause details when session is paused, null otherwise */
  pauseInfo: PauseInfo | null;
  model: SessionModel | null;
  tokens: SessionTokens;
  isDebugEnabled: boolean;
  version: number;
}

// =============================================================================
// Constants
// =============================================================================

const EMPTY_SNAPSHOT: RustSessionSnapshot = Object.freeze({
  status: 'idle',
  isLoading: false,
  isPaused: false,
  pauseInfo: null,
  model: null,
  tokens: DEFAULT_TOKENS,
  isDebugEnabled: false,
  version: 0,
});

// =============================================================================
// Snapshot Equality (DRY - reusable comparison functions)
// =============================================================================

function tokensEqual(a: SessionTokens, b: SessionTokens): boolean {
  return a.inputTokens === b.inputTokens && a.outputTokens === b.outputTokens;
}

function modelEqual(a: SessionModel | null, b: SessionModel | null): boolean {
  if (a === null && b === null) {
    return true;
  }
  if (a === null || b === null) {
    return false;
  }
  return a.providerId === b.providerId && a.modelId === b.modelId;
}

function snapshotsAreEqual(
  a: RustSessionSnapshot,
  b: RustSessionSnapshot
): boolean {
  return (
    a.status === b.status &&
    a.isDebugEnabled === b.isDebugEnabled &&
    a.isPaused === b.isPaused &&
    pauseInfoEqual(a.pauseInfo, b.pauseInfo) &&
    tokensEqual(a.tokens, b.tokens) &&
    modelEqual(a.model, b.model)
  );
}

// =============================================================================
// Snapshot Creation
// =============================================================================

function fetchFreshSnapshot(
  sessionId: string,
  version: number
): RustSessionSnapshot {
  const source = getRustStateSource();
  const status = source.getStatus(sessionId);
  const isLoading = status === 'running';
  const isPaused = status === 'paused';
  const pauseInfo = isPaused ? source.getPauseState(sessionId) : null;

  return {
    status,
    isLoading,
    isPaused,
    pauseInfo,
    model: source.getModel(sessionId),
    tokens: source.getTokens(sessionId),
    isDebugEnabled: source.getDebugEnabled(sessionId),
    version,
  };
}

/**
 * Get session snapshot with proper caching for useSyncExternalStore.
 *
 * CRITICAL: This function MUST return the cached reference if version hasn't changed.
 * React may call getSnapshot multiple times during a single render cycle.
 * Returning different object references triggers the error:
 * "The result of getSnapshot should be cached to avoid an infinite loop"
 */
function getSessionSnapshot(sessionId: string): RustSessionSnapshot {
  const sub = getOrCreateSubscription(sessionId);

  // CRITICAL FIX: Check cache FIRST, before any Rust/NAPI calls
  // If version matches, return cached snapshot immediately - no external calls needed
  if (sub.cachedSnapshot !== null && sub.cachedVersion === sub.version) {
    return sub.cachedSnapshot as RustSessionSnapshot;
  }

  // Cache is stale (version mismatch) - fetch fresh state from Rust
  const newSnapshot = fetchFreshSnapshot(sessionId, sub.version);

  // Even after fetching, prefer cached reference if data is unchanged
  // This provides additional stability for React (avoids re-renders when data hasn't changed)
  if (
    sub.cachedSnapshot !== null &&
    snapshotsAreEqual(sub.cachedSnapshot as RustSessionSnapshot, newSnapshot)
  ) {
    sub.cachedVersion = sub.version;
    return sub.cachedSnapshot as RustSessionSnapshot;
  }

  // Data actually changed - update cache with new snapshot
  sub.cachedSnapshot = newSnapshot;
  sub.cachedVersion = sub.version;
  return newSnapshot;
}

function getEmptySnapshot(): RustSessionSnapshot {
  return EMPTY_SNAPSHOT;
}

// =============================================================================
// Main hook implementation
// =============================================================================

/**
 * Hook for subscribing to Rust session state.
 *
 * @param sessionId - The session ID to subscribe to (null if no session)
 * @returns Object with snapshot and refresh function
 *
 * The refresh function accepts an optional targetSessionId parameter to handle
 * race conditions when a new session is created. React state updates are batched,
 * so calling setCurrentSessionId() doesn't immediately update the sessionId captured
 * in this hook's closure. By passing the session ID explicitly, callers can ensure
 * the correct session is refreshed even before React re-renders.
 */
export function useRustSessionState(sessionId: string | null): {
  snapshot: RustSessionSnapshot;
  refresh: (targetSessionId?: string | null) => void;
} {
  const subscribe = useCallback(
    (callback: () => void): (() => void) => {
      if (!sessionId) {
        return () => {};
      }

      const sub = getOrCreateSubscription(sessionId);
      sub.subscribers.add(callback);

      // KEY FIX: Invalidate cache on subscribe to force fresh fetch from Rust.
      // This handles the case where a session completed while detached (Done chunk
      // not forwarded because is_attached=false). When we re-subscribe, we need
      // fresh state from Rust, not stale cached state that still says "running".
      invalidateCache(sessionId);

      return () => {
        sub.subscribers.delete(callback);
      };
    },
    [sessionId]
  );

  const getSnapshot = useCallback((): RustSessionSnapshot => {
    if (!sessionId) {
      return getEmptySnapshot();
    }
    return getSessionSnapshot(sessionId);
  }, [sessionId]);

  const snapshot = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);

  // Accept optional targetSessionId to handle race conditions with React state updates.
  const refresh = useCallback(
    (targetSessionId?: string | null) => {
      const sessionToRefresh = targetSessionId ?? sessionId;
      if (sessionToRefresh) {
        refreshSessionState(sessionToRefresh);
      }
    },
    [sessionId]
  );

  return { snapshot, refresh };
}

// =============================================================================
// Utility hooks (Composable - built on main hook)
// =============================================================================

export function useRustIsLoading(sessionId: string | null): boolean {
  const { snapshot } = useRustSessionState(sessionId);
  return snapshot.isLoading;
}

export function useRustTokens(sessionId: string | null): SessionTokens {
  const { snapshot } = useRustSessionState(sessionId);
  return snapshot.tokens;
}

export function useRustModel(sessionId: string | null): SessionModel | null {
  const { snapshot } = useRustSessionState(sessionId);
  return snapshot.model;
}

export function useRustDebugEnabled(sessionId: string | null): boolean {
  const { snapshot } = useRustSessionState(sessionId);
  return snapshot.isDebugEnabled;
}

// =============================================================================
// Testing utilities
// =============================================================================

/**
 * Get empty snapshot constant for test assertions
 */
export function getEmptySnapshotForTesting(): RustSessionSnapshot {
  return EMPTY_SNAPSHOT;
}

/**
 * Direct access to snapshot function for testing caching behavior.
 * This bypasses the React hook wrapper to test the core caching logic.
 */
export function getSessionSnapshotForTesting(
  sessionId: string
): RustSessionSnapshot {
  return getSessionSnapshot(sessionId);
}
