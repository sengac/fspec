/**
 * useSessionStreamManager Hook
 *
 * Hook for React components to subscribe to session events via GlobalSessionStreamManager.
 * Registers a handler when the component mounts and unregisters on unmount or sessionId change.
 */

import { useEffect, useRef } from 'react';
import type { StreamChunk } from '@sengac/codelet-napi';
import {
  GlobalSessionStreamManager,
  type SessionChunkHandler,
} from '../services/globalSessionStreamManager';

/**
 * Attach to a session and register a handler.
 *
 * @param sessionId - Session to attach to
 * @param onChunk - Callback for stream chunks (UI events only, not FspecCommandRequest).
 *   Receives the routed sessionId (CMPCT-033) alongside the chunk so background
 *   sessions can be attributed correctly.
 * @returns Cleanup function to unregister
 */
export function attachToSession(
  sessionId: string,
  onChunk: (routedSessionId: string, chunk: StreamChunk) => void
): () => void {
  const manager = GlobalSessionStreamManager.getInstance();
  return manager.attachWithHandler(sessionId, onChunk);
}

/**
 * Hook to subscribe to session stream events.
 *
 * @param sessionId - The session to subscribe to (null/undefined = no subscription)
 * @param onChunk - Callback for stream chunks (UI events only, not FspecCommandRequest).
 *   Receives the routed sessionId (CMPCT-033) alongside the chunk.
 */
export function useSessionStreamManager(
  sessionId: string | null | undefined,
  onChunk: SessionChunkHandler | null | undefined
): void {
  const onChunkRef = useRef(onChunk);
  onChunkRef.current = onChunk;

  useEffect(() => {
    if (!sessionId || !onChunkRef.current) {
      return;
    }

    const manager = GlobalSessionStreamManager.getInstance();
    const handler: SessionChunkHandler = (
      routedSessionId: string,
      chunk: StreamChunk
    ) => {
      if (onChunkRef.current) {
        onChunkRef.current(routedSessionId, chunk);
      }
    };

    const unregister = manager.registerHandler(sessionId, handler);
    return () => {
      unregister();
    };
  }, [sessionId]);
}

/**
 * Hook to subscribe to all session stream events (global handler).
 *
 * @param onChunk - Callback for stream chunks from any session
 */
export function useGlobalSessionStreamManager(
  onChunk: ((sessionId: string, chunk: StreamChunk) => void) | null | undefined
): void {
  const onChunkRef = useRef(onChunk);
  onChunkRef.current = onChunk;

  useEffect(() => {
    if (!onChunkRef.current) {
      return;
    }

    const manager = GlobalSessionStreamManager.getInstance();
    const handler = (sessionId: string, chunk: StreamChunk) => {
      if (onChunkRef.current) {
        onChunkRef.current(sessionId, chunk);
      }
    };

    const unregister = manager.registerGlobalHandler(handler);
    return () => {
      unregister();
    };
  }, []);
}
