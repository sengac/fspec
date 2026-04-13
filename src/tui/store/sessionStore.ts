/**
 * sessionStore.ts - Zustand Store for Session State Management
 *
 * Key Responsibilities:
 * 1. Track current session ID (for AgentView's session creation logic)
 * 2. Track if ready to create new session on first message
 * 3. Manage navigation target (BoardView → AgentView handoff)
 * 4. Manage create session dialog visibility
 * 5. Track if session should be auto-created immediately
 * 6. Track current work unit ID and status for SessionHeader display
 *
 * IMPORTANT: This store syncs active session state with Rust's SessionManager
 * via sessionSetActive(). This ensures fspec CLI commands can detect the
 * active session when running within a TUI session.
 */

import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { logger } from '../../utils/logger';
import { sessionSetActive, sessionClearActive } from '@sengac/codelet-napi';

/** Session store state */
export interface SessionStoreState {
  /** Currently active session ID in AgentView. null = no active session. */
  currentSessionId: string | null;
  /** Whether AgentView should create a new session on next message.
   *  INVARIANT: If currentSessionId is null, this MUST be true. */
  isReadyForNewSession: boolean;
  /** VIEWNV-001: Whether AgentView should auto-create a session immediately.
   *  Set to true when user confirms dialog. Consumed after auto-creation. */
  shouldAutoCreateSession: boolean;
  /** Current work unit ID attached to the active session */
  currentWorkUnitId: string | null;
  /** Current work unit status (e.g., "specifying", "implementing") */
  currentWorkUnitStatus: string | null;
  /** Whether the current session is isolated (has a git worktree) */
  isIsolated: boolean;
  /** Path to the worktree (if isolated) */
  worktreePath: string | null;
  /** GIT-031: Whether the next auto-created session should be isolated */
  pendingIsolatedSession: boolean;
  /** Target session ID for navigation (set by BoardView, consumed by AgentView). */
  navigationTargetSessionId: string | null;
  /** Whether the create session confirmation dialog is visible. */
  showCreateSessionDialog: boolean;

  // ===== Actions =====

  /** Activate a session (called when session is created or resumed). */
  activateSession: (sessionId: string) => void;
  /** Prepare for creating a new session (ATOMIC: clears session + sets ready). */
  prepareForNewSession: () => void;
  /** VIEWNV-001: Request immediate auto-creation of a session. */
  requestAutoCreateSession: () => void;
  /** VIEWNV-001: Clear the auto-create request (after session is created). */
  clearAutoCreateRequest: () => void;
  /** Set current work unit ID and status */
  setCurrentWorkUnit: (
    workUnitId: string | null,
    workUnitStatus: string | null
  ) => void;
  /** GIT-029: Set isolation state for current session */
  setIsolationState: (isIsolated: boolean, worktreePath: string | null) => void;
  /** Set navigation target (called by BoardView when navigating to a session). */
  setNavigationTarget: (sessionId: string | null) => void;
  /** Clear navigation target (called after AgentView consumes it). */
  clearNavigationTarget: () => void;
  /** Open the create session confirmation dialog. */
  openCreateSessionDialog: () => void;
  /** Close the create session confirmation dialog. */
  closeCreateSessionDialog: () => void;
  /** Full reset - clears all state. Used for testing. */
  reset: () => void;
  /** Navigate to AgentView with auto-create session.
   *  @param isolated - Whether the session should be isolated (default: false) */
  navigateToNewSession: (isolated?: boolean) => void;
}

/** Options for the clearAndResetSession helper */
interface ClearAndResetOptions {
  shouldAutoCreateSession?: boolean;
  pendingIsolatedSession?: boolean;
  navigationTargetSessionId?: string | null;
}

/** Immer-compatible set function type */
type ImmerSet = (fn: (state: SessionStoreState) => void) => void;

/**
 * Reset common session state fields. Called by all session transition actions.
 * Clears the Rust active session and resets shared fields atomically.
 */
function clearAndResetSession(
  set: ImmerSet,
  options?: ClearAndResetOptions
): void {
  try {
    sessionClearActive();
  } catch (e) {
    logger.warn(`[SessionStore] Failed to clear active session in Rust: ${e}`);
  }
  set(state => {
    state.currentSessionId = null;
    state.isReadyForNewSession = true;
    state.showCreateSessionDialog = false;
    state.currentWorkUnitId = null;
    state.currentWorkUnitStatus = null;
    state.isIsolated = false;
    state.worktreePath = null;
    state.shouldAutoCreateSession = options?.shouldAutoCreateSession ?? false;
    state.pendingIsolatedSession = options?.pendingIsolatedSession ?? false;
    state.navigationTargetSessionId =
      options?.navigationTargetSessionId ?? null;
  });
}

/** Initial state values */
const initialState = {
  currentSessionId: null,
  isReadyForNewSession: true,
  shouldAutoCreateSession: false,
  currentWorkUnitId: null,
  currentWorkUnitStatus: null,
  isIsolated: false,
  worktreePath: null,
  pendingIsolatedSession: false,
  navigationTargetSessionId: null,
  showCreateSessionDialog: false,
};

/** Session store */
export const useSessionStore = create<SessionStoreState>()(
  immer(set => ({
    ...initialState,

    activateSession: (sessionId: string) => {
      logger.debug(`[SessionStore] activateSession: ${sessionId}`);
      try {
        sessionSetActive(sessionId);
      } catch (e) {
        logger.warn(
          `[SessionStore] Failed to set active session in Rust: ${e}`
        );
      }
      set(state => {
        state.currentSessionId = sessionId;
        state.isReadyForNewSession = false;
        state.shouldAutoCreateSession = false;
      });
    },

    prepareForNewSession: () => {
      logger.debug('[SessionStore] prepareForNewSession');
      clearAndResetSession(set);
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

    setCurrentWorkUnit: (
      workUnitId: string | null,
      workUnitStatus: string | null
    ) => {
      logger.debug(
        `[SessionStore] setCurrentWorkUnit: ${workUnitId} (${workUnitStatus})`
      );
      set(state => {
        state.currentWorkUnitId = workUnitId;
        state.currentWorkUnitStatus = workUnitStatus;
      });
    },

    setIsolationState: (isIsolated: boolean, worktreePath: string | null) => {
      logger.debug(
        `[SessionStore] setIsolationState: isIsolated=${isIsolated}, worktreePath=${worktreePath}`
      );
      set(state => {
        state.isIsolated = isIsolated;
        state.worktreePath = worktreePath;
        state.pendingIsolatedSession = false;
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
      clearAndResetSession(set);
    },

    navigateToNewSession: (isolated = false) => {
      logger.debug(`[SessionStore] navigateToNewSession: isolated=${isolated}`);
      clearAndResetSession(set, {
        shouldAutoCreateSession: true,
        pendingIsolatedSession: isolated,
      });
    },
  }))
);

// ===== Selector hooks (avoids re-renders from unused state) =====

export const useCurrentSessionId = () =>
  useSessionStore(state => state.currentSessionId);
export const useIsReadyForNewSession = () =>
  useSessionStore(state => state.isReadyForNewSession);
export const useShouldAutoCreateSession = () =>
  useSessionStore(state => state.shouldAutoCreateSession);
export const useCurrentWorkUnitId = () =>
  useSessionStore(state => state.currentWorkUnitId);
export const useCurrentWorkUnitStatus = () =>
  useSessionStore(state => state.currentWorkUnitStatus);
export const useNavigationTargetSessionId = () =>
  useSessionStore(state => state.navigationTargetSessionId);
export const useShowCreateSessionDialog = () =>
  useSessionStore(state => state.showCreateSessionDialog);
export const useIsIsolated = () => useSessionStore(state => state.isIsolated);
export const useWorktreePath = () =>
  useSessionStore(state => state.worktreePath);
export const usePendingIsolatedSession = () =>
  useSessionStore(state => state.pendingIsolatedSession);

// ===== Re-export action hooks from sessionActions =====

export { useSessionActions } from './sessionActions';
