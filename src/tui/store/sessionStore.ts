/**
 * sessionStore.ts - Zustand Store for Session State Management
 *
 * VIEWNV-001: This store manages session UI state for AgentView.
 *
 * Key Responsibilities:
 * 1. Track current session ID (for AgentView's session creation logic)
 * 2. Track if ready to create new session on first message
 * 3. Manage navigation target (BoardView → AgentView handoff)
 * 4. Manage create session dialog visibility
 *
 * Note: Rust's SessionManager separately tracks the "active" session for
 * navigation purposes (via sessionAttach/sessionDetach). This store handles
 * React-side state that doesn't belong in Rust.
 */

import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { useShallow } from 'zustand/react/shallow';
import { logger } from '../../utils/logger';

/**
 * Session store state
 */
interface SessionStoreState {
  // ===== Core Session State =====

  /**
   * Currently active session ID in AgentView.
   * null means no session is active (board view or ready for new session).
   */
  currentSessionId: string | null;

  /**
   * Whether AgentView should create a new session on next message.
   *
   * INVARIANT: If currentSessionId is null, this MUST be true.
   * The prepareForNewSession() action enforces this atomically.
   */
  isReadyForNewSession: boolean;

  // ===== Navigation State =====

  /**
   * Target session ID for navigation (set by BoardView, consumed by AgentView).
   * When set, AgentView should resume this session on mount.
   */
  navigationTargetSessionId: string | null;

  // ===== UI State =====

  /**
   * Whether the create session confirmation dialog is visible.
   */
  showCreateSessionDialog: boolean;

  // ===== Actions =====

  /**
   * Activate a session (called when session is created or resumed).
   */
  activateSession: (sessionId: string) => void;

  /**
   * Prepare for creating a new session.
   * ATOMIC: Sets currentSessionId=null AND isReadyForNewSession=true together.
   */
  prepareForNewSession: () => void;

  /**
   * Set navigation target (called by BoardView when navigating to a session).
   */
  setNavigationTarget: (sessionId: string | null) => void;

  /**
   * Clear navigation target (called after AgentView consumes it).
   */
  clearNavigationTarget: () => void;

  /**
   * Open the create session confirmation dialog.
   */
  openCreateSessionDialog: () => void;

  /**
   * Close the create session confirmation dialog.
   */
  closeCreateSessionDialog: () => void;

  /**
   * Full reset - clears all state. Used for testing.
   */
  reset: () => void;
}

/**
 * Initial state values
 */
const initialState = {
  currentSessionId: null,
  isReadyForNewSession: true,
  navigationTargetSessionId: null,
  showCreateSessionDialog: false,
};

/**
 * Session store
 */
export const useSessionStore = create<SessionStoreState>()(
  immer(set => ({
    ...initialState,

    activateSession: (sessionId: string) => {
      logger.debug(`[SessionStore] activateSession: ${sessionId}`);
      set(state => {
        state.currentSessionId = sessionId;
        state.isReadyForNewSession = false;
      });
    },

    prepareForNewSession: () => {
      logger.debug('[SessionStore] prepareForNewSession');
      set(state => {
        // ATOMIC: Both must change together to maintain invariant
        state.currentSessionId = null;
        state.isReadyForNewSession = true;
        state.showCreateSessionDialog = false;
      });
    },

    setNavigationTarget: (sessionId: string | null) => {
      logger.debug(`[SessionStore] setNavigationTarget: ${sessionId}`);
      set(state => {
        state.navigationTargetSessionId = sessionId;
      });
    },

    clearNavigationTarget: () => {
      logger.debug('[SessionStore] clearNavigationTarget');
      set(state => {
        state.navigationTargetSessionId = null;
      });
    },

    openCreateSessionDialog: () => {
      logger.debug('[SessionStore] openCreateSessionDialog');
      set(state => {
        state.showCreateSessionDialog = true;
      });
    },

    closeCreateSessionDialog: () => {
      logger.debug('[SessionStore] closeCreateSessionDialog');
      set(state => {
        state.showCreateSessionDialog = false;
      });
    },

    reset: () => {
      logger.debug('[SessionStore] reset');
      set(state => {
        state.currentSessionId = null;
        state.isReadyForNewSession = true;
        state.navigationTargetSessionId = null;
        state.showCreateSessionDialog = false;
      });
    },
  }))
);

/**
 * Selector hooks (avoids re-renders from unused state)
 */
export const useCurrentSessionId = () =>
  useSessionStore(state => state.currentSessionId);

export const useIsReadyForNewSession = () =>
  useSessionStore(state => state.isReadyForNewSession);

export const useNavigationTargetSessionId = () =>
  useSessionStore(state => state.navigationTargetSessionId);

export const useShowCreateSessionDialog = () =>
  useSessionStore(state => state.showCreateSessionDialog);

/**
 * Action hooks (stable references with shallow comparison)
 */
export const useSessionActions = () =>
  useSessionStore(
    useShallow(state => ({
      activateSession: state.activateSession,
      prepareForNewSession: state.prepareForNewSession,
      setNavigationTarget: state.setNavigationTarget,
      clearNavigationTarget: state.clearNavigationTarget,
      openCreateSessionDialog: state.openCreateSessionDialog,
      closeCreateSessionDialog: state.closeCreateSessionDialog,
      reset: state.reset,
    }))
  );
