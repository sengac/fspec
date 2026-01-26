/**
 * VIEWNV-001: Session Navigation Hook
 *
 * SIMPLE DESIGN: Rust is the source of truth.
 * This hook just wraps the Rust navigation functions and handles callbacks.
 */

import { useCallback } from 'react';
import {
  navigateRight,
  navigateLeft,
  clearActiveSession,
} from '../utils/sessionNavigation';
import {
  useShowCreateSessionDialog,
  useSessionActions,
} from '../store/sessionStore';

// Re-export for convenience
export { clearActiveSession };

interface UseSessionNavigationOptions {
  /** Called when navigating to a session */
  onNavigate: (sessionId: string) => void;
  /** Called when navigating to BoardView */
  onNavigateToBoard: () => void;
}

interface UseSessionNavigationReturn {
  /** Handle Shift+Right key */
  handleShiftRight: () => void;
  /** Handle Shift+Left key */
  handleShiftLeft: () => void;
  /** Whether the create session dialog is shown */
  showCreateSessionDialog: boolean;
}

/**
 * Hook for unified session navigation across all views
 */
export function useSessionNavigation({
  onNavigate,
  onNavigateToBoard,
}: UseSessionNavigationOptions): UseSessionNavigationReturn {
  const showCreateSessionDialog = useShowCreateSessionDialog();
  const { openCreateSessionDialog } = useSessionActions();

  const handleShiftRight = useCallback(() => {
    const result = navigateRight();

    switch (result.type) {
      case 'session':
        onNavigate(result.sessionId);
        break;
      case 'create-dialog':
        openCreateSessionDialog();
        break;
      case 'board':
        // Shouldn't happen on right navigation
        break;
    }
  }, [onNavigate, openCreateSessionDialog]);

  const handleShiftLeft = useCallback(() => {
    const result = navigateLeft();

    switch (result.type) {
      case 'session':
        onNavigate(result.sessionId);
        break;
      case 'board':
        clearActiveSession(); // Tell Rust we're going to board
        onNavigateToBoard();
        break;
      case 'create-dialog':
        // Shouldn't happen on left navigation
        break;
    }
  }, [onNavigate, onNavigateToBoard]);

  return {
    handleShiftRight,
    handleShiftLeft,
    showCreateSessionDialog,
  };
}
