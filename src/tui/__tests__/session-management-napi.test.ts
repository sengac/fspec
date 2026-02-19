/**
 * Feature: spec/features/isolated-session-napi-bindings.feature
 *
 * Tests for the session management NAPI bindings.
 * These tests call the actual NAPI bindings for session listing,
 * inspection, merging, discarding, and pruning.
 *
 * GIT-029: TUI integration for isolated sessions
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { execSync } from 'child_process';
import { randomUUID } from 'crypto';

// Import the NAPI bindings we need to test
import {
  sessionManagerCreateIsolated,
  sessionManagerCreateWithId,
  sessionManagerDestroy,
  sessionManagerList,
  listSessions,
  inspectSession,
  mergeSession,
  discardSession,
  pruneOrphaned,
  listWorktrees,
  removeWorktree,
  persistenceSetDataDirectory,
} from '@sengac/codelet-napi';

// Helper to get manifest path for a session
function getManifestPath(sessionId: string): string {
  return path.join(os.homedir(), '.fspec', 'git-sessions', `${sessionId}.json`);
}

// Helper to delete a session manifest (makes it orphaned)
function deleteManifest(sessionId: string): void {
  const manifestPath = getManifestPath(sessionId);
  if (fs.existsSync(manifestPath)) {
    fs.unlinkSync(manifestPath);
  }
}

describe('Feature: TUI integration for isolated sessions - Part C: NAPI Binding Tests', () => {
  let testDir: string;
  let dataDir: string;

  beforeEach(async () => {
    // Create a temporary git repository for testing
    testDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'fspec-session-mgmt-test-')
    );

    // Create a temporary data directory for persistence
    dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'fspec-data-'));
    persistenceSetDataDirectory(dataDir);

    // Initialize git repo
    execSync('git init', { cwd: testDir, stdio: 'pipe' });
    execSync('git config user.email "test@test.com"', {
      cwd: testDir,
      stdio: 'pipe',
    });
    execSync('git config user.name "Test User"', {
      cwd: testDir,
      stdio: 'pipe',
    });

    // Create initial commit so HEAD exists
    fs.writeFileSync(path.join(testDir, 'README.md'), '# Test Project');
    fs.writeFileSync(path.join(testDir, 'existing.txt'), 'existing content');
    execSync('git add .', { cwd: testDir, stdio: 'pipe' });
    execSync('git commit -m "Initial commit"', { cwd: testDir, stdio: 'pipe' });
  });

  afterEach(async () => {
    // Cleanup: remove all worktrees
    try {
      const worktrees = listWorktrees(testDir);
      for (const wt of worktrees) {
        try {
          removeWorktree(testDir, wt.sessionId);
        } catch {
          // Worktree may not exist
        }
      }
    } catch {
      // Repository may not exist
    }

    // Cleanup: remove test directory
    try {
      fs.rmSync(testDir, { recursive: true, force: true });
    } catch {
      // Directory may not exist
    }

    // Cleanup: remove data directory
    try {
      fs.rmSync(dataDir, { recursive: true, force: true });
    } catch {
      // Directory may not exist
    }
  });

  // Helper to create an isolated session with files
  async function createSessionWithFiles(
    sessionId: string,
    files: Record<string, string | null>
  ): Promise<string> {
    const result = await sessionManagerCreateIsolated(
      sessionId,
      'anthropic/claude-sonnet-4-20250514',
      testDir,
      'Test Session'
    );

    // Write/delete files in the worktree
    for (const [filename, content] of Object.entries(files)) {
      const filePath = path.join(result.worktreePath, filename);
      if (content === null) {
        // Delete file
        if (fs.existsSync(filePath)) {
          fs.unlinkSync(filePath);
        }
      } else {
        // Create/modify file
        const dir = path.dirname(filePath);
        if (!fs.existsSync(dir)) {
          fs.mkdirSync(dir, { recursive: true });
        }
        fs.writeFileSync(filePath, content);
      }
    }

    return result.worktreePath;
  }

  // Helper to destroy session (cleanup)
  function destroySession(sessionId: string): void {
    try {
      sessionManagerDestroy(sessionId);
    } catch {
      // Session may not exist
    }
  }

  describe('Scenario: listSessions returns sessions with derived status', () => {
    it('should return sessions with correct derived statuses', async () => {
      // @step Given a git repository at the project root
      // testDir is our git repository

      // @step And there are session worktrees in ".fspec/worktrees/"
      const activeSessionId = randomUUID();
      const pendingSessionId = randomUUID();
      const cleanSessionId = randomUUID();

      // Create an active session (will be in active sessions list)
      await createSessionWithFiles(activeSessionId, {
        'active-file.txt': 'active content',
      });

      // Create a session with pending changes (not in active list)
      await createSessionWithFiles(pendingSessionId, {
        'pending-file.txt': 'pending content',
      });
      destroySession(pendingSessionId); // Remove from background sessions

      // Create a clean session (no changes, not in active list)
      await sessionManagerCreateIsolated(
        cleanSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Clean Session'
      );
      destroySession(cleanSessionId); // Remove from background sessions

      // @step When I call listSessions with active session IDs
      const sessions = listSessions(testDir, [activeSessionId], undefined);

      // @step Then sessions with worktrees should be returned
      expect(sessions.length).toBeGreaterThanOrEqual(3);

      // @step And each session should have a derived status
      const activeSession = sessions.find(s => s.sessionId === activeSessionId);
      const pendingSession = sessions.find(
        s => s.sessionId === pendingSessionId
      );
      const cleanSession = sessions.find(s => s.sessionId === cleanSessionId);

      // @step And active sessions should have status "active"
      expect(activeSession).toBeDefined();
      expect(activeSession?.status).toBe('active');

      // @step And sessions with changes but not active should have status "pending_merge"
      // Note: status returned is "pendingmerge" (lowercased enum name without underscore)
      expect(pendingSession).toBeDefined();
      expect(pendingSession?.status).toBe('pendingmerge');
      expect(pendingSession?.filesChanged).toBeGreaterThan(0);

      // @step And sessions without changes and not active should have status "clean"
      expect(cleanSession).toBeDefined();
      expect(cleanSession?.status).toBe('clean');
      expect(cleanSession?.filesChanged).toBe(0);

      // Cleanup
      destroySession(activeSessionId);
    });

    it('should filter sessions by status', async () => {
      // Create sessions with different statuses
      const activeSessionId = randomUUID();
      const pendingSessionId = randomUUID();

      await createSessionWithFiles(activeSessionId, {});
      await createSessionWithFiles(pendingSessionId, {
        'new-file.txt': 'content',
      });
      destroySession(pendingSessionId);

      // Filter by active
      const activeSessions = listSessions(testDir, [activeSessionId], 'active');
      expect(activeSessions.every(s => s.status === 'active')).toBe(true);

      // Filter by pending_merge
      // Note: Filter uses "pending_merge" but returned status is "pendingmerge"
      const pendingSessions = listSessions(
        testDir,
        [activeSessionId],
        'pending_merge'
      );
      expect(pendingSessions.every(s => s.status === 'pendingmerge')).toBe(
        true
      );

      // Cleanup
      destroySession(activeSessionId);
    });
  });

  describe('Scenario: inspectSession returns diff without side effects', () => {
    it('should return SessionResult with diff and file lists', async () => {
      // @step Given a git repository at the project root
      // testDir is our git repository

      // @step And an isolated session worktree exists
      const sessionId = randomUUID();

      // @step And the session has modified files
      const worktreePath = await createSessionWithFiles(sessionId, {
        'new-file.txt': 'new content',
        'existing.txt': 'modified content',
      });

      // Delete a file
      fs.unlinkSync(path.join(worktreePath, 'README.md'));

      destroySession(sessionId); // Remove from background sessions

      // @step When I call inspectSession for that session
      const result = inspectSession(testDir, sessionId);

      // @step Then a SessionResult should be returned
      expect(result).toBeDefined();
      expect(result.sessionId).toBe(sessionId);

      // @step And the result should contain a unified diff
      expect(result.diff).toBeDefined();
      expect(result.diff.length).toBeGreaterThan(0);
      // Unified diff format uses --- and +++ for file headers
      expect(result.diff).toContain('---');
      expect(result.diff).toContain('+++');

      // @step And the result should contain lists of changed, added, and deleted files
      expect(result.filesChanged).toContain('existing.txt');
      expect(result.filesAdded).toContain('new-file.txt');
      expect(result.filesDeleted).toContain('README.md');

      // @step And the worktree should remain intact
      expect(fs.existsSync(worktreePath)).toBe(true);
      expect(fs.existsSync(path.join(worktreePath, 'new-file.txt'))).toBe(true);
    });
  });

  describe('Scenario: mergeSession applies changes and removes worktree', () => {
    it('should copy files to main worktree and remove session worktree', async () => {
      // @step Given a git repository at the project root
      // testDir is our git repository

      // @step And an isolated session worktree exists
      const sessionId = randomUUID();

      // @step And the session has modified files
      const worktreePath = await createSessionWithFiles(sessionId, {
        'added-file.txt': 'added content',
        'existing.txt': 'modified by session',
      });

      // Also delete README.md in the session
      fs.unlinkSync(path.join(worktreePath, 'README.md'));

      destroySession(sessionId); // Remove from background sessions

      // Verify files don't exist in main yet
      expect(fs.existsSync(path.join(testDir, 'added-file.txt'))).toBe(false);
      expect(fs.readFileSync(path.join(testDir, 'existing.txt'), 'utf-8')).toBe(
        'existing content'
      );
      expect(fs.existsSync(path.join(testDir, 'README.md'))).toBe(true);

      // @step When I call mergeSession for that session
      const result = mergeSession(testDir, sessionId);

      // @step Then the modified files should be copied to the main worktree
      expect(fs.readFileSync(path.join(testDir, 'existing.txt'), 'utf-8')).toBe(
        'modified by session'
      );

      // @step And new files should be added to the main worktree
      expect(fs.existsSync(path.join(testDir, 'added-file.txt'))).toBe(true);
      expect(
        fs.readFileSync(path.join(testDir, 'added-file.txt'), 'utf-8')
      ).toBe('added content');

      // @step And deleted files should be removed from the main worktree
      expect(fs.existsSync(path.join(testDir, 'README.md'))).toBe(false);

      // @step And the session worktree should be removed
      expect(fs.existsSync(worktreePath)).toBe(false);

      // @step And a MergeResult should be returned with file lists
      expect(result.sessionId).toBe(sessionId);
      expect(result.filesModified).toContain('existing.txt');
      expect(result.filesAdded).toContain('added-file.txt');
      expect(result.filesDeleted).toContain('README.md');
    });
  });

  describe('Scenario: discardSession removes worktree without applying changes', () => {
    it('should remove worktree and leave main worktree unchanged', async () => {
      // @step Given a git repository at the project root
      // testDir is our git repository

      // @step And an isolated session worktree exists
      const sessionId = randomUUID();

      // @step And the session has modified files
      const worktreePath = await createSessionWithFiles(sessionId, {
        'discard-file.txt': 'this will be discarded',
        'existing.txt': 'modified but discarded',
      });

      destroySession(sessionId); // Remove from background sessions

      // Store original content
      const originalExisting = fs.readFileSync(
        path.join(testDir, 'existing.txt'),
        'utf-8'
      );

      // @step When I call discardSession for that session
      const result = discardSession(testDir, sessionId);

      // @step Then the worktree should be removed
      expect(fs.existsSync(worktreePath)).toBe(false);

      // @step And no files should be modified in the main worktree
      expect(fs.existsSync(path.join(testDir, 'discard-file.txt'))).toBe(false);
      expect(fs.readFileSync(path.join(testDir, 'existing.txt'), 'utf-8')).toBe(
        originalExisting
      );

      // @step And a DiscardResult should be returned with the files discarded count
      expect(result.sessionId).toBe(sessionId);
      expect(result.filesDiscarded).toBeGreaterThan(0);
    });
  });

  describe('Scenario: pruneOrphaned removes worktrees with no session records', () => {
    it('should remove orphaned worktrees and return count', async () => {
      // @step Given a git repository at the project root
      // testDir is our git repository

      // @step And there are orphaned worktrees with no session records
      const orphanedId1 = randomUUID();
      const orphanedId2 = randomUUID();
      const activeId = randomUUID();

      // Create worktrees
      await createSessionWithFiles(orphanedId1, {
        'orphan1.txt': 'orphan content 1',
      });
      await createSessionWithFiles(orphanedId2, {
        'orphan2.txt': 'orphan content 2',
      });
      await createSessionWithFiles(activeId, {});

      // Make orphaned sessions by destroying them AND deleting their manifests
      // (A session is only orphaned if it has NO valid manifest)
      destroySession(orphanedId1);
      destroySession(orphanedId2);
      deleteManifest(orphanedId1);
      deleteManifest(orphanedId2);
      // activeId remains active with its manifest

      // Verify worktrees exist
      const worktreesBefore = listWorktrees(testDir);
      expect(worktreesBefore.some(w => w.sessionId === orphanedId1)).toBe(true);
      expect(worktreesBefore.some(w => w.sessionId === orphanedId2)).toBe(true);

      // @step When I call pruneOrphaned with active session IDs
      const result = pruneOrphaned(testDir, [activeId]);

      // @step Then all orphaned worktrees should be removed
      const worktreesAfter = listWorktrees(testDir);
      expect(worktreesAfter.some(w => w.sessionId === orphanedId1)).toBe(false);
      expect(worktreesAfter.some(w => w.sessionId === orphanedId2)).toBe(false);

      // Active session worktree should remain
      expect(worktreesAfter.some(w => w.sessionId === activeId)).toBe(true);

      // @step And a PruneResult should be returned with the count of pruned sessions
      expect(result.count).toBe(2);
      expect(result.pruned).toContain(orphanedId1);
      expect(result.pruned).toContain(orphanedId2);
      expect(result.pruned).not.toContain(activeId);

      // Cleanup
      destroySession(activeId);
    });

    it('should return count of 0 when no orphaned worktrees exist', async () => {
      // Create an active session
      const activeId = randomUUID();
      await createSessionWithFiles(activeId, {});

      // Prune with the active session in the list
      const result = pruneOrphaned(testDir, [activeId]);

      // Should not prune active sessions
      expect(result.count).toBe(0);
      expect(result.pruned).toHaveLength(0);

      // Cleanup
      destroySession(activeId);
    });
  });

  // ========================================
  // GIT-029: SessionInfo isolation state fields
  // ========================================

  describe('Scenario: SessionInfo includes isolation state fields', () => {
    it('should return isIsolated=true and worktreePath for isolated sessions', async () => {
      // @step Given an isolated session has been created
      const sessionId = randomUUID();
      await sessionManagerCreateIsolated(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Isolated Session'
      );

      // @step When I call sessionManagerList
      const sessions = sessionManagerList();

      // @step Then the SessionInfo for that session should have isIsolated set to true
      const sessionInfo = sessions.find(s => s.id === sessionId);
      expect(sessionInfo).toBeDefined();
      expect(sessionInfo?.isIsolated).toBe(true);

      // @step Then the SessionInfo should have worktreePath set to the worktree directory
      expect(sessionInfo?.worktreePath).toBeDefined();
      expect(sessionInfo?.worktreePath).toContain('.fspec/worktrees');
      expect(sessionInfo?.worktreePath).toContain(sessionId);

      // Cleanup
      destroySession(sessionId);
    });

    it('should return isIsolated=false and worktreePath=undefined for normal sessions', async () => {
      // @step Given a normal (non-isolated) session has been created
      const sessionId = randomUUID();
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Normal Session'
      );

      // @step When I call sessionManagerList
      const sessions = sessionManagerList();

      // @step Then the SessionInfo for that session should have isIsolated set to false
      const sessionInfo = sessions.find(s => s.id === sessionId);
      expect(sessionInfo).toBeDefined();
      expect(sessionInfo?.isIsolated).toBe(false);

      // @step And the SessionInfo should have worktreePath set to undefined/null
      expect(sessionInfo?.worktreePath).toBeUndefined();

      // Cleanup
      destroySession(sessionId);
    });
  });
});
