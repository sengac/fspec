/**
 * useRustSessionState - Hook for subscribing to Rust session state using useSyncExternalStore
 *
 * This hook provides React components with a reactive connection to Rust session state.
 * It uses React 18's useSyncExternalStore to properly subscribe to external state changes.
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
// Global subscription registry
// =============================================================================

type Subscriber = () => void;

interface SessionSubscription {
  subscribers: Set<Subscriber>;
  version: number;
  cachedSnapshot: RustSessionSnapshot | null;
  cachedVersion: number;
}

const subscriptions = new Map<string, SessionSubscription>();

function getSubscription(sessionId: string): SessionSubscription {
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
// Snapshot comparison helpers
// =============================================================================

function tokensEqual(a: SessionTokens, b: SessionTokens): boolean {
  return a.inputTokens === b.inputTokens && a.outputTokens === b.outputTokens;
}

function modelEqual(a: SessionModel | null, b: SessionModel | null): boolean {
  if (a === null && b === null) return true;
  if (a === null || b === null) return false;
  return a.providerId === b.providerId && a.modelId === b.modelId;
}

function snapshotEqual(
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
// Snapshot functions
// =============================================================================

/**
 * Fetch fresh state from Rust and return a snapshot.
 * Returns cached snapshot if version matches AND data is unchanged (for React stability).
 */
function getSessionSnapshot(sessionId: string): RustSessionSnapshot {
  const sub = getSubscription(sessionId);

  // Fetch fresh state from Rust
  let status = 'idle';
  let model: SessionModel | null = null;
  let tokens: SessionTokens = { inputTokens: 0, outputTokens: 0 };
  let isDebugEnabled = false;

  try {
    status = sessionGetStatus(sessionId);
  } catch {
    // Session may not exist yet
  }

  try {
    model = sessionGetModel(sessionId);
  } catch {
    // Ignore
  }

  try {
    tokens = sessionGetTokens(sessionId);
  } catch {
    // Ignore
  }

  try {
    isDebugEnabled = sessionGetDebugEnabled(sessionId);
  } catch {
    // Ignore
  }

  const newSnapshot: RustSessionSnapshot = {
    status,
    isLoading: status === 'running',
    model,
    tokens,
    isDebugEnabled,
    version: sub.version,
  };

  // Return cached snapshot if data is unchanged (React requires stable references)
  if (
    sub.cachedSnapshot &&
    sub.cachedVersion === sub.version &&
    snapshotEqual(sub.cachedSnapshot, newSnapshot)
  ) {
    return sub.cachedSnapshot;
  }

  // Cache and return new snapshot
  sub.cachedSnapshot = newSnapshot;
  sub.cachedVersion = sub.version;
  return newSnapshot;
}

const EMPTY_SNAPSHOT: RustSessionSnapshot = {
  status: 'idle',
  isLoading: false,
  model: null,
  tokens: { inputTokens: 0, outputTokens: 0 },
  isDebugEnabled: false,
  version: 0,
};

function getEmptySnapshot(): RustSessionSnapshot {
  return EMPTY_SNAPSHOT;
}

// =============================================================================
// Main hook implementation
// =============================================================================

export function useRustSessionState(sessionId: string | null): {
  snapshot: RustSessionSnapshot;
  refresh: () => void;
} {
  const subscribe = useCallback(
    (callback: () => void): (() => void) => {
      if (!sessionId) {
        return () => {};
      }

      const sub = getSubscription(sessionId);
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

  const refresh = useCallback(() => {
    if (sessionId) {
      refreshSessionState(sessionId);
    }
  }, [sessionId]);

  return { snapshot, refresh };
}

// =============================================================================
// Utility hooks
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
