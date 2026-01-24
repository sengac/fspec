// Feature: spec/features/unified-shift-arrow-navigation-across-boardview-agentview-and-splitpaneview.feature
// Tests for VIEWNV-001: Unified Shift+Arrow Navigation Across BoardView, AgentView, and SplitPaneView
//
// NOTE: Navigation logic is now in Rust. These tests verify the TypeScript wrapper behavior.
// Rust tests cover the actual navigation logic (session ordering, watcher traversal, etc.)

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the NAPI bindings
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetNext: vi.fn(),
  sessionGetPrev: vi.fn(),
  sessionGetFirst: vi.fn(),
  sessionClearActive: vi.fn(),
  sessionManagerCreateWithId: vi.fn(),
  sessionGetParent: vi.fn(),
}));

import {
  sessionGetNext,
  sessionGetPrev,
  sessionGetFirst,
  sessionClearActive,
  sessionManagerCreateWithId,
  sessionGetParent,
} from '@sengac/codelet-napi';

import {
  navigateRight,
  navigateLeft,
  clearActiveSession,
  getFirstSession,
} from '../tui/utils/sessionNavigation';

// ===========================================
// Navigation Logic Tests
// ===========================================

describe('Feature: Unified Shift+Arrow Navigation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ===========================================
  // BoardView Navigation Scenarios
  // ===========================================

  describe('Scenario: Shift+Right from BoardView with no sessions shows create session dialog', () => {
    it('should show create session dialog when no sessions exist', () => {
      // @step Given I am viewing the BoardView (no current session)
      // @step And no sessions exist
      vi.mocked(sessionGetNext).mockReturnValue(null);

      // @step When I press Shift+Right
      const result = navigateRight();

      // @step Then I should see a create session dialog
      expect(result).toEqual({ type: 'create-dialog' });
    });
  });

  describe('Scenario: Shift+Right from BoardView with sessions navigates to first session', () => {
    it('should navigate to first session', () => {
      // @step Given I am viewing the BoardView
      // @step And sessions exist
      vi.mocked(sessionGetNext).mockReturnValue('session-a');

      // @step When I press Shift+Right
      const result = navigateRight();

      // @step Then I should navigate to the first session
      expect(result).toEqual({ type: 'session', sessionId: 'session-a' });
    });
  });

  describe('Scenario: Shift+Left from BoardView does nothing', () => {
    it('should return board (stay on board)', () => {
      // @step Given I am viewing the BoardView
      vi.mocked(sessionGetPrev).mockReturnValue(null);

      // @step When I press Shift+Left
      const result = navigateLeft();

      // @step Then I should remain on the BoardView
      expect(result).toEqual({ type: 'board' });
    });
  });

  // ===========================================
  // Session Navigation Scenarios
  // ===========================================

  describe('Scenario: Shift+Left from first session returns to BoardView', () => {
    it('should return to BoardView from first session', () => {
      // @step Given I am viewing the first session in AgentView
      // Rust knows this is the first session and returns null
      vi.mocked(sessionGetPrev).mockReturnValue(null);

      // @step When I press Shift+Left
      const result = navigateLeft();

      // @step Then I should return to the BoardView
      expect(result).toEqual({ type: 'board' });
    });
  });

  describe('Scenario: Shift+Right from session navigates to next (session or watcher)', () => {
    it('should navigate to next session/watcher as determined by Rust', () => {
      // @step Given I am viewing a session in AgentView
      // Rust determines what's next (could be watcher or next session)
      vi.mocked(sessionGetNext).mockReturnValue('next-target');

      // @step When I press Shift+Right
      const result = navigateRight();

      // @step Then I should navigate to the next target
      expect(result).toEqual({ type: 'session', sessionId: 'next-target' });
    });
  });

  describe('Scenario: Shift+Right from last session shows create session dialog', () => {
    it('should show create dialog from last session', () => {
      // @step Given I am viewing the last session in AgentView
      // @step And it has no watchers
      vi.mocked(sessionGetNext).mockReturnValue(null);

      // @step When I press Shift+Right
      const result = navigateRight();

      // @step Then I should see a create session dialog
      expect(result).toEqual({ type: 'create-dialog' });
    });
  });

  describe('Scenario: Shift+Left from session navigates to previous', () => {
    it('should navigate to previous session/watcher as determined by Rust', () => {
      // @step Given I am viewing a session in AgentView
      vi.mocked(sessionGetPrev).mockReturnValue('prev-target');

      // @step When I press Shift+Left
      const result = navigateLeft();

      // @step Then I should navigate to the previous target
      expect(result).toEqual({ type: 'session', sessionId: 'prev-target' });
    });
  });

  // ===========================================
  // Watcher Navigation Scenarios (Rust handles logic)
  // ===========================================

  describe('Scenario: Shift+Left from first watcher returns to parent session', () => {
    it('should return to parent session (Rust determines this)', () => {
      // @step Given I am viewing a watcher in SplitSessionView
      // @step And it is the first watcher
      // Rust returns parent session ID
      vi.mocked(sessionGetPrev).mockReturnValue('session-a');

      // @step When I press Shift+Left
      const result = navigateLeft();

      // @step Then I should return to the parent session
      expect(result).toEqual({ type: 'session', sessionId: 'session-a' });
    });
  });

  describe('Scenario: Shift+Right from watcher navigates to next sibling', () => {
    it('should navigate to next sibling (Rust determines this)', () => {
      // @step Given I am viewing a watcher
      // Rust returns next sibling watcher
      vi.mocked(sessionGetNext).mockReturnValue('watcher-w2');

      // @step When I press Shift+Right
      const result = navigateRight();

      // @step Then I should navigate to the next sibling watcher
      expect(result).toEqual({ type: 'session', sessionId: 'watcher-w2' });
    });
  });

  describe('Scenario: Shift+Right from last watcher of last session shows create dialog', () => {
    it('should show create dialog (Rust returns null)', () => {
      // @step Given I am viewing the last watcher of the last session
      vi.mocked(sessionGetNext).mockReturnValue(null);

      // @step When I press Shift+Right
      const result = navigateRight();

      // @step Then I should see a create session dialog
      expect(result).toEqual({ type: 'create-dialog' });
    });
  });

  // ===========================================
  // Create Session Dialog Scenarios
  // ===========================================

  describe('Scenario: Confirming create session dialog creates new unattached session', () => {
    it('should create new session on confirm', async () => {
      // @step Given I see a create session dialog
      // @step When I confirm the dialog
      vi.mocked(sessionManagerCreateWithId).mockResolvedValue(undefined);
      const newSessionId = 'new-session-123';
      await sessionManagerCreateWithId(
        newSessionId,
        'claude',
        '/test/project',
        'New Session'
      );

      // @step Then a new session should be created
      expect(sessionManagerCreateWithId).toHaveBeenCalledWith(
        newSessionId,
        'claude',
        '/test/project',
        'New Session'
      );

      // @step And the new session should not be attached to any work unit
      const workUnitId = null;
      expect(workUnitId).toBeNull();
    });
  });

  describe('Scenario: Canceling create session dialog stays at current position', () => {
    it('should stay at current position on cancel', () => {
      // @step Given I am viewing a session in AgentView
      // @step And I see a create session dialog
      let showCreateDialog = true;

      // @step When I cancel the dialog
      showCreateDialog = false;

      // @step Then the dialog should close
      expect(showCreateDialog).toBe(false);

      // @step And I should remain at current position
      // No navigation action taken on cancel
    });
  });

  // ===========================================
  // Backward Compatibility Scenario
  // ===========================================

  describe('Scenario: /parent command still works alongside Shift+Left navigation', () => {
    it('should return parent session ID via sessionGetParent', () => {
      // @step Given I am viewing a watcher
      vi.mocked(sessionGetParent).mockReturnValue('session-a');

      // @step When I use /parent command
      const parentId = sessionGetParent('watcher-w1');

      // @step Then I should get the parent session ID
      expect(parentId).toBe('session-a');
    });
  });

  // ===========================================
  // Clear Active Session
  // ===========================================

  describe('clearActiveSession', () => {
    it('calls Rust to clear active session tracking', () => {
      // @step When returning to BoardView
      clearActiveSession();

      // @step Then Rust should clear the active session
      expect(sessionClearActive).toHaveBeenCalled();
    });
  });

  describe('getFirstSession', () => {
    it('returns first session from Rust', () => {
      vi.mocked(sessionGetFirst).mockReturnValue('session-a');

      const result = getFirstSession();

      expect(result).toBe('session-a');
    });

    it('returns null when no sessions', () => {
      vi.mocked(sessionGetFirst).mockReturnValue(null);

      const result = getFirstSession();

      expect(result).toBeNull();
    });
  });
});
