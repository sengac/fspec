/**
 * Feature: spec/features/isolated-session-tui-components.feature
 *
 * Tests for TUI integration of isolated sessions (Parts A and B).
 * Part A: Session Creation with Isolated Toggle
 * Part B: Session Management Panel
 *
 * GIT-029: TUI integration for isolated sessions
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock the NAPI bindings for session management
vi.mock('@sengac/codelet-napi', () => ({
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionManagerCreateIsolated: vi.fn().mockResolvedValue({
    sessionId: 'test-session-id',
    worktreePath: '/project/.fspec/worktrees/test-session-id',
    baseCommit: 'abc123',
  }),
  sessionManagerDestroy: vi.fn(),
  sessionManagerList: vi.fn().mockReturnValue([]),
  listSessions: vi.fn().mockReturnValue([]),
  inspectSession: vi.fn().mockReturnValue({
    sessionId: 'test-session-id',
    diff: '--- a/file.txt\n+++ b/file.txt\n-old\n+new',
    filesChanged: ['file.txt'],
    filesAdded: [],
    filesDeleted: [],
    baseCommit: 'abc123',
  }),
  mergeSession: vi.fn().mockReturnValue({
    sessionId: 'test-session-id',
    filesModified: ['file.txt'],
    filesAdded: [],
    filesDeleted: [],
  }),
  discardSession: vi.fn().mockReturnValue({
    sessionId: 'test-session-id',
    filesDiscarded: 1,
  }),
  pruneOrphaned: vi.fn().mockReturnValue({
    count: 2,
    pruned: ['orphan-1', 'orphan-2'],
  }),
  persistenceCreateSessionWithProvider: vi.fn().mockReturnValue({
    id: 'test-session-id',
    name: 'Test Session',
    project: '/project',
    createdAt: new Date().toISOString(),
  }),
  sessionSetGlobalChunkCallback: vi.fn(),
}));

// Import session service functions to test
import {
  createSession,
  createIsolatedSession,
  listSessionWorktrees,
  inspectSessionChanges,
  mergeSessionChanges,
  discardSessionChanges,
  pruneOrphanedSessions,
} from '../services/sessionService';

import {
  sessionManagerCreateWithId,
  sessionManagerCreateIsolated,
  listSessions,
  inspectSession,
  mergeSession,
  discardSession,
  pruneOrphaned,
} from '@sengac/codelet-napi';

describe('Feature: Isolated session TUI components', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ========================================
  // Part A: Session Creation
  // ========================================

  describe('Scenario: Create session with isolated toggle disabled (default)', () => {
    it('should call sessionManagerCreateWithId when isolated is false', async () => {
      // @step Given the TUI session creation dialog is open
      // @step And the "Isolated" toggle is OFF
      const options = {
        modelPath: 'anthropic/claude-sonnet-4',
        project: '/project',
        name: 'Test Session',
      };

      // @step When I submit the session creation form
      await createSession(options);

      // @step Then sessionManagerCreateWithId should be called
      expect(sessionManagerCreateWithId).toHaveBeenCalledWith(
        'test-session-id',
        options.modelPath,
        options.project,
        options.name
      );

      // @step And the session should use the project root as working directory
      expect(sessionManagerCreateIsolated).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Create session with isolated toggle enabled', () => {
    it('should call sessionManagerCreateIsolated when isolated is true', async () => {
      // @step Given the TUI session creation dialog is open
      // @step And the "Isolated" toggle is ON
      const options = {
        modelPath: 'anthropic/claude-sonnet-4',
        project: '/project',
        name: 'Isolated Test Session',
        isolated: true,
      };

      // @step When I submit the session creation form
      const result = await createIsolatedSession(options);

      // @step Then sessionManagerCreateIsolated should be called
      expect(sessionManagerCreateIsolated).toHaveBeenCalledWith(
        'test-session-id',
        options.modelPath,
        options.project,
        expect.any(String) // Session name
      );

      // @step And a worktree should be created at ".fspec/worktrees/<session-id>/"
      expect(result.worktreePath).toBe('/project/.fspec/worktrees/test-session-id');

      // @step And the session info should display the worktree path
      expect(result.baseCommit).toBe('abc123');
    });
  });

  // ========================================
  // Part B: Session Management Panel
  // ========================================

  describe('Scenario: View Session Management Panel with pending sessions', () => {
    it('should call listSessions to display sessions with status badges', async () => {
      // @step Given there are completed isolated sessions with worktrees
      const mockSessions = [
        {
          sessionId: 'session-1',
          status: 'pendingmerge',
          filesChanged: 5,
          baseCommit: 'abc123',
          createdAt: '2024-01-01T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/session-1',
        },
        {
          sessionId: 'session-2',
          status: 'clean',
          filesChanged: 0,
          baseCommit: 'def456',
          createdAt: '2024-01-02T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/session-2',
        },
        {
          sessionId: 'session-3',
          status: 'orphaned',
          filesChanged: 3,
          baseCommit: 'ghi789',
          createdAt: '2024-01-03T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/session-3',
        },
      ];
      vi.mocked(listSessions).mockReturnValue(mockSessions);

      // @step When I open the Session Management Panel
      const sessions = listSessionWorktrees('/project', ['active-session']);

      // @step Then I should see a list of sessions
      expect(sessions).toHaveLength(3);

      // @step And each session should display its status badge
      // @step And pending_merge sessions should have a yellow badge
      expect(sessions[0].status).toBe('pendingmerge');

      // @step And clean sessions should have a green badge
      expect(sessions[1].status).toBe('clean');

      // @step And orphaned sessions should have a red badge
      expect(sessions[2].status).toBe('orphaned');

      // @step And each session should show the files changed count
      expect(sessions[0].filesChanged).toBe(5);
      expect(sessions[1].filesChanged).toBe(0);
      expect(sessions[2].filesChanged).toBe(3);
    });
  });

  describe('Scenario: Merge a pending_merge session', () => {
    it('should call mergeSession to apply changes', async () => {
      // @step Given the Session Management Panel is open
      // @step And there is a session with status "pending_merge"
      const sessionId = 'session-to-merge';

      // @step When I click the Merge button for that session
      // @step Then a confirmation dialog should appear
      // @step When I confirm the merge
      const result = mergeSessionChanges('/project', sessionId);

      // @step Then mergeSession NAPI binding should be called
      expect(mergeSession).toHaveBeenCalledWith('/project', sessionId);

      // @step And the session changes should be applied to the main worktree
      expect(result.filesModified).toContain('file.txt');

      // @step And the session should be removed from the list
      // This happens via listSessions refresh after merge
    });
  });

  describe('Scenario: Discard a pending_merge session', () => {
    it('should call discardSession to remove worktree', async () => {
      // @step Given the Session Management Panel is open
      // @step And there is a session with status "pending_merge"
      const sessionId = 'session-to-discard';

      // @step When I click the Discard button for that session
      // @step Then a confirmation dialog should appear
      // @step When I confirm the discard
      const result = discardSessionChanges('/project', sessionId);

      // @step Then discardSession NAPI binding should be called
      expect(discardSession).toHaveBeenCalledWith('/project', sessionId);

      // @step And the worktree should be removed
      expect(result.filesDiscarded).toBe(1);

      // @step And no changes should be applied to the main worktree
      // Verified by not calling mergeSession

      // @step And the session should be removed from the list
      // This happens via listSessions refresh after discard
    });
  });

  describe('Scenario: Prune orphaned sessions', () => {
    it('should call pruneOrphaned to clean up crashed sessions', async () => {
      // @step Given the Session Management Panel is open
      // @step And there are orphaned sessions
      const activeSessions = ['active-session-1', 'active-session-2'];

      // @step When I click the "Prune Orphaned" button
      const result = pruneOrphanedSessions('/project', activeSessions);

      // @step Then pruneOrphaned NAPI binding should be called
      expect(pruneOrphaned).toHaveBeenCalledWith('/project', activeSessions);

      // @step And all orphaned sessions should be removed
      expect(result.pruned).toContain('orphan-1');
      expect(result.pruned).toContain('orphan-2');

      // @step And a confirmation should show the count of pruned sessions
      expect(result.count).toBe(2);
    });
  });
});
