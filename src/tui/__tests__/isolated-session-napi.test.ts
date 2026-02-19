/**
 * Feature: spec/features/create-isolated-session-napi.feature
 *
 * Tests for the sessionManagerCreateIsolated NAPI binding.
 * These tests call the actual NAPI binding (not source-code-grep).
 *
 * GIT-028: Add createIsolatedSession NAPI binding
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { execSync } from 'child_process';
import { randomUUID } from 'crypto';

// Import the NAPI bindings we need to test
// Note: sessionManagerCreateIsolated doesn't exist yet - tests will fail
import {
  sessionManagerCreateIsolated,
  sessionManagerDestroy,
  listSessions,
  listWorktrees,
  removeWorktree,
  persistenceSetDataDirectory,
} from '@sengac/codelet-napi';

describe('Feature: Add createIsolatedSession NAPI binding', () => {
  let testDir: string;
  let testSessionId: string;
  let dataDir: string;

  beforeEach(async () => {
    // Create a temporary git repository for testing
    testDir = fs.mkdtempSync(
      path.join(os.tmpdir(), 'fspec-isolated-session-test-')
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
    execSync('git add .', { cwd: testDir, stdio: 'pipe' });
    execSync('git commit -m "Initial commit"', { cwd: testDir, stdio: 'pipe' });

    // Generate unique session ID (UUID format required)
    testSessionId = randomUUID();
  });

  afterEach(async () => {
    // Cleanup: destroy session if it exists
    try {
      sessionManagerDestroy(testSessionId);
    } catch {
      // Session may not exist
    }

    // Cleanup: remove worktree if it exists
    try {
      removeWorktree(testDir, testSessionId);
    } catch {
      // Worktree may not exist
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

  describe('Scenario: Create isolated session with worktree', () => {
    it('should create worktree and return session info with worktree_path', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And no worktree exists for session "abc-123"
      const worktreesBefore = listWorktrees(testDir);
      expect(
        worktreesBefore.find(w => w.sessionId === testSessionId)
      ).toBeUndefined();

      // @step When I call sessionManagerCreateIsolated with session ID "abc-123", model "anthropic/claude", project "/project", and name "My Session"
      const result = await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Test Isolated Session'
      );

      // @step Then a worktree should be created at "/project/.fspec/worktrees/abc-123/"
      const worktreePath = path.join(
        testDir,
        '.fspec',
        'worktrees',
        testSessionId
      );
      expect(fs.existsSync(worktreePath)).toBe(true);

      // @step And the returned session info should include worktree_path
      expect(result.worktreePath).toBeDefined();
      expect(result.worktreePath).toContain(testSessionId);

      // @step And the returned session info should include base_commit
      expect(result.baseCommit).toBeDefined();
      expect(result.baseCommit.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Isolated session files appear in worktree not main project', () => {
    it('should write files to worktree directory not main project', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session "abc-123" has been created
      const result = await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Test Isolated Session'
      );
      const worktreePath = result.worktreePath;

      // @step When a file "test.txt" is written via the isolated session
      const testFilePath = path.join(worktreePath, 'test.txt');
      fs.writeFileSync(testFilePath, 'test content');

      // @step Then the file should exist at "/project/.fspec/worktrees/abc-123/test.txt"
      expect(fs.existsSync(testFilePath)).toBe(true);

      // @step And the file should NOT exist at "/project/test.txt"
      const mainProjectFile = path.join(testDir, 'test.txt');
      expect(fs.existsSync(mainProjectFile)).toBe(false);
    });
  });

  describe('Scenario: Creating duplicate isolated session fails with WorktreeExists error', () => {
    it('should fail when creating session with same ID twice', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step And an isolated session "abc-123" already exists
      await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'First Session'
      );

      // @step When I call sessionManagerCreateIsolated with session ID "abc-123"
      // @step Then the operation should fail with WorktreeExists error
      // @step And the error message should reference session "abc-123"
      await expect(
        sessionManagerCreateIsolated(
          testSessionId,
          'anthropic/claude-sonnet-4-20250514',
          testDir,
          'Second Session'
        )
      ).rejects.toThrow(/WorktreeExists|already exists/i);
    });
  });

  describe('Scenario: Session manifest created for orphan detection', () => {
    it('should create manifest file at ~/.fspec/git-sessions/', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step When I call sessionManagerCreateIsolated with session ID "abc-123"
      const result = await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Test Session'
      );

      // @step Then a session manifest should exist at "~/.fspec/git-sessions/abc-123.json"
      const homeDir = os.homedir();
      const manifestPath = path.join(
        homeDir,
        '.fspec',
        'git-sessions',
        `${testSessionId}.json`
      );
      expect(fs.existsSync(manifestPath)).toBe(true);

      // @step And the manifest should contain the project root path
      const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf-8'));
      expect(manifest.project_root || manifest.projectRoot).toBeDefined();

      // @step And the manifest should contain the worktree path
      expect(manifest.worktree_path || manifest.worktreePath).toBeDefined();
    });
  });

  describe('Scenario: Isolated session appears in listSessions with active status', () => {
    it('should show session as active in listSessions', async () => {
      // @step Given a git repository at "/project"
      // testDir is our git repository

      // @step When I call sessionManagerCreateIsolated with session ID "abc-123"
      await sessionManagerCreateIsolated(
        testSessionId,
        'anthropic/claude-sonnet-4-20250514',
        testDir,
        'Test Session'
      );

      // @step And I call listSessions with "abc-123" in the active sessions set
      const sessions = listSessions(testDir, [testSessionId], 'all');

      // @step Then the session should appear in the results
      const session = sessions.find(s => s.sessionId === testSessionId);
      expect(session).toBeDefined();

      // @step And the session status should be "active"
      expect(session?.status).toBe('active');
    });
  });
});
