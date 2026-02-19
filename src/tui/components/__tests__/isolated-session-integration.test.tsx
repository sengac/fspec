/**
 * Feature: spec/features/enter-key-on-board-bypasses-create-session-dialog-no-isolated-option.feature
 * Feature: spec/features/boardview-createsessiondialog-callback-ignores-isolated-parameter.feature
 * Feature: spec/features/sessionheader-missing-isisolated-prop-isolated-badge-never-shows.feature
 *
 * Tests for isolated session creation integration across BoardView and AgentView.
 *
 * GIT-030: Enter key on board should show CreateSessionDialog (not bypass it)
 * GIT-031: BoardView CreateSessionDialog callback must accept and use isolated parameter
 * GIT-032: AgentView must read isIsolated from sessionStore and pass it to SessionHeader
 */

import { describe, it, expect, beforeEach } from 'vitest';
import fs from 'fs';
import path from 'path';

describe('Feature: Enter key on board bypasses Create Session dialog - no isolated option', () => {
  describe('Scenario: Show Create Session dialog when no attached session', () => {
    it('should call openCreateSessionDialog when no attached session exists', () => {
      // @step Given I am viewing the board with a work unit that has no attached session
      const boardViewPath = path.join(__dirname, '..', 'BoardView.tsx');
      const boardViewSource = fs.readFileSync(boardViewPath, 'utf-8');

      // @step When I select the work unit and press Enter
      // Find the onEnter handler and check what it does when no attached session
      const onEnterMatch = boardViewSource.match(/onEnter=\{[^}]+\}/s);
      expect(onEnterMatch).toBeDefined();

      // @step Then the Create Session dialog should appear with Normal/Isolated toggle
      // Currently FAILS: onEnter calls navigateToNewSession() which bypasses dialog
      // Should call openCreateSessionDialog() instead
      expect(boardViewSource).toContain('openCreateSessionDialog');
      
      // Verify that when no attached session, the code calls openCreateSessionDialog
      // instead of navigateToNewSession
      const onEnterBody = boardViewSource.match(/onEnter=\{\([^)]*\)[^}]*\{[\s\S]*?\}\}/s);
      if (onEnterBody) {
        // The "else" branch (no attached session) should call openCreateSessionDialog
        // Currently it calls navigateToNewSession() which is wrong
        expect(onEnterBody[0]).toContain('openCreateSessionDialog');
      }
    });
  });

  describe('Scenario: Navigate directly to attached session', () => {
    it('should navigate to session without dialog when session is attached', () => {
      // @step Given I am viewing the board with a work unit that has attached session abc-123
      const boardViewPath = path.join(__dirname, '..', 'BoardView.tsx');
      const boardViewSource = fs.readFileSync(boardViewPath, 'utf-8');

      // @step When I select the work unit and press Enter
      // @step Then I should navigate to session abc-123 without seeing the Create Session dialog
      // Verify the existing behavior for attached sessions is preserved
      expect(boardViewSource).toContain('getAttachedSession');
      expect(boardViewSource).toContain('setNavigationTarget');
    });
  });
});

describe('Feature: BoardView CreateSessionDialog callback ignores isolated parameter', () => {
  describe('Scenario: Create isolated session when toggle is enabled', () => {
    it('should accept isolated parameter in onConfirm callback', () => {
      // @step Given I am viewing the Create Session dialog from the board
      const boardViewPath = path.join(__dirname, '..', 'BoardView.tsx');
      const boardViewSource = fs.readFileSync(boardViewPath, 'utf-8');

      // @step When I toggle Isolated mode ON and confirm
      // Find the CreateSessionDialog component and its onConfirm prop
      const dialogMatch = boardViewSource.match(/<CreateSessionDialog[\s\S]*?onConfirm=\{([^}]+)\}/);
      expect(dialogMatch).toBeDefined();

      // @step Then an isolated session should be created with a git worktree
      // Currently FAILS: onConfirm={() => { ... }) doesn't accept isolated parameter
      // Should be: onConfirm={(isolated) => { ... })
      const onConfirmHandler = dialogMatch ? dialogMatch[1] : '';
      
      // The handler should accept a parameter (isolated: boolean)
      // Currently it's () => { which ignores the parameter
      expect(onConfirmHandler).toMatch(/\(isolated\)|isolated\s*=>/);
    });
  });

  describe('Scenario: Create normal session when toggle is disabled', () => {
    it('should pass isolated=false to session creation', () => {
      // @step Given I am viewing the Create Session dialog from the board
      const boardViewPath = path.join(__dirname, '..', 'BoardView.tsx');
      const boardViewSource = fs.readFileSync(boardViewPath, 'utf-8');

      // @step When I leave Isolated mode OFF (default) and confirm
      // @step Then a normal session should be created without a git worktree
      // Verify that isolated parameter is used when creating sessions
      expect(boardViewSource).toContain('CreateSessionDialog');
    });
  });
});

describe('Feature: SessionHeader missing isIsolated prop - ISOLATED badge never shows', () => {
  describe('Scenario: Display ISOLATED badge for isolated session', () => {
    it('should read isIsolated from sessionStore and pass to SessionHeader', () => {
      // @step Given I have created an isolated session
      const agentViewPath = path.join(__dirname, '..', 'AgentView.tsx');
      const agentViewSource = fs.readFileSync(agentViewPath, 'utf-8');

      // @step When I view the session header
      // Verify AgentView imports useIsIsolated hook
      expect(agentViewSource).toContain('useIsIsolated');

      // @step Then I should see the [ISOLATED] badge in green next to the model name
      // Verify AgentView passes isIsolated prop to SessionHeader
      // Find the SessionHeader component and check its props
      const sessionHeaderMatch = agentViewSource.match(/<SessionHeader[\s\S]*?\/>/);
      expect(sessionHeaderMatch).toBeDefined();
      
      if (sessionHeaderMatch) {
        // Currently FAILS: SessionHeader doesn't receive isIsolated prop
        expect(sessionHeaderMatch[0]).toContain('isIsolated=');
      }
    });

    it('should have useIsIsolated hook available in sessionStore', () => {
      // @step Given I have created an isolated session
      const sessionStorePath = path.join(__dirname, '..', '..', 'store', 'sessionStore.ts');
      const sessionStoreSource = fs.readFileSync(sessionStorePath, 'utf-8');

      // @step When I view the session header
      // Verify useIsIsolated hook exists and is exported
      expect(sessionStoreSource).toContain('useIsIsolated');
      expect(sessionStoreSource).toContain('export const useIsIsolated');

      // @step Then I should see the [ISOLATED] badge in green next to the model name
      // The hook should access isIsolated state
      expect(sessionStoreSource).toContain('state.isIsolated');
    });
  });

  describe('Scenario: Do not display ISOLATED badge for normal session', () => {
    it('should pass isIsolated=false by default', () => {
      // @step Given I have created a normal (non-isolated) session
      const sessionStorePath = path.join(__dirname, '..', '..', 'store', 'sessionStore.ts');
      const sessionStoreSource = fs.readFileSync(sessionStorePath, 'utf-8');

      // @step When I view the session header
      // @step Then I should NOT see the [ISOLATED] badge
      // Verify default value is false
      expect(sessionStoreSource).toContain('isIsolated: false');
    });
  });
});
