/**
 * UX-002: Unified Compaction State Hook
 *
 * Single source of truth for ALL compaction state across ALL triggers:
 * - Manual /compact command
 * - Hook-triggered compaction (token threshold)
 * - Emergency compaction (API rejection)
 *
 * SOLID Principles:
 * - Single Responsibility: Manages compaction state and operations
 * - Open/Closed: Extensible for new triggers via CompactionTrigger type
 * - Dependency Inversion: Depends on NAPI abstractions
 *
 * Architecture:
 * - ONE state source (eliminates 3-way OR in UI)
 * - ALL pathways call startCompaction() to set state IMMEDIATELY
 * - Progress polling handled internally
 * - Retry logic only applies to manual compaction
 */

import { useState, useCallback, useEffect, useRef } from 'react';
import {
  sessionCompact,
  sessionGetCompactionProgress,
  type CompactionResult,
  type CompactionProgress,
} from '@sengac/codelet-napi';

// Re-export for convenience
export type { CompactionResult, CompactionProgress };

/**
 * Compaction trigger types - identifies what initiated compaction
 */
export type CompactionTrigger = 'manual' | 'hook-triggered' | 'emergency';

/**
 * Retry dialog state (only applies to manual compaction)
 */
export interface CompactionRetryState {
  isVisible: boolean;
  error: string;
  retryCount: number;
}

/**
 * Unified compaction state - single source of truth
 */
export interface UnifiedCompactionState {
  /** Whether ANY compaction is currently active */
  isActive: boolean;
  /** Current progress (phase, current, total) */
  progress: CompactionProgress | null;
  /** What triggered this compaction */
  trigger: CompactionTrigger | null;
  /** Session ID being compacted */
  sessionId: string | null;
}

/**
 * Hook return type
 */
export interface CompactionHookReturn {
  /** Unified state - use this for UI */
  state: UnifiedCompactionState;

  /** Start compaction (called by ALL pathways) */
  startCompaction: (
    trigger: CompactionTrigger,
    sessionId: string,
    initialProgress?: CompactionProgress
  ) => void;

  /** End compaction (called when compaction completes or fails) */
  endCompaction: () => void;

  /** Update progress (for polling or stream updates) */
  updateProgress: (progress: CompactionProgress) => void;

  /** Execute manual compaction with retry support */
  performManualCompaction: (sessionId: string) => Promise<CompactionResult>;

  /** Retry state (only for manual compaction failures) */
  retryState: CompactionRetryState;

  /** Clear retry dialog */
  clearRetryState: () => void;

  /** Handle retry dialog option */
  handleRetryOption: (option: 'retry' | 'continue' | 'cancel') => void;
}

const INITIAL_STATE: UnifiedCompactionState = {
  isActive: false,
  progress: null,
  trigger: null,
  sessionId: null,
};

const INITIAL_RETRY_STATE: CompactionRetryState = {
  isVisible: false,
  error: '',
  retryCount: 0,
};

const MAX_AUTO_RETRIES = 1;
const PROGRESS_POLL_INTERVAL_MS = 100;

/**
 * Unified Compaction Hook
 *
 * Usage:
 *   const compaction = useCompaction();
 *
 *   // For manual /compact:
 *   await compaction.performManualCompaction(sessionId);
 *
 *   // For automatic (hook-triggered/emergency) - call from stream callback:
 *   if (chunk.state === 'Compacting') {
 *     compaction.startCompaction('hook-triggered', sessionId);
 *   }
 *
 *   // When compaction ends (CompactionComplete chunk):
 *   compaction.endCompaction();
 *
 *   // In UI - just use ONE source:
 *   isCompacting={compaction.state.isActive}
 *   compactionProgress={compaction.state.progress}
 */
export function useCompaction(): CompactionHookReturn {
  // Unified state - THE SINGLE SOURCE OF TRUTH
  const [state, setState] = useState<UnifiedCompactionState>(INITIAL_STATE);

  // Retry state (manual compaction only)
  const [retryState, setRetryState] =
    useState<CompactionRetryState>(INITIAL_RETRY_STATE);

  // Ref to track if we're actively polling (to avoid stale closures)
  const isPollingRef = useRef(false);

  /**
   * Start compaction - called by ALL pathways
   * Sets state IMMEDIATELY (synchronously) to avoid React batching race conditions
   */
  const startCompaction = useCallback(
    (
      trigger: CompactionTrigger,
      sessionId: string,
      initialProgress?: CompactionProgress
    ) => {
      // Set state IMMEDIATELY - this is the key to avoiding race conditions
      setState({
        isActive: true,
        progress: initialProgress ?? {
          phase: 'Starting',
          current: 0,
          total: 1,
        },
        trigger,
        sessionId,
      });
    },
    []
  );

  /**
   * End compaction - called when compaction completes or fails
   */
  const endCompaction = useCallback(() => {
    setState(INITIAL_STATE);
  }, []);

  /**
   * Update progress - for polling updates or stream progress events
   */
  const updateProgress = useCallback((progress: CompactionProgress) => {
    setState(prev => ({
      ...prev,
      progress,
    }));
  }, []);

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
   * This is used by /compact command - it calls startCompaction internally
   */
  const performManualCompaction = useCallback(
    async (
      sessionId: string,
      isRetry: boolean = false
    ): Promise<CompactionResult> => {
      try {
        // Start compaction IMMEDIATELY (synchronous state update)
        startCompaction('manual', sessionId, {
          phase: 'Analyzing anchors',
          current: 0,
          total: 1,
        });

        // Execute the actual compaction
        const result = await sessionCompact(sessionId);

        // Clear any previous retry state on success
        clearRetryState();

        // Brief delay to show completion, then end
        setTimeout(() => {
          endCompaction();
        }, 1000);

        return result;
      } catch (error) {
        const errorMessage =
          error instanceof Error ? error.message : 'Failed to compact';
        const isNetwork = isNetworkError(errorMessage);
        const currentRetryCount = isRetry
          ? retryState.retryCount
          : retryState.retryCount + 1;

        // End compaction state on error
        endCompaction();

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
    [
      startCompaction,
      endCompaction,
      clearRetryState,
      isNetworkError,
      retryState.retryCount,
    ]
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

  /**
   * Progress polling effect
   * Polls Rust for progress updates while compaction is active
   * (Progress is polled, not streamed, per Rust design decision)
   */
  useEffect(() => {
    if (!state.isActive || !state.sessionId) {
      isPollingRef.current = false;
      return;
    }

    isPollingRef.current = true;
    const sessionId = state.sessionId;

    const pollInterval = setInterval(() => {
      if (!isPollingRef.current) return;

      try {
        const progress = sessionGetCompactionProgress(sessionId);
        if (progress) {
          updateProgress(progress);
        }
      } catch {
        // Session might have ended - ignore errors
      }
    }, PROGRESS_POLL_INTERVAL_MS);

    return () => {
      isPollingRef.current = false;
      clearInterval(pollInterval);
    };
  }, [state.isActive, state.sessionId, updateProgress]);

  return {
    state,
    startCompaction,
    endCompaction,
    updateProgress,
    performManualCompaction,
    retryState,
    clearRetryState,
    handleRetryOption,
  };
}
