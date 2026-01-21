/**
 * Hook for managing turn selection state in VirtualList-based views
 *
 * SOLID: Single responsibility - encapsulates turn selection state management
 * DRY: Shared pattern between AgentView and SplitSessionView
 *
 * Provides:
 * - Selection mode state (on/off)
 * - Selection ref for accessing VirtualList's selected index
 * - Toggle/exit callbacks for keyboard handling
 */

import { useState, useRef, useCallback, type MutableRefObject } from 'react';

export interface TurnSelectionState {
  /** Whether turn selection mode is active */
  isSelectMode: boolean;
  /** Ref to access VirtualList's current selection index */
  selectionRef: MutableRefObject<{ selectedIndex: number }>;
  /** Toggle selection mode on/off */
  toggleSelectMode: () => void;
  /** Exit selection mode (set to off) */
  exitSelectMode: () => void;
  /** Direct setter for selection mode */
  setSelectMode: (mode: boolean) => void;
}

/**
 * Hook for managing turn selection state.
 *
 * Usage:
 * ```tsx
 * const { isSelectMode, selectionRef, toggleSelectMode } = useTurnSelection();
 *
 * // In keyboard handler
 * if (key.tab) toggleSelectMode();
 *
 * // In VirtualList
 * <VirtualList
 *   selectionMode={isSelectMode ? 'item' : 'scroll'}
 *   selectionRef={selectionRef}
 *   ...
 * />
 * ```
 */
export function useTurnSelection(): TurnSelectionState {
  const [isSelectMode, setSelectMode] = useState(false);
  const selectionRef = useRef<{ selectedIndex: number }>({ selectedIndex: 0 });

  const toggleSelectMode = useCallback(() => {
    setSelectMode(prev => !prev);
  }, []);

  const exitSelectMode = useCallback(() => {
    setSelectMode(false);
  }, []);

  return {
    isSelectMode,
    selectionRef,
    toggleSelectMode,
    exitSelectMode,
    setSelectMode,
  };
}
