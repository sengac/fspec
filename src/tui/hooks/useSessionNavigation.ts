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
import { logger } from '../../utils/logger';

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
    logger.debug('[VIEWNV-001] handleShiftRight called');
    const result = navigateRight();

    switch (result.type) {
      case 'session':
        logger.debug(`[VIEWNV-001] Navigating to session ${result.sessionId}`);
        onNavigate(result.sessionId);
        break;
      case 'create-dialog':
        logger.debug('[VIEWNV-001] Showing create session dialog');
        openCreateSessionDialog();
        break;
      case 'board':
        // Shouldn't happen on right navigation, but handle it
        logger.debug(
          '[VIEWNV-001] Unexpected board result on right navigation'
        );
        break;
    }
  }, [onNavigate, openCreateSessionDialog]);

  const handleShiftLeft = useCallback(() => {
    logger.debug('[VIEWNV-001] handleShiftLeft called');
    const result = navigateLeft();

    switch (result.type) {
      case 'session':
        logger.debug(`[VIEWNV-001] Navigating to session ${result.sessionId}`);
        onNavigate(result.sessionId);
        break;
      case 'board':
        logger.debug('[VIEWNV-001] Navigating to board');
        clearActiveSession(); // Tell Rust we're going to board
        onNavigateToBoard();
        break;
      case 'create-dialog':
        // Shouldn't happen on left navigation
        logger.debug(
          '[VIEWNV-001] Unexpected create-dialog on left navigation'
        );
        break;
    }
  }, [onNavigate, onNavigateToBoard]);

  return {
    handleShiftRight,
    handleShiftLeft,
    showCreateSessionDialog,
  };
}
