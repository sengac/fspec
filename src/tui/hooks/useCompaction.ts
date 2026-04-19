/**
 * UX-002: Compaction Operations Hook
 *
 * Manages manual compaction operations and retry dialog state.
 *
 * CMPCT-034: Display state (isActive, progress, trigger, sessionId) has been REMOVED.
 * Rust is the source of truth for compaction status — SessionStatus::Compacting and
 * CompactionProgress are stored per-session in Rust, and useRustSessionState already
 * reads rustSnapshot.isCompacting / rustSnapshot.compactionProgress from Rust via
 * sessionGetStatus() and sessionGetCompactionProgress(). The UI reads from rustSnapshot
 * directly — no local React state duplication needed.
 *
 * This hook now only manages:
 * - Manual /compact command execution (performManualCompaction)
 * - Retry dialog state (retryState, handleRetryOption, clearRetryState)
 *
 * SOLID Principles:
 * - Single Responsibility: Manages compaction operations and retry dialog only
 * - Dependency Inversion: Depends on NAPI abstractions
 */

import { useState, useCallback } from 'react';
import { sessionCompact, type CompactionResult } from '@sengac/codelet-napi';

// Re-export for convenience
export type { CompactionResult };
export type { CompactionProgress } from '@sengac/codelet-napi';

/**
 * Retry dialog state (only applies to manual compaction)
 */
export interface CompactionRetryState {
  isVisible: boolean;
  error: string;
  retryCount: number;
}

/**
 * Hook return type
 *
 * CMPCT-034: Removed state/startCompaction/endCompaction/updateProgress.
 * Display state comes from rustSnapshot.isCompacting / rustSnapshot.compactionProgress.
 */
export interface CompactionHookReturn {
  /** Execute manual compaction with retry support */
  performManualCompaction: (sessionId: string) => Promise<CompactionResult>;

  /** Retry state (only for manual compaction failures) */
  retryState: CompactionRetryState;

  /** Clear retry dialog */
  clearRetryState: () => void;

  /** Handle retry dialog option */
  handleRetryOption: (option: 'retry' | 'continue' | 'cancel') => void;
}

const INITIAL_RETRY_STATE: CompactionRetryState = {
  isVisible: false,
  error: '',
  retryCount: 0,
};

const MAX_AUTO_RETRIES = 1;

/**
 * Compaction Operations Hook
 *
 * CMPCT-034: This hook no longer tracks display state. Rust is the source of truth.
 *
 * Usage:
 *   const compaction = useCompaction();
 *
 *   // For manual /compact:
 *   await compaction.performManualCompaction(sessionId);
 *
 *   // Display state — read from rustSnapshot (useRustSessionState):
 *   isCompacting={rustSnapshot.isCompacting}
 *   compactionProgress={rustSnapshot.compactionProgress}
 */
export function useCompaction(): CompactionHookReturn {
  // Retry state (manual compaction only)
  const [retryState, setRetryState] =
    useState<CompactionRetryState>(INITIAL_RETRY_STATE);

  /**
   * Clear retry state
   */
  const clearRetryState = useCallback(() => {
    setRetryState(INITIAL_RETRY_STATE);
  }, []);

  /**
   * Check if error is transient network issue
   */
  const isNetworkError = useCallback((error: string) => {
    return (
      error.includes('timeout') ||
      error.includes('network') ||
      error.includes('connection') ||
      error.includes('unavailable')
    );
  }, []);

  /**
   * Execute manual compaction with retry support
   *
   * CMPCT-034: No longer calls startCompaction/endCompaction — Rust sets
   * SessionStatus::Compacting before the agent loop processes the compaction
   * instruction, and sets it back to Idle on completion/failure. The UI picks
   * this up via rustSnapshot.isCompacting (useRustSessionState).
   */
  const performManualCompaction = useCallback(
    async (
      sessionId: string,
      isRetry: boolean = false
    ): Promise<CompactionResult> => {
      try {
        // Execute the actual compaction (Rust in-memory setup).
        // Rust sets SessionStatus::Compacting internally.
        // NOTE: Do NOT call endCompaction() here — the DAG construction phase
        // (SessionSearch + inject_summary) runs asynchronously via the agent loop.
        // CompactionComplete chunk will end compaction when the DAG is fully applied.
        const result = await sessionCompact(sessionId);

        // Clear any previous retry state on success
        clearRetryState();

        return result;
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : 'Failed to compact';
        const isNetwork = isNetworkError(errorMessage);
        const currentRetryCount = isRetry
          ? retryState.retryCount
          : retryState.retryCount + 1;

        // Auto-retry for transient network issues
        if (isNetwork && currentRetryCount <= MAX_AUTO_RETRIES && !isRetry) {
          await new Promise(resolve => setTimeout(resolve, 1000));
          setRetryState(prev => ({ ...prev, retryCount: currentRetryCount }));
          return performManualCompaction(sessionId, false);
        }

        // Show retry dialog for persistent failures
        setRetryState({
          isVisible: true,
          error: errorMessage,
          retryCount: currentRetryCount,
        });

        throw error;
      }
    },
    [clearRetryState, isNetworkError, retryState.retryCount]
  );

  /**
   * Handle retry dialog option
   */
  const handleRetryOption = useCallback(
    (option: 'retry' | 'continue' | 'cancel') => {
      switch (option) {
        case 'retry':
          setRetryState(prev => ({ ...prev, isVisible: false }));
          // Caller should call performManualCompaction again
          break;
        case 'continue':
        case 'cancel':
          clearRetryState();
          break;
      }
    },
    [clearRetryState]
  );

  return {
    performManualCompaction,
    retryState,
    clearRetryState,
    handleRetryOption,
  };
}
