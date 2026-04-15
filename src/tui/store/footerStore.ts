/**
 * footerStore.ts - Zustand Store for Session Footer State
 *
 * Stores per-session CWD and git branch name, driven by FooterStateUpdate
 * events from Rust's background poller (every 5 seconds).
 *
 * Data flow:
 *   Rust (tokio task, 5s interval) → reads .git/HEAD for branch name → emits FooterStateUpdate
 *   → GlobalSessionStreamManager receives chunk → updates this store
 *   → SessionFooter reads from this store via selector hooks
 *
 * IMPORTANT: The Rust poller ONLY reads the branch name (near-zero CPU).
 * It does NOT call get_staged_files, get_unstaged_files, or get_untracked_files.
 */

import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { logger } from '../../utils/logger';

/** Git info for a single session — branch name only, no status indicators */
export interface FooterGitStatus {
  isGitRepo: boolean;
  branch: string | null;
}

/** Complete footer state for a single session */
export interface FooterState {
  cwd: string;
  displayPath: string;
  git: FooterGitStatus;
}

/** Store state: maps session IDs to their footer state */
export interface FooterStoreState {
  /** Per-session footer state, keyed by session ID */
  sessions: Record<string, FooterState>;

  // ===== Actions =====

  /** Update footer state for a session (called when FooterStateUpdate arrives from Rust) */
  updateFooterState: (
    sessionId: string,
    cwd: string,
    displayPath: string,
    isGitRepo: boolean,
    branch: string | null
  ) => void;

  /** Remove footer state for a session (called on session destroy) */
  removeSession: (sessionId: string) => void;

  /** Full reset - clears all state. Used for testing. */
  reset: () => void;
}

/** Default empty footer state */
export const EMPTY_FOOTER_STATE: FooterState = Object.freeze({
  cwd: '',
  displayPath: '',
  git: Object.freeze({
    isGitRepo: false,
    branch: null,
  }),
});

export const useFooterStore = create<FooterStoreState>()(
  immer(set => ({
    sessions: {},

    updateFooterState: (
      sessionId: string,
      cwd: string,
      displayPath: string,
      isGitRepo: boolean,
      branch: string | null
    ) => {
      logger.debug(
        `[FooterStore] updateFooterState: ${sessionId} cwd=${displayPath} branch=${branch ?? '(detached)'}`
      );
      set(state => {
        state.sessions[sessionId] = {
          cwd,
          displayPath,
          git: { isGitRepo, branch },
        };
      });
    },

    removeSession: (sessionId: string) => {
      logger.debug(`[FooterStore] removeSession: ${sessionId}`);
      set(state => {
        delete state.sessions[sessionId];
      });
    },

    reset: () => {
      logger.debug('[FooterStore] reset');
      set(state => {
        state.sessions = {};
      });
    },
  }))
);

// ===== Selector hooks (avoids re-renders from unrelated sessions) =====

/**
 * Get footer state for a specific session.
 * Returns EMPTY_FOOTER_STATE if session has no footer data yet.
 */
export const useSessionFooterState = (
  sessionId: string | null
): FooterState => {
  return useFooterStore(state => {
    if (!sessionId) {
      return EMPTY_FOOTER_STATE;
    }
    return state.sessions[sessionId] ?? EMPTY_FOOTER_STATE;
  });
};
