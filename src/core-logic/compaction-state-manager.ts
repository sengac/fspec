/**
 * Core Business Logic: Compaction State Management
 *
 * Centralized logic for coordinating compaction state from multiple sources:
 * - Manual compaction (/compact command)
 * - Hook-triggered compaction (token thresholds)
 * - Emergency compaction (API rejection)
 * - Rust backend state synchronization
 *
 * SOLID Principles:
 * - Single Responsibility: Only manages compaction state coordination
 * - Open/Closed: Extensible for new compaction triggers
 * - Dependency Inversion: Depends on abstractions, not implementations
 */

import type { CompactionProgress } from '../tui/hooks/useRustSessionState';

export interface CompactionTrigger {
  type: 'manual' | 'hook-triggered' | 'emergency-auto';
  reason: string;
  metadata?: Record<string, any>;
}

export interface CompactionState {
  isActive: boolean;
  progress: CompactionProgress | null;
  trigger: CompactionTrigger | null;
  startTime: number | null;
}

export interface CompactionStateSources {
  // React local state (manual /compact)
  localProgressState: {
    isActive: boolean;
    progress: CompactionProgress | null;
    trigger: CompactionTrigger | null;
  };

  // Rust backend state (all triggers)
  rustBackendState: {
    isCompacting: boolean;
    compactionProgress: CompactionProgress | null;
  };
}

/**
 * Core Logic: Determines if compaction is actually active
 *
 * Business Rule: Compaction is active if EITHER source indicates it's active
 * This ensures UI updates correctly regardless of trigger type
 */
export function isCompactionActive(sources: CompactionStateSources): boolean {
  return (
    sources.localProgressState.isActive || sources.rustBackendState.isCompacting
  );
}

/**
 * Core Logic: Determines the current compaction progress to display
 *
 * Priority Rules:
 * 1. If local state is active, use local progress (manual /compact)
 * 2. If only Rust state is active, use Rust progress (hooks/emergency)
 * 3. If neither active, return null
 */
export function getCurrentCompactionProgress(
  sources: CompactionStateSources
): CompactionProgress | null {
  // Local state takes priority when active (manual compaction)
  if (
    sources.localProgressState.isActive &&
    sources.localProgressState.progress
  ) {
    return sources.localProgressState.progress;
  }

  // Fall back to Rust state for automatic triggers
  if (
    sources.rustBackendState.isCompacting &&
    sources.rustBackendState.compactionProgress
  ) {
    return sources.rustBackendState.compactionProgress;
  }

  return null;
}

/**
 * Core Logic: Determines which trigger initiated the current compaction
 *
 * Business Rules:
 * - Manual triggers are tracked in local state
 * - Automatic triggers must be inferred from Rust state
 */
export function getCurrentCompactionTrigger(
  sources: CompactionStateSources
): CompactionTrigger | null {
  // If local state is active, return its trigger
  if (
    sources.localProgressState.isActive &&
    sources.localProgressState.trigger
  ) {
    return sources.localProgressState.trigger;
  }

  // If only Rust state is active, infer it's an automatic trigger
  if (sources.rustBackendState.isCompacting) {
    return {
      type: 'hook-triggered', // Could be hook or emergency, but we can't distinguish
      reason: 'Automatic compaction triggered by backend',
      metadata: { source: 'rust-backend' },
    };
  }

  return null;
}

/**
 * Core Logic: Validates compaction state consistency
 *
 * Detects potential bugs where state sources are inconsistent
 */
export function validateCompactionStateConsistency(
  sources: CompactionStateSources
): {
  isValid: boolean;
  warnings: string[];
} {
  const warnings: string[] = [];

  // Check: Local state active but no progress
  if (
    sources.localProgressState.isActive &&
    !sources.localProgressState.progress
  ) {
    warnings.push('Local compaction state is active but missing progress data');
  }

  // Check: Rust state active but no progress
  if (
    sources.rustBackendState.isCompacting &&
    !sources.rustBackendState.compactionProgress
  ) {
    warnings.push('Rust compaction state is active but missing progress data');
  }

  // Check: Both states active with different progress data
  if (
    sources.localProgressState.isActive &&
    sources.rustBackendState.isCompacting &&
    sources.localProgressState.progress &&
    sources.rustBackendState.compactionProgress
  ) {
    const localPhase = sources.localProgressState.progress.phase;
    const rustPhase = sources.rustBackendState.compactionProgress.phase;

    if (localPhase !== rustPhase) {
      warnings.push(
        `State conflict: Local phase "${localPhase}" differs from Rust phase "${rustPhase}"`
      );
    }
  }

  return {
    isValid: warnings.length === 0,
    warnings,
  };
}

/**
 * Core Logic: Creates a unified compaction state from multiple sources
 */
export function createUnifiedCompactionState(
  sources: CompactionStateSources
): CompactionState {
  const isActive = isCompactionActive(sources);
  const progress = getCurrentCompactionProgress(sources);
  const trigger = getCurrentCompactionTrigger(sources);

  return {
    isActive,
    progress,
    trigger,
    startTime: isActive ? Date.now() : null,
  };
}

/**
 * Core Logic: Input blocking decision
 *
 * Business Rule: Block input when ANY compaction is active
 */
export function shouldBlockInput(sources: CompactionStateSources): boolean {
  return isCompactionActive(sources);
}

/**
 * Core Logic: Placeholder text decision
 *
 * Business Rule: Show compaction progress when active, otherwise show default
 */
export function getPlaceholderText(
  sources: CompactionStateSources,
  defaultPlaceholder: string,
  formatFunction: (progress: CompactionProgress) => string
): string {
  const progress = getCurrentCompactionProgress(sources);

  if (progress) {
    return formatFunction(progress);
  }

  return defaultPlaceholder;
}
