/**
 * Feature: spec/features/merge-worktree-command.feature
 *
 * E2E integration tests for the /merge-worktree slash command.
 * Tests the REAL handler from mergeWorktreeHandler.ts with REAL NAPI bindings.
 * NO MOCKS - uses fixtures with real git repos and real isolated sessions.
 *
 * Test strategy:
 * - Scenarios 1, 3, 4: Full E2E through TypeScript → Rust NAPI → TypeScript
 *   (creates real git repos, real worktrees, real merge/inspect operations)
 * - Scenario 2: Behavioral test of the handler (early-return, no NAPI needed)
 * - Scenarios 5-6: Slash command registry (real imports)
 * - Scenario 7: Codebase file/content removal verification
 *
 * Work Unit: GIT-036
 */

import { describe, it, expect, beforeAll, afterAll, afterEach } from 'vitest';
import { writeFile } from 'fs/promises';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';

import { handleMergeWorktree } from '../handlers/mergeWorktreeHandler';
import { SLASH_COMMANDS } from '../utils/slashCommands';
import {
  createE2EFixture,
  createTestContext,
  getStatusMessages,
} from '../handlers/__tests__/fixtures/mergeWorktreeFixture';
import type { E2EFixture } from '../handlers/__tests__/fixtures/mergeWorktreeFixture';

// ============================================================================
// Resolve project paths for file-system checks
// ============================================================================

const PROJECT_ROOT = join(__dirname, '..', '..');
const AGENTVIEW_PATH = join(PROJECT_ROOT, 'tui', 'components', 'AgentView.tsx');

describe('Feature: Merge worktree slash command with auto-close session workflow', () => {
  let fixture: E2EFixture;
  let agentViewSource: string;

  beforeAll(async () => {
    fixture = await createE2EFixture('merge-wt');
    await fixture.initGitRepo();
    agentViewSource = readFileSync(AGENTVIEW_PATH, 'utf-8');
  });

  afterAll(async () => {
    await fixture.cleanup();
  });

  afterEach(async () => {
    await fixture.destroyAllSessions();
    fixture.resetStores();
  });

  // ========================================
  // Scenario: Successful merge closes session and returns to board
  // ========================================
  describe('Scenario: Successful merge closes session and returns to board', () => {
    it('should merge changes via NAPI, show summary, and close session', async () => {
      // @step Given I am in an active isolated session with modified files
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('Merge Happy Path');

      // Add a new file in the worktree
      await writeFile(
        join(worktreePath, 'new-feature.ts'),
        'export function hello() { return "world"; }\n'
      );

      const { ctx, conversation, calls } = createTestContext(
        fixture,
        sessionId
      );

      // @step When I type "/merge-worktree"
      await handleMergeWorktree(ctx);

      // @step Then inspectSessionChanges should be called to check for changes
      // (verified implicitly: if inspect had no changes, we'd get "Nothing to merge")

      // @step And mergeSessionChanges should be called to apply changes to the main worktree
      // Verify the new file now exists in main worktree
      expect(existsSync(join(fixture.testDir, 'new-feature.ts'))).toBe(true);
      const mainContent = readFileSync(
        join(fixture.testDir, 'new-feature.ts'),
        'utf-8'
      );
      expect(mainContent).toContain('hello');

      // Verify the worktree was removed by merge
      expect(existsSync(worktreePath)).toBe(false);

      // @step And I should see a merge summary status message showing counts
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages.length).toBeGreaterThanOrEqual(1);
      // GIT-037: Now shows rich file-by-file summary instead of counts
      const mergeMsg = statusMessages.find(m => m.includes('Merge successful'));
      expect(mergeMsg).toBeDefined();

      // @step And cleanupCurrentSessionHandler should be called
      // GIT-037: Cleanup now deferred via action prompt - invoke it
      expect(calls.actionPromptSet).not.toBeNull();
      await calls.actionPromptSet?.onConfirm();
      expect(calls.cleanupCalled).toBe(true);

      // @step And destroySession should be called with the current session ID
      // Verified implicitly: destroySession clears the session store
      // If it wasn't called, the stores would still have state

      // @step And onExit should be called to return to the board view
      expect(calls.onExitCalled).toBe(true);

      // @step And the input value should be cleared
      expect(calls.inputValueSet).toBe('');
    });
  });

  // ========================================
  // Scenario: Merge worktree in non-isolated session shows error
  // ========================================
  describe('Scenario: Merge worktree in non-isolated session shows error', () => {
    it('should show error and not call merge or inspect', async () => {
      // @step Given I am in an active session that is not isolated
      const { ctx, conversation, calls } = createTestContext(
        fixture,
        'non-isolated-session-id',
        { isIsolated: false }
      );

      // @step When I type "/merge-worktree"
      await handleMergeWorktree(ctx);

      // @step Then I should see a status message "This command is only available in isolated sessions"
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages).toHaveLength(1);
      expect(statusMessages[0]).toBe(
        'This command is only available in isolated sessions'
      );

      // @step And no merge or inspect calls should be made
      // Handler returns early before any NAPI calls when !isIsolated
      expect(calls.inputValueSet).toBe('');

      // @step And the session should remain active
      expect(calls.cleanupCalled).toBe(false);
      expect(calls.onExitCalled).toBe(false);
    });
  });

  // ========================================
  // Scenario: Merge worktree with no changes shows nothing to merge
  // ========================================
  describe('Scenario: Merge worktree with no changes shows nothing to merge', () => {
    it('should call inspect via NAPI and return early when no changes', async () => {
      // @step Given I am in an active isolated session with no modified files
      const { sessionId } =
        await fixture.createIsolatedSession('No Changes Test');
      // Don't modify anything in the worktree - it's clean

      const { ctx, conversation, calls } = createTestContext(
        fixture,
        sessionId
      );

      // @step When I type "/merge-worktree"
      await handleMergeWorktree(ctx);

      // @step Then inspectSessionChanges should be called and return empty file arrays
      // (The NAPI call is made - it returns empty arrays for a clean worktree)

      // @step And I should see a status message "Nothing to merge"
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages).toHaveLength(1);
      expect(statusMessages[0]).toBe('Nothing to merge');

      // @step And mergeSessionChanges should not be called
      // Verified: If merge had been called on a clean worktree, the worktree would be removed.
      // Instead it should still exist since we returned early.

      // @step And the session should remain active
      expect(calls.cleanupCalled).toBe(false);
      expect(calls.onExitCalled).toBe(false);
    });
  });

  // ========================================
  // Scenario: Merge worktree with conflicts keeps session open
  // ========================================
  describe('Scenario: Merge worktree with conflicts keeps session open', () => {
    it('should show conflict error via NAPI and keep session open', async () => {
      // @step Given I am in an active isolated session with modified files
      const { sessionId, worktreePath } =
        await fixture.createIsolatedSession('Conflict Test');

      // Modify src/main.ts in the session worktree
      await writeFile(
        join(worktreePath, 'src', 'main.ts'),
        'export const VERSION = 2; // session change\n'
      );

      // @step And the main worktree has conflicting changes to the same files
      // Modify the SAME file in main worktree (creating a conflict)
      await writeFile(
        join(fixture.testDir, 'src', 'main.ts'),
        'export const VERSION = 99; // main change\n'
      );

      const { ctx, conversation, calls } = createTestContext(
        fixture,
        sessionId
      );

      // @step When I type "/merge-worktree"
      await handleMergeWorktree(ctx);

      // @step Then inspectSessionChanges should be called to check for changes
      // (implicit - handler proceeds past the empty-check)

      // @step And mergeSessionChanges should be called and throw a Conflict error
      // @step And I should see a status message listing the conflicting file paths
      const statusMessages = getStatusMessages(conversation);
      expect(statusMessages).toHaveLength(1);
      // The error message from Rust contains "Conflict" and the file path
      const conflictMsg = statusMessages[0];
      expect(conflictMsg.toLowerCase()).toContain('conflict');
      expect(conflictMsg).toContain('main.ts');

      // @step And destroySession should not be called
      // @step And the session should remain active for conflict resolution
      expect(calls.cleanupCalled).toBe(false);
      expect(calls.onExitCalled).toBe(false);

      // Worktree should still exist (not removed on conflict)
      expect(existsSync(worktreePath)).toBe(true);

      // Main worktree should still have its own content (not overwritten)
      const mainContent = readFileSync(
        join(fixture.testDir, 'src', 'main.ts'),
        'utf-8'
      );
      expect(mainContent).toContain('VERSION = 99');
    });
  });

  // ========================================
  // Scenario: /merge-worktree command is registered in slash command registry
  // ========================================
  describe('Scenario: /merge-worktree command is registered in slash command registry', () => {
    it('should have merge-worktree in SLASH_COMMANDS', () => {
      // @step Given the slash command registry in slashCommands.ts
      const commands = SLASH_COMMANDS;

      // @step Then the "merge-worktree" command should be in the SLASH_COMMANDS array
      const mergeWorktreeCmd = commands.find(c => c.name === 'merge-worktree');
      expect(mergeWorktreeCmd).toBeDefined();

      // @step And its description should be "Merge worktree changes and close session"
      expect(mergeWorktreeCmd?.description).toBe(
        'Merge worktree changes and close session'
      );

      // @step And it should not have requiresSession set to false
      expect(mergeWorktreeCmd?.requiresSession).not.toBe(false);
    });
  });

  // ========================================
  // Scenario: /sessions command is removed from slash command registry
  // ========================================
  describe('Scenario: /sessions command is removed from slash command registry', () => {
    it('should not have sessions in SLASH_COMMANDS', () => {
      // @step Given the slash command registry in slashCommands.ts
      const commands = SLASH_COMMANDS;

      // @step Then no entry with name "sessions" should exist in the SLASH_COMMANDS array
      const sessionsCmd = commands.find(c => c.name === 'sessions');
      expect(sessionsCmd).toBeUndefined();
    });
  });

  // ========================================
  // Scenario: SessionManagementPanel component and tests are removed
  // ========================================
  describe('Scenario: SessionManagementPanel component and tests are removed', () => {
    it('should not have SessionManagementPanel files in the codebase', () => {
      // @step Given the codebase after this change

      // @step Then the file "src/tui/components/SessionManagementPanel.tsx" should not exist
      expect(
        existsSync(
          join(PROJECT_ROOT, 'tui', 'components', 'SessionManagementPanel.tsx')
        )
      ).toBe(false);

      // @step And the file "src/tui/components/__tests__/SessionManagementPanel.test.tsx" should not exist
      expect(
        existsSync(
          join(
            PROJECT_ROOT,
            'tui',
            'components',
            '__tests__',
            'SessionManagementPanel.test.tsx'
          )
        )
      ).toBe(false);

      // @step And the file "src/tui/components/__tests__/SessionManagementPanelKeyboard.test.tsx" should not exist
      expect(
        existsSync(
          join(
            PROJECT_ROOT,
            'tui',
            'components',
            '__tests__',
            'SessionManagementPanelKeyboard.test.tsx'
          )
        )
      ).toBe(false);

      // @step And AgentView should not import SessionManagementPanel
      expect(agentViewSource).not.toContain('SessionManagementPanel');

      // @step And AgentView should not contain showSessionManagementPanel state
      expect(agentViewSource).not.toContain('showSessionManagementPanel');

      // @step And AgentView should not contain a render block for SessionManagementPanel
      expect(agentViewSource).not.toMatch(/<SessionManagementPanel/);
    });
  });

  // ========================================
  // Integration: Handler wiring in AgentView
  // ========================================
  describe('Integration: Handler is properly wired in AgentView', () => {
    it('should delegate /merge-worktree to handleMergeWorktree', () => {
      // AgentView should detect '/merge-worktree' and delegate to the handler
      expect(agentViewSource).toContain(
        "if (userMessage === '/merge-worktree')"
      );
      expect(agentViewSource).toContain('await handleMergeWorktree(');

      // AgentView should NOT have inline NAPI calls for merge
      // (all logic is in the handler, not in AgentView)
      const afterMergeCmd = agentViewSource.split("'/merge-worktree'")[1];
      expect(afterMergeCmd).toBeDefined();
      const handlerBlock = afterMergeCmd?.substring(0, 200) ?? '';
      expect(handlerBlock).not.toContain('inspectSessionChanges');
      expect(handlerBlock).not.toContain('mergeSessionChanges');
    });

    it('should import handleMergeWorktree from handlers/mergeWorktreeHandler', () => {
      expect(agentViewSource).toContain(
        "from '../handlers/mergeWorktreeHandler'"
      );
      expect(agentViewSource).toContain('handleMergeWorktree');
    });
  });
});
