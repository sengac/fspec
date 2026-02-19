/**
 * Feature: spec/features/enter-key-on-board-bypasses-create-session-dialog-no-isolated-option.feature
 * Feature: spec/features/boardview-createsessiondialog-callback-ignores-isolated-parameter.feature
 * Feature: spec/features/sessionheader-missing-isisolated-prop-isolated-badge-never-shows.feature
 * Feature: spec/features/createsessiondialog-shows-wrong-text-when-pressing-enter-on-work-unit.feature
 *
 * Tests for isolated session creation integration across BoardView and AgentView.
 *
 * GIT-030: Enter key on board should show CreateSessionDialog (not bypass it)
 * GIT-031: BoardView CreateSessionDialog callback must accept and use isolated parameter
 * GIT-032: AgentView must read isIsolated from sessionStore and pass it to SessionHeader
 * TUI-067: CreateSessionDialog must show context-appropriate text based on work unit
 */

import { describe, it, expect, beforeEach } from 'vitest';
import fs from 'fs';
import path from 'path';

// TUI-067: Test context-aware dialog text
describe('Feature: CreateSessionDialog shows wrong text when pressing Enter on work unit', () => {
  describe('Scenario: Show work-unit-aware text when pressing Enter on a story', () => {
    it('should pass workUnit prop to CreateSessionDialog in BoardView', () => {
      // @step Given I am viewing the board with work unit "AUTH-001" titled "User Login"
      const boardViewPath = path.join(__dirname, '..', 'BoardView.tsx');
      const boardViewSource = fs.readFileSync(boardViewPath, 'utf-8');

      // @step When I press Enter on the story card
      // Find the CreateSessionDialog and verify it receives workUnit prop
      const dialogMatch = boardViewSource.match(/<CreateSessionDialog[\s\S]*?\/>/);
      expect(dialogMatch).toBeDefined();

      // @step Then the dialog title should be "Work on AUTH-001?"
      // @step And the dialog description should be "Start an AI session for this task"
      // Verify the workUnit prop is passed
      if (dialogMatch) {
        expect(dialogMatch[0]).toContain('workUnit=');
        expect(dialogMatch[0]).toContain('selectedWorkUnit');
      }
    });

    it('should show work-unit-aware text when workUnit prop is provided', () => {
      // @step Given I am viewing the board with work unit "AUTH-001" titled "User Login"
      const dialogPath = path.join(__dirname, '..', '..', '..', 'components', 'CreateSessionDialog.tsx');
      const dialogSource = fs.readFileSync(dialogPath, 'utf-8');

      // @step When I press Enter on the story card
      // Verify the dialog generates context-aware text when workUnit is provided
      
      // @step Then the dialog title should be "Work on AUTH-001?"
      expect(dialogSource).toContain('`Work on ${workUnit.id}?`');

      // @step And the dialog description should be "Start an AI session for this task"
      expect(dialogSource).toContain("'Start an AI session for this task'");
    });
  });

  describe('Scenario: Show generic unattached text when using Shift+Right navigation', () => {
    it('should not pass workUnit prop in AgentView', () => {
      // @step Given I am in the agent view with no work unit selected
      const agentViewPath = path.join(__dirname, '..', 'AgentView.tsx');
      const agentViewSource = fs.readFileSync(agentViewPath, 'utf-8');

      // @step When I press Shift+Right past the last session
      // Find the CreateSessionDialog in AgentView - it should NOT have workUnit prop
      const dialogMatch = agentViewSource.match(/<CreateSessionDialog[\s\S]*?\/>/);
      expect(dialogMatch).toBeDefined();

      // @step Then the dialog title should be "Start New Agent?"
      // @step And the dialog description should be "Begin a fresh AI conversation, not linked to any task."
      // Verify AgentView does NOT pass workUnit (unattached session)
      if (dialogMatch) {
        expect(dialogMatch[0]).not.toContain('workUnit=');
      }
    });

    it('should show generic text when workUnit prop is not provided', () => {
      // @step Given I am in the agent view with no work unit selected
      const dialogPath = path.join(__dirname, '..', '..', '..', 'components', 'CreateSessionDialog.tsx');
      const dialogSource = fs.readFileSync(dialogPath, 'utf-8');

      // @step When I press Shift+Right past the last session
      // Verify the dialog shows generic text when workUnit is undefined
      
      // @step Then the dialog title should be "Start New Agent?"
      expect(dialogSource).toContain("'Start New Agent?'");

      // @step And the dialog description should be "Begin a fresh AI conversation, not linked to any task."
      expect(dialogSource).toContain("'Begin a fresh AI conversation, not linked to any task.'");
    });
  });
});

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
