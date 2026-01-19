/**
 * Feature: spec/features/session-attachment-for-work-units-with-tui-resume-integration.feature
 *
 * Tests for session attachment to work units in TUI (SESS-001)
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFspecStore } from '../store/fspecStore';

describe('Feature: Session attachment for work units with TUI resume integration', () => {
  beforeEach(() => {
    // Reset store state before each test using Zustand's setState
    useFspecStore.setState({
      workUnits: [],
      isLoaded: false,
      error: null,
      sessionAttachments: new Map(),
      currentWorkUnitId: null,
    });
  });

  describe('Scenario: Session auto-attaches to work unit on first message', () => {
    it('should auto-attach session when first message is sent', () => {
      // @step Given I am viewing the board with work unit "AUTH-001" selected
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];

      // @step And "AUTH-001" has no attached session
      expect(store.getAttachedSession?.('AUTH-001')).toBeUndefined();

      // @step When I press Enter to open the agent view
      // Simulated by setting currentWorkUnitId
      store.setCurrentWorkUnitId?.('AUTH-001');

      // @step And I send my first message
      // This triggers session creation - simulated by calling attachSession
      const mockSessionId = 'session-123';
      store.attachSession?.('AUTH-001', mockSessionId);

      // @step Then a new session should be created
      // @step And that session should be automatically attached to "AUTH-001"
      expect(store.getAttachedSession?.('AUTH-001')).toBe(mockSessionId);
    });
  });

  describe('Scenario: Work unit with attached session displays green indicator', () => {
    it('should show green indicator for work unit with attached session', () => {
      // @step Given work unit "AUTH-001" has an attached session
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];
      store.attachSession?.('AUTH-001', 'session-123');

      // @step When I view the board
      const hasSession = store.hasAttachedSession?.('AUTH-001');

      // @step Then "AUTH-001" should display with a 🟢 indicator on the left side
      // @step And the display should show "🟢 AUTH-001"
      expect(hasSession).toBe(true);
    });
  });

  describe('Scenario: Work unit without attached session displays normally', () => {
    it('should not show indicator for work unit without attached session', () => {
      // @step Given work unit "AUTH-001" has no attached session
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];

      // @step When I view the board
      const hasSession = store.hasAttachedSession?.('AUTH-001');

      // @step Then "AUTH-001" should display without any session indicator
      expect(hasSession).toBe(false);
    });
  });

  describe('Scenario: Pressing Enter on work unit with attached session resumes conversation', () => {
    it('should resume session when entering work unit with attached session', () => {
      // @step Given work unit "AUTH-001" has an attached session with previous messages
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];
      const sessionId = 'session-with-messages';
      store.attachSession?.('AUTH-001', sessionId);

      // @step When I press Enter on "AUTH-001"
      store.setCurrentWorkUnitId?.('AUTH-001');
      const attachedSession = store.getAttachedSession?.('AUTH-001');

      // @step Then the agent view should open
      // @step And the previous conversation messages should be restored
      // @step And I can continue the conversation from where I left off
      expect(attachedSession).toBe(sessionId);
    });
  });

  describe('Scenario: Detach command removes session attachment and clears conversation', () => {
    it('should detach session and clear conversation on /detach', () => {
      // @step Given I am in the agent view for work unit "AUTH-001"
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];
      store.setCurrentWorkUnitId?.('AUTH-001');

      // @step And "AUTH-001" has an attached session with messages
      store.attachSession?.('AUTH-001', 'session-123');
      expect(store.getAttachedSession?.('AUTH-001')).toBe('session-123');

      // @step When I type "/detach"
      store.detachSession?.('AUTH-001');

      // @step Then the session should be detached from "AUTH-001"
      expect(store.getAttachedSession?.('AUTH-001')).toBeUndefined();

      // @step And the conversation should be cleared
      // @step And I should have a fresh empty session ready
      // (conversation clearing is handled by AgentView, tested separately)
    });
  });

  describe('Scenario: Resume command attaches selected session to current work unit', () => {
    it('should attach selected session to current work unit on /resume', () => {
      // @step Given I am in the agent view for work unit "AUTH-001"
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];
      store.setCurrentWorkUnitId?.('AUTH-001');

      // @step And "AUTH-001" has no attached session
      expect(store.getAttachedSession?.('AUTH-001')).toBeUndefined();

      // @step And there are existing sessions available
      const existingSessionId = 'existing-session-456';

      // @step When I type "/resume"
      // @step And I select a session from the list
      store.attachSession?.('AUTH-001', existingSessionId);

      // @step Then the selected session should be attached to "AUTH-001"
      // @step And the session messages should be restored
      expect(store.getAttachedSession?.('AUTH-001')).toBe(existingSessionId);
    });
  });

  describe('Scenario: Resume command replaces existing session attachment', () => {
    it('should replace existing session attachment when resuming different session', () => {
      // @step Given I am in the agent view for work unit "AUTH-001"
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];
      store.setCurrentWorkUnitId?.('AUTH-001');

      // @step And "AUTH-001" has "session-A" attached
      store.attachSession?.('AUTH-001', 'session-A');
      expect(store.getAttachedSession?.('AUTH-001')).toBe('session-A');

      // @step And there are other sessions available including "session-B"
      const sessionB = 'session-B';

      // @step When I type "/resume"
      // @step And I select "session-B" from the list
      store.attachSession?.('AUTH-001', sessionB);

      // @step Then "session-B" should replace "session-A" as the attachment for "AUTH-001"
      // @step And "session-B" messages should be restored
      expect(store.getAttachedSession?.('AUTH-001')).toBe(sessionB);
    });
  });

  describe('Scenario: Shift+ESC returns to board while keeping session attached', () => {
    it('should keep session attached when pressing Shift+ESC', () => {
      // @step Given I am in the agent view for work unit "AUTH-001"
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];
      store.setCurrentWorkUnitId?.('AUTH-001');

      // @step And "AUTH-001" has an attached session
      store.attachSession?.('AUTH-001', 'session-123');

      // @step When I press Shift+ESC
      // Shift+ESC returns to board but does NOT call detachSession
      store.setCurrentWorkUnitId?.(null); // Clear current work unit (return to board)

      // @step Then I should return to the board view
      expect(store.getCurrentWorkUnitId?.()).toBeNull();

      // @step And "AUTH-001" should still show the 🟢 indicator
      // @step And the session should continue running in the background
      expect(store.getAttachedSession?.('AUTH-001')).toBe('session-123');
    });
  });

  describe('Scenario: Session attachments are cleared when app closes', () => {
    it('should clear all session attachments when store is reset', () => {
      // @step Given work unit "AUTH-001" has an attached session
      const store = useFspecStore.getState();
      store.workUnits = [
        {
          id: 'AUTH-001',
          title: 'Test 1',
          status: 'specifying',
          type: 'story',
        },
        { id: 'UI-002', title: 'Test 2', status: 'specifying', type: 'story' },
      ];
      store.attachSession?.('AUTH-001', 'session-1');

      // @step And work unit "UI-002" has an attached session
      store.attachSession?.('UI-002', 'session-2');

      expect(store.getAttachedSession?.('AUTH-001')).toBe('session-1');
      expect(store.getAttachedSession?.('UI-002')).toBe('session-2');

      // @step When I close and reopen the TUI application
      // Simulated by calling clearAllSessionAttachments or resetting store
      store.clearAllSessionAttachments?.();

      // @step Then "AUTH-001" should have no attached session
      expect(store.getAttachedSession?.('AUTH-001')).toBeUndefined();

      // @step And "UI-002" should have no attached session
      expect(store.getAttachedSession?.('UI-002')).toBeUndefined();

      // @step And no work units should display the 🟢 indicator
      expect(store.hasAttachedSession?.('AUTH-001')).toBe(false);
      expect(store.hasAttachedSession?.('UI-002')).toBe(false);
    });
  });

  // Additional integration tests for SESS-001

  describe('Close Session clears attachment', () => {
    it('should clear session attachment when session is destroyed', () => {
      // @step Given I am in the agent view for work unit "AUTH-001"
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];
      store.setCurrentWorkUnitId?.('AUTH-001');

      // @step And "AUTH-001" has an attached session
      store.attachSession?.('AUTH-001', 'session-to-destroy');
      expect(store.hasAttachedSession?.('AUTH-001')).toBe(true);

      // @step When I select "Close Session" from the exit dialog
      // This destroys the session AND clears the attachment
      store.detachSession?.('AUTH-001');

      // @step Then the session attachment should be cleared
      expect(store.getAttachedSession?.('AUTH-001')).toBeUndefined();
      expect(store.hasAttachedSession?.('AUTH-001')).toBe(false);
    });
  });

  describe('Multiple work units with independent sessions', () => {
    it('should maintain separate session attachments for different work units', () => {
      // @step Given multiple work units exist
      const store = useFspecStore.getState();
      store.workUnits = [
        {
          id: 'AUTH-001',
          title: 'Auth',
          status: 'implementing',
          type: 'story',
        },
        { id: 'UI-002', title: 'UI', status: 'implementing', type: 'story' },
        { id: 'API-003', title: 'API', status: 'implementing', type: 'story' },
      ];

      // @step When I attach different sessions to each work unit
      store.attachSession?.('AUTH-001', 'auth-session');
      store.attachSession?.('UI-002', 'ui-session');
      // API-003 intentionally has no session

      // @step Then each work unit should have its own session
      expect(store.getAttachedSession?.('AUTH-001')).toBe('auth-session');
      expect(store.getAttachedSession?.('UI-002')).toBe('ui-session');
      expect(store.getAttachedSession?.('API-003')).toBeUndefined();

      // @step And the indicators should reflect the attachments
      expect(store.hasAttachedSession?.('AUTH-001')).toBe(true);
      expect(store.hasAttachedSession?.('UI-002')).toBe(true);
      expect(store.hasAttachedSession?.('API-003')).toBe(false);

      // @step When I detach one session
      store.detachSession?.('AUTH-001');

      // @step Then only that work unit's attachment should be affected
      expect(store.getAttachedSession?.('AUTH-001')).toBeUndefined();
      expect(store.getAttachedSession?.('UI-002')).toBe('ui-session');
    });
  });

  describe('Session attachment persists across navigation', () => {
    it('should maintain attachment when navigating between work units', () => {
      // @step Given work unit "AUTH-001" has an attached session
      const store = useFspecStore.getState();
      store.workUnits = [
        {
          id: 'AUTH-001',
          title: 'Auth',
          status: 'implementing',
          type: 'story',
        },
        { id: 'UI-002', title: 'UI', status: 'implementing', type: 'story' },
      ];
      store.attachSession?.('AUTH-001', 'persistent-session');
      store.setCurrentWorkUnitId?.('AUTH-001');

      // @step When I navigate to a different work unit
      store.setCurrentWorkUnitId?.('UI-002');

      // @step Then "AUTH-001" should still have its session attached
      expect(store.getAttachedSession?.('AUTH-001')).toBe('persistent-session');
      expect(store.hasAttachedSession?.('AUTH-001')).toBe(true);

      // @step And the current work unit should be "UI-002"
      expect(store.getCurrentWorkUnitId?.()).toBe('UI-002');

      // @step When I return to the board view
      store.setCurrentWorkUnitId?.(null);

      // @step Then the attachment should still persist
      expect(store.getAttachedSession?.('AUTH-001')).toBe('persistent-session');
    });
  });

  describe('Safe handling of edge cases', () => {
    it('should safely handle detaching non-existent session', () => {
      // @step Given work unit "AUTH-001" has no attached session
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];

      // @step When I try to detach a session from it
      // @step Then it should not throw an error
      expect(() => store.detachSession?.('AUTH-001')).not.toThrow();
      expect(store.hasAttachedSession?.('AUTH-001')).toBe(false);
    });

    it('should return undefined for non-existent work unit', () => {
      // @step Given work unit "NONEXISTENT-001" does not exist
      const store = useFspecStore.getState();

      // @step When I query its session attachment
      const result = store.getAttachedSession?.('NONEXISTENT-001');

      // @step Then it should return undefined
      expect(result).toBeUndefined();
      expect(store.hasAttachedSession?.('NONEXISTENT-001')).toBe(false);
    });

    it('should handle attaching session to non-existent work unit', () => {
      // @step Given work unit "NONEXISTENT-001" does not exist in workUnits array
      const store = useFspecStore.getState();
      store.workUnits = [];

      // @step When I attach a session to it (store allows this for flexibility)
      store.attachSession?.('NONEXISTENT-001', 'orphan-session');

      // @step Then the attachment should be stored
      // (useful for cases where work unit may be loaded later)
      expect(store.getAttachedSession?.('NONEXISTENT-001')).toBe(
        'orphan-session'
      );
    });
  });

  describe('Current work unit tracking', () => {
    it('should track current work unit when entering agent view', () => {
      // @step Given I am on the board view
      const store = useFspecStore.getState();
      store.workUnits = [
        { id: 'AUTH-001', title: 'Test', status: 'specifying', type: 'story' },
      ];
      expect(store.getCurrentWorkUnitId?.()).toBeNull();

      // @step When I enter the agent view for "AUTH-001"
      store.setCurrentWorkUnitId?.('AUTH-001');

      // @step Then the current work unit should be "AUTH-001"
      expect(store.getCurrentWorkUnitId?.()).toBe('AUTH-001');
    });

    it('should clear current work unit when returning to board', () => {
      // @step Given I am in the agent view for "AUTH-001"
      const store = useFspecStore.getState();
      store.setCurrentWorkUnitId?.('AUTH-001');
      expect(store.getCurrentWorkUnitId?.()).toBe('AUTH-001');

      // @step When I return to the board view
      store.setCurrentWorkUnitId?.(null);

      // @step Then the current work unit should be null
      expect(store.getCurrentWorkUnitId?.()).toBeNull();
    });
  });
});
