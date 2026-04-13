/**
 * sessionActions.ts - Action hooks for Session Store
 *
 * Provides stable action references using shallow comparison
 * to avoid unnecessary re-renders.
 */

import { useShallow } from 'zustand/react/shallow';
import { useSessionStore } from './sessionStore';

/**
 * Hook that returns all session store actions with stable references.
 * Uses useShallow to prevent re-renders when only state (not actions) changes.
 */
export const useSessionActions = () =>
  useSessionStore(
    useShallow(state => ({
      activateSession: state.activateSession,
      prepareForNewSession: state.prepareForNewSession,
      requestAutoCreateSession: state.requestAutoCreateSession,
      clearAutoCreateRequest: state.clearAutoCreateRequest,
      setCurrentWorkUnit: state.setCurrentWorkUnit,
      setIsolationState: state.setIsolationState,
      setNavigationTarget: state.setNavigationTarget,
      clearNavigationTarget: state.clearNavigationTarget,
      openCreateSessionDialog: state.openCreateSessionDialog,
      closeCreateSessionDialog: state.closeCreateSessionDialog,
      navigateToNewSession: state.navigateToNewSession,
      reset: state.reset,
    }))
  );
