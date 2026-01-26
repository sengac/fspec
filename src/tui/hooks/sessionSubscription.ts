/**
 * Session Subscription Management
 *
 * Manages subscriptions to Rust session state changes using a pub/sub pattern.
 * Each session has its own subscription tracking with version-based cache invalidation.
 *
 * SOLID: Single Responsibility - Only handles subscription lifecycle
 * Composable: Can be used by any hook that needs to track session state
 */

type Subscriber = () => void;

export interface SessionSubscription {
  subscribers: Set<Subscriber>;
  version: number;
  cachedSnapshot: unknown | null;
  cachedVersion: number;
}

const subscriptions = new Map<string, SessionSubscription>();

/**
 * Get or create a subscription for a session ID.
 * Creates a new subscription entry if one doesn't exist.
 */
export function getOrCreateSubscription(
  sessionId: string
): SessionSubscription {
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
 * Invalidate the cache for a session, forcing the next getSnapshot to fetch fresh state.
 * This is the key fix for the detached Done issue - when we subscribe to a session,
 * we invalidate its cache so we always get fresh state from Rust on first read.
 */
export function invalidateCache(sessionId: string): void {
  const sub = subscriptions.get(sessionId);
  if (sub) {
    sub.version++;
  }
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
