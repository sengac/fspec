/**
 * useWorkUnitContext - React hook for work unit context
 *
 * SOLID: Single Responsibility - Only React integration for work unit context
 * DRY: Delegates to service layer, no duplicated logic
 * COMPOSABLE: Can be composed with other hooks
 *
 * TUI-059: Work Unit Context in Environment Information
 *
 * This hook manages the work unit context for a session:
 * - Sets context when entering AgentView from BoardView with a work unit
 * - Syncs context when session changes
 * - Provides access to current context
 */

import { useCallback, useEffect, useRef } from 'react';
import { useFspecStore } from '../store/fspecStore';
import {
  setWorkUnitContext,
  getWorkUnitContext,
} from '../services/workUnitContextService';
import type { WorkUnitContext } from '../types/workUnitContext';
import { logger } from '../../utils/logger';

interface UseWorkUnitContextOptions {
  /** Current session ID (null if no session) */
  sessionId: string | null;
  /** Work unit ID passed from BoardView (null if entering without work unit) */
  workUnitId?: string | null;
}

interface UseWorkUnitContextResult {
  /** Current work unit context from session (null if not set) */
  currentContext: WorkUnitContext | null;
  /** Set or clear the work unit context for the session */
  setContext: (context: WorkUnitContext | null) => void;
  /** Sync work unit context with session (call when session becomes active) */
  syncWithSession: () => void;
}

/**
 * Hook for managing work unit context in AgentView
 *
 * Usage:
 * ```tsx
 * const { syncWithSession } = useWorkUnitContext({
 *   sessionId: currentSessionId,
 *   workUnitId,
 * });
 *
 * // Call syncWithSession when session is created/resumed
 * useEffect(() => {
 *   if (sessionId) {
 *     syncWithSession();
 *   }
 * }, [sessionId]);
 * ```
 */
export function useWorkUnitContext(
  options: UseWorkUnitContextOptions
): UseWorkUnitContextResult {
  const { sessionId, workUnitId } = options;

  // Track if we've already synced for this session/workUnit combo
  const syncedRef = useRef<string | null>(null);

  // Zustand state for work units
  const workUnits = useFspecStore(state => state.workUnits);

  // Get current context from Rust (only if we have a session)
  const currentContext = sessionId ? getWorkUnitContext(sessionId) : null;

  // Set context (updates Rust state)
  const setContext = useCallback(
    (context: WorkUnitContext | null) => {
      if (!sessionId) {
        logger.debug('[useWorkUnitContext] No session ID, skipping setContext');
        return;
      }

      logger.debug(
        `[useWorkUnitContext] Setting context for session ${sessionId}:`,
        context
      );
      setWorkUnitContext(sessionId, context);
    },
    [sessionId]
  );

  // Sync work unit ID with session context
  const syncWithSession = useCallback(() => {
    if (!sessionId) {
      logger.debug('[useWorkUnitContext] No session ID, skipping sync');
      return;
    }

    if (!workUnitId) {
      logger.debug(
        '[useWorkUnitContext] No work unit ID, clearing context for session'
      );
      setContext(null);
      return;
    }

    // Find work unit in store
    const workUnit = workUnits.find(wu => wu.id === workUnitId);
    if (!workUnit) {
      logger.debug(
        `[useWorkUnitContext] Work unit ${workUnitId} not found in store`
      );
      return;
    }

    // Set context with work unit details
    const context: WorkUnitContext = {
      id: workUnit.id,
      title: workUnit.title,
      status: workUnit.status,
    };

    logger.debug(
      `[useWorkUnitContext] Syncing work unit ${workUnitId} to session ${sessionId}`
    );
    setContext(context);
  }, [sessionId, workUnitId, workUnits, setContext]);

  // Auto-sync when session becomes active with a work unit
  useEffect(() => {
    // Create a key for this session/workUnit combination
    const syncKey = `${sessionId}:${workUnitId}`;

    // Only sync once per session/workUnit combo
    if (sessionId && workUnitId && syncedRef.current !== syncKey) {
      syncedRef.current = syncKey;
      syncWithSession();
    }

    // Clear sync tracking when no session
    if (!sessionId) {
      syncedRef.current = null;
    }
  }, [sessionId, workUnitId, syncWithSession]);

  return {
    currentContext,
    setContext,
    syncWithSession,
  };
}
