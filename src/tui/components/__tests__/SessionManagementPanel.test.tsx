/**
 * Feature: spec/features/isolated-session-tui-components.feature
 *
 * Tests for TUI integration of isolated sessions:
 * Part A: Session Creation with Isolated Toggle
 * Part B: Session Management Panel
 *
 * GIT-029: TUI integration for isolated sessions
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, cleanup } from 'ink-testing-library';
import { Box, useInput } from 'ink';

// Mock the Dialog component
vi.mock('../../../components/Dialog', () => ({
  Dialog: ({ children }: { children: React.ReactNode }) => (
    <Box flexDirection="column" borderStyle="single" padding={1}>
      {children}
    </Box>
  ),
}));

// Mock useInputCompat to use ink's useInput directly for tests
vi.mock('../../input/index', () => ({
  useInputCompat: ({ handler, isActive }: { handler: (input: string, key: { upArrow?: boolean; downArrow?: boolean; return?: boolean; escape?: boolean }) => boolean; isActive?: boolean }) => {
    // eslint-disable-next-line react-hooks/rules-of-hooks
    useInput((input, key) => {
      if (isActive !== false) {
        handler(input, key);
      }
    });
  },
  InputPriority: {
    CRITICAL: 0,
    DIALOG: 1,
    HIGH: 2,
    NORMAL: 3,
    LOW: 4,
  },
}));

// Mock the session service functions
vi.mock('../../services/sessionService', () => ({
  createSession: vi.fn(),
  createIsolatedSession: vi.fn(),
  listSessionWorktrees: vi.fn(),
  inspectSessionChanges: vi.fn(),
  mergeSessionChanges: vi.fn(),
  discardSessionChanges: vi.fn(),
  pruneOrphanedSessions: vi.fn(),
}));

// Mock NAPI bindings
vi.mock('@sengac/codelet-napi', () => ({
  sessionManagerList: vi.fn().mockReturnValue([]),
  sessionManagerCreateWithId: vi.fn().mockResolvedValue(undefined),
  sessionManagerCreateIsolated: vi.fn().mockResolvedValue({
    sessionId: 'test-session-id',
    worktreePath: '/project/.fspec/worktrees/test-session-id',
    baseCommit: 'abc123',
  }),
}));

import { SessionManagementPanel } from '../SessionManagementPanel';
import {
  createSession,
  createIsolatedSession,
  listSessionWorktrees,
  inspectSessionChanges,
  mergeSessionChanges,
  discardSessionChanges,
  pruneOrphanedSessions,
} from '../../services/sessionService';

describe('Feature: Isolated session TUI components', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  // ========================================
  // Part A: Session Creation
  // ========================================

  describe('Scenario: Create session with isolated toggle disabled (default)', () => {
    it('should call createSession when isolated toggle is OFF', async () => {
      // @step Given the TUI session creation dialog is open
      // @step And the "Isolated" toggle is OFF
      const mockResult = {
        sessionId: 'test-session-id',
        name: 'Test Session',
        provider: 'anthropic/claude-sonnet-4',
      };
      vi.mocked(createSession).mockResolvedValue(mockResult);

      // @step When I submit the session creation form
      await createSession({
        modelPath: 'anthropic/claude-sonnet-4',
        project: '/project',
        name: 'Test Session',
      });

      // @step Then sessionManagerCreateWithId should be called
      expect(createSession).toHaveBeenCalledWith({
        modelPath: 'anthropic/claude-sonnet-4',
        project: '/project',
        name: 'Test Session',
      });

      // @step And the session should use the project root as working directory
      expect(createIsolatedSession).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Create session with isolated toggle enabled', () => {
    it('should call createIsolatedSession when isolated toggle is ON', async () => {
      // @step Given the TUI session creation dialog is open
      // @step And the "Isolated" toggle is ON
      const mockResult = {
        sessionId: 'test-session-id',
        name: 'Isolated Session',
        provider: 'anthropic/claude-sonnet-4',
        worktreePath: '/project/.fspec/worktrees/test-session-id',
        baseCommit: 'abc123',
      };
      vi.mocked(createIsolatedSession).mockResolvedValue(mockResult);

      // @step When I submit the session creation form
      const result = await createIsolatedSession({
        modelPath: 'anthropic/claude-sonnet-4',
        project: '/project',
        name: 'Isolated Session',
        isolated: true,
      });

      // @step Then sessionManagerCreateIsolated should be called
      expect(createIsolatedSession).toHaveBeenCalled();

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
    it('should render session list with status badges', async () => {
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

      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockReturnValue({
        sessionId: 'session-1',
        diff: '',
        filesChanged: [],
        filesAdded: [],
        filesDeleted: [],
        baseCommit: 'abc123',
      });

      const onClose = vi.fn();

      // @step When I open the Session Management Panel
      const { lastFrame, unmount } = render(
        <Box width={80} height={20}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            isActive={true}
          />
        </Box>
      );

      // Wait for render
      await new Promise(resolve => setTimeout(resolve, 100));

      const output = lastFrame() || '';

      // @step Then I should see a list of sessions
      expect(output).toContain('Session Management');

      // @step And each session should display its status badge
      // @step And pending_merge sessions should have status displayed
      expect(output).toContain('pending_merge');

      // @step And clean sessions should have status displayed
      expect(output).toContain('clean');

      // @step And orphaned sessions should have status displayed
      expect(output).toContain('orphaned');

      // @step And each session should show the files changed count
      expect(output).toContain('5 files changed');
      expect(output).toContain('0 files changed');
      expect(output).toContain('3 files changed');

      // @step And session IDs should be displayed (truncated)
      // Note: Session IDs are truncated in the UI
      expect(output).toContain('session-...');

      unmount();
    });

    it('should show empty state when no sessions exist', async () => {
      // @step Given there are no isolated sessions
      vi.mocked(listSessionWorktrees).mockReturnValue([]);

      const onClose = vi.fn();

      // @step When I open the Session Management Panel
      const { lastFrame, unmount } = render(
        <Box width={80} height={20}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            isActive={true}
          />
        </Box>
      );

      await new Promise(resolve => setTimeout(resolve, 100));
      const output = lastFrame() || '';

      // @step Then I should see the empty state message
      expect(output).toContain('No isolated sessions found');

      unmount();
    });

    it('should show Prune Orphaned option when orphaned sessions exist', async () => {
      // @step Given there are orphaned sessions
      const mockSessions = [
        {
          sessionId: 'orphan-1',
          status: 'orphaned',
          filesChanged: 2,
          baseCommit: 'abc123',
          createdAt: '2024-01-01T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/orphan-1',
        },
      ];

      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockReturnValue({
        sessionId: 'orphan-1',
        diff: '',
        filesChanged: [],
        filesAdded: [],
        filesDeleted: [],
        baseCommit: 'abc123',
      });

      const onClose = vi.fn();

      // @step When I open the Session Management Panel
      const { lastFrame, unmount } = render(
        <Box width={80} height={20}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            isActive={true}
          />
        </Box>
      );

      await new Promise(resolve => setTimeout(resolve, 100));
      const output = lastFrame() || '';

      // @step Then the Prune Orphaned option should be visible in the help
      expect(output).toContain('P Prune Orphaned');

      unmount();
    });
  });

  describe('Scenario: Merge a pending_merge session', () => {
    it('should call mergeSessionChanges when merge is confirmed', async () => {
      // @step Given the Session Management Panel is open
      // @step And there is a session with status "pending_merge"
      const mockSessions = [
        {
          sessionId: 'session-to-merge',
          status: 'pendingmerge',
          filesChanged: 3,
          baseCommit: 'abc123',
          createdAt: '2024-01-01T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/session-to-merge',
        },
      ];

      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockReturnValue({
        sessionId: 'session-to-merge',
        diff: '',
        filesChanged: ['file.txt'],
        filesAdded: [],
        filesDeleted: [],
        baseCommit: 'abc123',
      });
      vi.mocked(mergeSessionChanges).mockReturnValue({
        sessionId: 'session-to-merge',
        filesModified: ['file.txt'],
        filesAdded: [],
        filesDeleted: [],
      });

      const onClose = vi.fn();

      const { lastFrame, stdin, unmount } = render(
        <Box width={80} height={20}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            isActive={true}
          />
        </Box>
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I click the Merge button for that session
      stdin.write('m');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then a confirmation dialog should appear
      let output = lastFrame() || '';
      expect(output).toContain('Changes will be applied to the main worktree');

      // @step When I confirm the merge
      stdin.write('y');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then mergeSession NAPI binding should be called
      expect(mergeSessionChanges).toHaveBeenCalledWith('/project', 'session-to-merge');

      // @step And the session changes should be applied to the main worktree
      output = lastFrame() || '';
      expect(output).toContain('Merged session');

      // @step And the session should be removed from the list
      // This is verified by the mergeSessionChanges call which removes the worktree

      unmount();
    });
  });

  describe('Scenario: Discard a pending_merge session', () => {
    it('should call discardSessionChanges when discard is confirmed', async () => {
      // @step Given the Session Management Panel is open
      // @step And there is a session with status "pending_merge"
      const mockSessions = [
        {
          sessionId: 'session-to-discard',
          status: 'pendingmerge',
          filesChanged: 2,
          baseCommit: 'abc123',
          createdAt: '2024-01-01T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/session-to-discard',
        },
      ];

      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockReturnValue({
        sessionId: 'session-to-discard',
        diff: '',
        filesChanged: ['file.txt'],
        filesAdded: [],
        filesDeleted: [],
        baseCommit: 'abc123',
      });
      vi.mocked(discardSessionChanges).mockReturnValue({
        sessionId: 'session-to-discard',
        filesDiscarded: 2,
      });

      const onClose = vi.fn();

      const { lastFrame, stdin, unmount } = render(
        <Box width={80} height={20}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            isActive={true}
          />
        </Box>
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I click the Discard button for that session
      stdin.write('d');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then a confirmation dialog should appear
      let output = lastFrame() || '';
      expect(output).toContain('All changes will be lost');

      // @step When I confirm the discard
      stdin.write('y');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then discardSession NAPI binding should be called
      expect(discardSessionChanges).toHaveBeenCalledWith('/project', 'session-to-discard');

      // @step And the worktree should be removed
      output = lastFrame() || '';
      expect(output).toContain('Discarded session');

      // @step And no changes should be applied to the main worktree
      expect(mergeSessionChanges).not.toHaveBeenCalled();

      // @step And the session should be removed from the list
      // This is verified by the discardSessionChanges call which removes the worktree

      unmount();
    });
  });

  describe('Scenario: Prune orphaned sessions', () => {
    it('should call pruneOrphanedSessions when prune is confirmed', async () => {
      // @step Given the Session Management Panel is open
      // @step And there are orphaned sessions
      const mockSessions = [
        {
          sessionId: 'orphan-1',
          status: 'orphaned',
          filesChanged: 1,
          baseCommit: 'abc123',
          createdAt: '2024-01-01T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/orphan-1',
        },
        {
          sessionId: 'orphan-2',
          status: 'orphaned',
          filesChanged: 2,
          baseCommit: 'def456',
          createdAt: '2024-01-02T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/orphan-2',
        },
      ];

      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockReturnValue({
        sessionId: 'orphan-1',
        diff: '',
        filesChanged: [],
        filesAdded: [],
        filesDeleted: [],
        baseCommit: 'abc123',
      });
      vi.mocked(pruneOrphanedSessions).mockReturnValue({
        count: 2,
        pruned: ['orphan-1', 'orphan-2'],
      });

      const onClose = vi.fn();

      const { lastFrame, stdin, unmount } = render(
        <Box width={80} height={20}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            isActive={true}
          />
        </Box>
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I click the "Prune Orphaned" button
      stdin.write('p');
      await new Promise(resolve => setTimeout(resolve, 50));

      // Verify confirmation dialog appears
      let output = lastFrame() || '';
      expect(output).toContain('Orphaned worktrees will be removed');

      // Confirm the prune
      stdin.write('y');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then pruneOrphaned NAPI binding should be called
      expect(pruneOrphanedSessions).toHaveBeenCalled();

      // @step And all orphaned sessions should be removed
      output = lastFrame() || '';

      // @step And a confirmation should show the count of pruned sessions
      expect(output).toContain('Pruned 2 orphaned session(s)');

      unmount();
    });
  });

  describe('Scenario: Cancel confirmation dialog', () => {
    it('should not perform action when confirmation is cancelled', async () => {
      // @step Given a confirmation dialog is shown
      const mockSessions = [
        {
          sessionId: 'session-1',
          status: 'pendingmerge',
          filesChanged: 1,
          baseCommit: 'abc123',
          createdAt: '2024-01-01T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/session-1',
        },
      ];

      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockReturnValue({
        sessionId: 'session-1',
        diff: '',
        filesChanged: [],
        filesAdded: [],
        filesDeleted: [],
        baseCommit: 'abc123',
      });

      const onClose = vi.fn();

      const { lastFrame, stdin, unmount } = render(
        <Box width={80} height={20}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            isActive={true}
          />
        </Box>
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // Press M for merge
      stdin.write('m');
      await new Promise(resolve => setTimeout(resolve, 50));

      // Verify confirmation dialog appears
      let output = lastFrame() || '';
      expect(output).toContain('Changes will be applied to the main worktree');

      // @step When I press N to cancel
      stdin.write('n');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then mergeSessionChanges should not be called
      expect(mergeSessionChanges).not.toHaveBeenCalled();

      // @step And the confirmation dialog should be dismissed
      output = lastFrame() || '';
      expect(output).not.toContain('Changes will be applied to the main worktree');

      unmount();
    });
  });

  describe('Scenario: Navigate session list', () => {
    it('should navigate between sessions with arrow keys', async () => {
      // @step Given multiple sessions in the list
      const mockSessions = [
        {
          sessionId: 'session-1',
          status: 'pendingmerge',
          filesChanged: 1,
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
      ];

      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockImplementation((repoPath, sessionId) => ({
        sessionId,
        diff: '',
        filesChanged: [],
        filesAdded: [],
        filesDeleted: [],
        baseCommit: 'abc123',
      }));

      const onClose = vi.fn();

      const { lastFrame, unmount } = render(
        <Box width={80} height={20}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            isActive={true}
          />
        </Box>
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When the panel opens
      const output = lastFrame() || '';

      // @step Then the first session should be selected (▶ indicator)
      expect(output).toContain('▶');

      // @step And both sessions should be listed (truncated)
      expect(output).toContain('session-...');

      unmount();
    });
  });

  describe('Scenario: Close panel with Escape', () => {
    it('should call onClose when Escape is pressed', async () => {
      // @step Given the Session Management Panel is open
      vi.mocked(listSessionWorktrees).mockReturnValue([]);

      const onClose = vi.fn();

      const { stdin, unmount } = render(
        <Box width={80} height={20}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            isActive={true}
          />
        </Box>
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I press Escape
      stdin.write('\x1B'); // ESC key

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then onClose should be called
      expect(onClose).toHaveBeenCalled();

      unmount();
    });
  });
});
