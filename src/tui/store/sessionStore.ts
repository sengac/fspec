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
 * 5. VIEWNV-001: Track if session should be auto-created immediately
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

  /**
   * VIEWNV-001: Whether AgentView should auto-create a session immediately.
   * Set to true when user confirms "Start New Agent?" dialog.
   * Consumed (set to false) after auto-creation completes.
   * This allows /thinking and other commands to work right away.
   */
  shouldAutoCreateSession: boolean;

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
   * VIEWNV-001: Request immediate auto-creation of a session.
   * Called when user confirms "Start New Agent?" dialog.
   */
  requestAutoCreateSession: () => void;

  /**
   * VIEWNV-001: Clear the auto-create request (after session is created).
   */
  clearAutoCreateRequest: () => void;

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

  /**
   * VIEWNV-001: Navigate to AgentView with auto-create session.
   * This is the canonical way to start a new agent session.
   * Used by both BoardView's Enter key (on story) and CreateSessionDialog confirmation.
   *
   * Combines: prepareForNewSession() + requestAutoCreateSession()
   */
  navigateToNewSession: () => void;
}

/**
 * Initial state values
 */
const initialState = {
  currentSessionId: null,
  isReadyForNewSession: true,
  shouldAutoCreateSession: false,
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
        state.shouldAutoCreateSession = false;
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

    requestAutoCreateSession: () => {
      logger.debug('[SessionStore] requestAutoCreateSession');
      set(state => {
        state.shouldAutoCreateSession = true;
      });
    },

    clearAutoCreateRequest: () => {
      logger.debug('[SessionStore] clearAutoCreateRequest');
      set(state => {
        state.shouldAutoCreateSession = false;
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
        state.shouldAutoCreateSession = false;
        state.navigationTargetSessionId = null;
        state.showCreateSessionDialog = false;
      });
    },

    navigateToNewSession: () => {
      logger.debug('[SessionStore] navigateToNewSession');
      set(state => {
        // ATOMIC: Prepare for new session + request auto-create together
        state.currentSessionId = null;
        state.isReadyForNewSession = true;
        state.shouldAutoCreateSession = true;
        state.showCreateSessionDialog = false;
        state.navigationTargetSessionId = null;
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

export const useShouldAutoCreateSession = () =>
  useSessionStore(state => state.shouldAutoCreateSession);

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
      requestAutoCreateSession: state.requestAutoCreateSession,
      clearAutoCreateRequest: state.clearAutoCreateRequest,
      setNavigationTarget: state.setNavigationTarget,
      clearNavigationTarget: state.clearNavigationTarget,
      openCreateSessionDialog: state.openCreateSessionDialog,
      closeCreateSessionDialog: state.closeCreateSessionDialog,
      navigateToNewSession: state.navigateToNewSession,
      reset: state.reset,
    }))
  );
