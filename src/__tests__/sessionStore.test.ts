/**
 * sessionStore.test.ts - Tests for the session state store
 *
 * VIEWNV-001: Session state management for AgentView
 *
 * Tests verify:
 * - Atomic state transitions
 * - State machine invariants
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { useSessionStore } from '../tui/store/sessionStore';

describe('sessionStore', () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useSessionStore.getState().reset();
  });

  describe('initial state', () => {
    it('should start with no session and ready for new session', () => {
      const state = useSessionStore.getState();
      expect(state.currentSessionId).toBeNull();
      expect(state.isReadyForNewSession).toBe(true);
      expect(state.showCreateSessionDialog).toBe(false);
      expect(state.navigationTargetSessionId).toBeNull();
    });
  });

  describe('activateSession', () => {
    it('should set currentSessionId and clear ready flag atomically', () => {
      const { activateSession } = useSessionStore.getState();

      activateSession('session-123');

      const state = useSessionStore.getState();
      expect(state.currentSessionId).toBe('session-123');
      expect(state.isReadyForNewSession).toBe(false);
    });

    it('should allow switching between sessions', () => {
      const { activateSession } = useSessionStore.getState();

      activateSession('session-1');
      expect(useSessionStore.getState().currentSessionId).toBe('session-1');

      activateSession('session-2');
      expect(useSessionStore.getState().currentSessionId).toBe('session-2');
      expect(useSessionStore.getState().isReadyForNewSession).toBe(false);
    });
  });

  describe('prepareForNewSession', () => {
    it('should clear session and set ready flag atomically', () => {
      const { activateSession, prepareForNewSession } =
        useSessionStore.getState();

      // First activate a session
      activateSession('session-123');
      expect(useSessionStore.getState().currentSessionId).toBe('session-123');
      expect(useSessionStore.getState().isReadyForNewSession).toBe(false);

      // Now prepare for new session
      prepareForNewSession();

      const state = useSessionStore.getState();
      expect(state.currentSessionId).toBeNull();
      expect(state.isReadyForNewSession).toBe(true);
    });

    it('should close create dialog if open', () => {
      const { openCreateSessionDialog, prepareForNewSession } =
        useSessionStore.getState();

      openCreateSessionDialog();
      expect(useSessionStore.getState().showCreateSessionDialog).toBe(true);

      prepareForNewSession();
      expect(useSessionStore.getState().showCreateSessionDialog).toBe(false);
    });
  });

  describe('create session dialog', () => {
    it('should open and close dialog', () => {
      const { openCreateSessionDialog, closeCreateSessionDialog } =
        useSessionStore.getState();

      expect(useSessionStore.getState().showCreateSessionDialog).toBe(false);

      openCreateSessionDialog();
      expect(useSessionStore.getState().showCreateSessionDialog).toBe(true);

      closeCreateSessionDialog();
      expect(useSessionStore.getState().showCreateSessionDialog).toBe(false);
    });
  });

  describe('navigation target', () => {
    it('should set and clear navigation target', () => {
      const { setNavigationTarget, clearNavigationTarget } =
        useSessionStore.getState();

      expect(useSessionStore.getState().navigationTargetSessionId).toBeNull();

      setNavigationTarget('target-session');
      expect(useSessionStore.getState().navigationTargetSessionId).toBe(
        'target-session'
      );

      clearNavigationTarget();
      expect(useSessionStore.getState().navigationTargetSessionId).toBeNull();
    });
  });

  describe('full reset', () => {
    it('should reset all state to initial values', () => {
      const {
        activateSession,
        setNavigationTarget,
        openCreateSessionDialog,
        reset,
      } = useSessionStore.getState();

      // Set up some state
      activateSession('session-123');
      setNavigationTarget('target-session');
      openCreateSessionDialog();

      // Full reset
      reset();

      const state = useSessionStore.getState();
      expect(state.currentSessionId).toBeNull();
      expect(state.isReadyForNewSession).toBe(true);
      expect(state.navigationTargetSessionId).toBeNull();
      expect(state.showCreateSessionDialog).toBe(false);
    });
  });

  describe('BoardView exit flow: VIEWNV-001', () => {
    it('should reset currentSessionId to null when exiting to BoardView', () => {
      const { activateSession, prepareForNewSession } =
        useSessionStore.getState();

      // Initial state - in BoardView
      expect(useSessionStore.getState().currentSessionId).toBeNull();

      // Simulate: Navigate to a session
      activateSession('session-1');
      expect(useSessionStore.getState().currentSessionId).toBe('session-1');

      // Simulate: Exit back to BoardView
      prepareForNewSession();

      expect(useSessionStore.getState().currentSessionId).toBeNull();
      expect(useSessionStore.getState().isReadyForNewSession).toBe(true);
    });

    it('should allow navigating through multiple sessions and returning to BoardView', () => {
      const { activateSession, prepareForNewSession } =
        useSessionStore.getState();

      // Start in BoardView
      expect(useSessionStore.getState().currentSessionId).toBeNull();

      // Navigate through sessions
      activateSession('session-1');
      activateSession('session-2');
      activateSession('session-3');
      expect(useSessionStore.getState().currentSessionId).toBe('session-3');

      // Exit to BoardView
      prepareForNewSession();
      expect(useSessionStore.getState().currentSessionId).toBeNull();

      // Navigate again
      activateSession('session-4');
      expect(useSessionStore.getState().currentSessionId).toBe('session-4');
    });
  });

  describe('state machine invariant', () => {
    it('should never allow null session with isReadyForNewSession=false', () => {
      const { activateSession, prepareForNewSession, reset } =
        useSessionStore.getState();

      const checkInvariant = () => {
        const state = useSessionStore.getState();
        // Invalid state: currentSessionId=null AND isReadyForNewSession=false
        const isInvalidState =
          state.currentSessionId === null &&
          state.isReadyForNewSession === false;
        expect(isInvalidState).toBe(false);
      };

      // Initial state
      checkInvariant();

      // After activation
      activateSession('session-1');
      checkInvariant();

      // After prepare
      prepareForNewSession();
      checkInvariant();

      // After reset
      reset();
      checkInvariant();
    });

    it('should always satisfy: if currentSessionId is null, isReadyForNewSession must be true', () => {
      const { activateSession, prepareForNewSession } =
        useSessionStore.getState();

      const checkInvariant = () => {
        const state = useSessionStore.getState();
        if (state.currentSessionId === null) {
          expect(state.isReadyForNewSession).toBe(true);
        }
      };

      checkInvariant();
      activateSession('session-1');
      checkInvariant();
      prepareForNewSession();
      checkInvariant();
      activateSession('session-2');
      checkInvariant();
    });
  });
});
