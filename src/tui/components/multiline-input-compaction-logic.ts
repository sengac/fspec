/**
 * Core Logic Integration: MultiLineInput Compaction Behavior
 *
 * Provides consistent compaction behavior by integrating with core business logic
 * Replaces inline decision-making with centralized state management
 */

import type { CompactionProgress } from '../hooks/useRustSessionState';
import {
  shouldBlockInput,
  getPlaceholderText,
  type CompactionStateSources,
} from '../../core-logic/compaction-state-manager';
import { formatCompactionPlaceholder } from '../../utils/compaction-formatting';

/**
 * Converts MultiLineInput props to CompactionStateSources for core logic integration
 */
export function createCompactionStateSources(
  isCompacting: boolean,
  compactionProgress: CompactionProgress | null,
  localCompactionState?: {
    isActive: boolean;
    progress: CompactionProgress | null;
    trigger: any;
  }
): CompactionStateSources {
  return {
    localProgressState: localCompactionState || {
      isActive: false,
      progress: null,
      trigger: null,
    },
    rustBackendState: {
      isCompacting,
      compactionProgress,
    },
  };
}

/**
 * Determines if input should be blocked based on compaction state
 * Uses core business logic for consistent behavior
 */
export function shouldBlockInputForMultiLineInput(
  isCompacting: boolean,
  compactionProgress: CompactionProgress | null,
  localCompactionState?: {
    isActive: boolean;
    progress: CompactionProgress | null;
    trigger: any;
  }
): boolean {
  const sources = createCompactionStateSources(
    isCompacting,
    compactionProgress,
    localCompactionState
  );
  return shouldBlockInput(sources);
}

/**
 * Gets the display placeholder text using core business logic
 */
export function getDisplayPlaceholderForMultiLineInput(
  isCompacting: boolean,
  compactionProgress: CompactionProgress | null,
  defaultPlaceholder: string,
  localCompactionState?: {
    isActive: boolean;
    progress: CompactionProgress | null;
    trigger: any;
  }
): string {
  const sources = createCompactionStateSources(
    isCompacting,
    compactionProgress,
    localCompactionState
  );
  return getPlaceholderText(
    sources,
    defaultPlaceholder,
    formatCompactionPlaceholder
  );
}

/**
 * Validates that component state is consistent with core logic expectations
 */
export function validateMultiLineInputCompactionState(
  isCompacting: boolean,
  compactionProgress: CompactionProgress | null,
  suppressEnter: boolean
): {
  isValid: boolean;
  warnings: string[];
} {
  const warnings: string[] = [];

  // Check: isCompacting true but suppressEnter false
  if (isCompacting && !suppressEnter) {
    warnings.push(
      'isCompacting=true but suppressEnter=false - input may not be properly blocked'
    );
  }

  // Check: suppressEnter true but not compacting
  if (suppressEnter && !isCompacting) {
    warnings.push(
      'suppressEnter=true but isCompacting=false - may be intentional (overlay active) or incorrect state'
    );
  }

  // Check: compacting without progress
  if (isCompacting && !compactionProgress) {
    warnings.push(
      'isCompacting=true but compactionProgress is null - user will not see progress'
    );
  }

  return {
    isValid: warnings.length === 0,
    warnings,
  };
}
