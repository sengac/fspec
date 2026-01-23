/**
 * useRustSessionState - Hook for subscribing to Rust session state using useSyncExternalStore
 *
 * This hook provides React components with a reactive connection to Rust session state.
 * It uses React 18's useSyncExternalStore to properly subscribe to external state changes.
 *
 * CRITICAL: getSnapshot MUST return a cached reference when version hasn't changed.
 * React may call getSnapshot multiple times during a single render, and returning
 * different object references will cause infinite loops.
 */

import { useSyncExternalStore, useCallback } from 'react';
import {
  sessionGetStatus,
  sessionGetModel,
  sessionGetTokens,
  sessionGetDebugEnabled,
  sessionGetMergedOutput,
  sessionAttach,
  sessionDetach,
  type StreamChunk,
  type SessionModel,
  type SessionTokens,
} from '@sengac/codelet-napi';

// =============================================================================
// Types
// =============================================================================

export interface RustSessionSnapshot {
  status: string;
  isLoading: boolean;
  model: SessionModel | null;
  tokens: SessionTokens;
  isDebugEnabled: boolean;
  version: number;
}

// =============================================================================
// Constants
// =============================================================================

const DEFAULT_TOKENS: SessionTokens = Object.freeze({
  inputTokens: 0,
  outputTokens: 0,
});

const EMPTY_SNAPSHOT: RustSessionSnapshot = Object.freeze({
  status: 'idle',
  isLoading: false,
  model: null,
  tokens: DEFAULT_TOKENS,
  isDebugEnabled: false,
  version: 0,
});

// =============================================================================
// Session Subscription Management (Single Responsibility)
// =============================================================================

type Subscriber = () => void;

interface SessionSubscription {
  subscribers: Set<Subscriber>;
  version: number;
  cachedSnapshot: RustSessionSnapshot | null;
  cachedVersion: number;
}

const subscriptions = new Map<string, SessionSubscription>();

function getOrCreateSubscription(sessionId: string): SessionSubscription {
  let sub = subscriptions.get(sessionId);
  if (!sub) {
    sub = {
      subscribers: new Set(),
      version: 0,
      cachedSnapshot: null,
      cachedVersion: -1,
    };
    subscriptions.set(sessionId, sub);
  }
  return sub;
}

/**
 * Notify all subscribers that state has changed for a session.
 * Call this after any operation that changes Rust state.
 */
export function refreshSessionState(sessionId: string): void {
  const sub = subscriptions.get(sessionId);
  if (sub) {
    sub.version++;
    sub.subscribers.forEach(callback => callback());
  }
}

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
    tokensEqual(a.tokens, b.tokens) &&
    modelEqual(a.model, b.model)
  );
}

// =============================================================================
// Rust State Fetching (Composable - injectable for testing)
// =============================================================================

export interface RustStateSource {
  getStatus(sessionId: string): string;
  getModel(sessionId: string): SessionModel | null;
  getTokens(sessionId: string): SessionTokens;
  getDebugEnabled(sessionId: string): boolean;
}

// Default implementation using actual NAPI calls
const defaultRustStateSource: RustStateSource = {
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
};

// Injectable state source for testing
let rustStateSource: RustStateSource = defaultRustStateSource;

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

// =============================================================================
// Snapshot Creation (Single Responsibility)
// =============================================================================

function fetchFreshSnapshot(
  sessionId: string,
  version: number
): RustSessionSnapshot {
  const status = rustStateSource.getStatus(sessionId);
  return {
    status,
    isLoading: status === 'running',
    model: rustStateSource.getModel(sessionId),
    tokens: rustStateSource.getTokens(sessionId),
    isDebugEnabled: rustStateSource.getDebugEnabled(sessionId),
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
    return sub.cachedSnapshot;
  }

  // Cache is stale (version mismatch) - fetch fresh state from Rust
  const newSnapshot = fetchFreshSnapshot(sessionId, sub.version);

  // Even after fetching, prefer cached reference if data is unchanged
  // This provides additional stability for React (avoids re-renders when data hasn't changed)
  if (
    sub.cachedSnapshot !== null &&
    snapshotsAreEqual(sub.cachedSnapshot, newSnapshot)
  ) {
    sub.cachedVersion = sub.version; // Mark cache as current version
    return sub.cachedSnapshot; // Return existing reference for React stability
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
  // When creating a new session, the local activeSessionId variable has the correct value
  // before React's batched state update takes effect. Passing it explicitly ensures
  // we refresh the correct session immediately.
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
// Session attachment helpers
// =============================================================================

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

export function manualDetach(sessionId: string): void {
  try {
    sessionDetach(sessionId);
  } catch {
    // Ignore detach errors
  }
}

export function getSessionChunks(sessionId: string): StreamChunk[] {
  try {
    return sessionGetMergedOutput(sessionId);
  } catch {
    return [];
  }
}

// =============================================================================
// Testing utilities (exported for test isolation)
// =============================================================================

/**
 * Clear all subscription state - use in test cleanup
 */
export function clearAllSubscriptions(): void {
  subscriptions.clear();
}

/**
 * Get subscription for test inspection (read-only)
 */
export function getSubscriptionForTesting(
  sessionId: string
): Readonly<SessionSubscription> | undefined {
  return subscriptions.get(sessionId);
}

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
