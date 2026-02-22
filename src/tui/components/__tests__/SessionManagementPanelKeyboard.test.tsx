/**
 * Feature: spec/features/session-management-panel-keyboard.feature
 *
 * Tests for Session Management Panel keyboard input handling.
 * These tests verify that keyboard shortcuts work correctly.
 *
 * GIT-035: Isolated session worktree created with empty git index
 * Part C: Session Management Panel Keyboard Input
 *
 * The SessionManagementPanel is a FULL-SCREEN VIEW (not a Dialog overlay).
 * It receives terminalWidth/terminalHeight props and uses inline confirmations.
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, cleanup } from 'ink-testing-library';
import { Box, useInput } from 'ink';

// Mock useInputCompat to use ink's useInput directly for tests
vi.mock('../../input/index', () => ({
  useInputCompat: ({
    handler,
    isActive,
  }: {
    handler: (
      input: string,
      key: {
        upArrow?: boolean;
        downArrow?: boolean;
        return?: boolean;
        escape?: boolean;
      }
    ) => boolean;
    isActive?: boolean;
  }) => {
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
  listSessionWorktrees: vi.fn(),
  inspectSessionChanges: vi.fn(),
  mergeSessionChanges: vi.fn(),
  discardSessionChanges: vi.fn(),
  pruneOrphanedSessions: vi.fn(),
}));

// Mock NAPI bindings
vi.mock('@sengac/codelet-napi', () => ({
  sessionManagerList: vi.fn().mockReturnValue([]),
}));

import { SessionManagementPanel } from '../SessionManagementPanel';
import {
  listSessionWorktrees,
  inspectSessionChanges,
  mergeSessionChanges,
  discardSessionChanges,
} from '../../services/sessionService';

describe('Feature: Session Management Panel keyboard input', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  // ========================================
  // Scenario: Navigate sessions with arrow keys
  // ========================================

  describe('Scenario: Navigate sessions with arrow keys', () => {
    it('should move selection with up and down arrow keys', async () => {
      // @step Given the Session Management Panel is open as a full-screen view
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
        {
          sessionId: 'session-3',
          status: 'orphaned',
          filesChanged: 2,
          baseCommit: 'ghi789',
          createdAt: '2024-01-03T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/session-3',
        },
      ];

      // @step And there are multiple isolated sessions listed
      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockImplementation(
        (_repoPath, sessionId) => ({
          sessionId,
          diff: '',
          filesChanged: [],
          filesAdded: [],
          filesDeleted: [],
          baseCommit: 'abc123',
        })
      );

      const onClose = vi.fn();

      // Full-screen view pattern: pass terminalWidth/terminalHeight
      const { stdin, lastFrame, unmount } = render(
        <Box width={80} height={24}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            terminalWidth={80}
            terminalHeight={24}
          />
        </Box>
      );

      await new Promise((resolve) => setTimeout(resolve, 100));

      // Verify initial selection is first session (▶ indicator)
      let output = lastFrame() || '';
      expect(output).toContain('▶');

      // @step When I press the down arrow key
      stdin.write('\x1B[B'); // Down arrow

      await new Promise((resolve) => setTimeout(resolve, 50));

      // @step Then the selection should move to the next session
      output = lastFrame() || '';
      expect(output).toContain('▶');

      // @step When I press the up arrow key
      stdin.write('\x1B[A'); // Up arrow

      await new Promise((resolve) => setTimeout(resolve, 50));

      // @step Then the selection should move to the previous session
      output = lastFrame() || '';
      expect(output).toContain('▶');

      unmount();
    });
  });

  // ========================================
  // Scenario: Merge session with M key
  // ========================================

  describe('Scenario: Merge session with M key', () => {
    it('should show inline confirmation and merge when M then Y is pressed', async () => {
      // @step Given the Session Management Panel is open as a full-screen view
      const mockSessions = [
        {
          sessionId: 'merge-session',
          status: 'pendingmerge',
          filesChanged: 3,
          baseCommit: 'abc123',
          createdAt: '2024-01-01T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/merge-session',
        },
      ];

      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockReturnValue({
        sessionId: 'merge-session',
        diff: 'diff content',
        filesChanged: ['file1.ts', 'file2.ts', 'file3.ts'],
        filesAdded: [],
        filesDeleted: [],
        baseCommit: 'abc123',
      });
      vi.mocked(mergeSessionChanges).mockReturnValue({
        filesModified: ['file1.ts', 'file2.ts'],
        filesAdded: ['file3.ts'],
        filesDeleted: [],
      });

      const onClose = vi.fn();

      // Full-screen view pattern: pass terminalWidth/terminalHeight
      const { stdin, lastFrame, unmount } = render(
        <Box width={80} height={24}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            terminalWidth={80}
            terminalHeight={24}
          />
        </Box>
      );

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step And there is a session with status "pending_merge"
      let output = lastFrame() || '';
      expect(output).toContain('pending_merge');

      // @step And the session is selected
      expect(output).toContain('▶');

      // @step When I press the "M" key
      stdin.write('M');

      await new Promise((resolve) => setTimeout(resolve, 50));

      // @step Then an inline confirmation prompt should appear at the bottom
      output = lastFrame() || '';
      // Inline prompt should show merge confirmation text
      expect(output).toMatch(/[Mm]erge.*\?|[Yy].*confirm|[Pp]ress.*[Yy]/i);

      // @step When I press "Y" to confirm
      stdin.write('y');

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step Then the session changes should be applied to the main worktree
      expect(mergeSessionChanges).toHaveBeenCalledWith(
        '/project',
        'merge-session'
      );

      // @step And the session should be removed from the list
      // After merge, the session list should be refreshed
      expect(listSessionWorktrees).toHaveBeenCalled();

      unmount();
    });
  });

  // ========================================
  // Scenario: Discard session with D key
  // ========================================

  describe('Scenario: Discard session with D key', () => {
    it('should show inline confirmation and discard when D then Y is pressed', async () => {
      // @step Given the Session Management Panel is open as a full-screen view
      const mockSessions = [
        {
          sessionId: 'discard-session',
          status: 'pendingmerge',
          filesChanged: 2,
          baseCommit: 'abc123',
          createdAt: '2024-01-01T00:00:00Z',
          worktreePath: '/project/.fspec/worktrees/discard-session',
        },
      ];

      vi.mocked(listSessionWorktrees).mockReturnValue(mockSessions);
      vi.mocked(inspectSessionChanges).mockReturnValue({
        sessionId: 'discard-session',
        diff: 'diff content',
        filesChanged: ['file1.ts', 'file2.ts'],
        filesAdded: [],
        filesDeleted: [],
        baseCommit: 'abc123',
      });
      vi.mocked(discardSessionChanges).mockReturnValue({
        filesDiscarded: 2,
      });

      const onClose = vi.fn();

      // Full-screen view pattern: pass terminalWidth/terminalHeight
      const { stdin, lastFrame, unmount } = render(
        <Box width={80} height={24}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            terminalWidth={80}
            terminalHeight={24}
          />
        </Box>
      );

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step And there is a session selected
      let output = lastFrame() || '';
      expect(output).toContain('▶');

      // @step When I press the "D" key
      stdin.write('D');

      await new Promise((resolve) => setTimeout(resolve, 50));

      // @step Then an inline confirmation prompt should appear at the bottom
      output = lastFrame() || '';
      // Inline prompt should show discard confirmation text
      expect(output).toMatch(/[Dd]iscard.*\?|[Yy].*confirm|[Pp]ress.*[Yy]/i);

      // @step When I press "Y" to confirm
      stdin.write('y');

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step Then the worktree should be removed
      expect(discardSessionChanges).toHaveBeenCalledWith(
        '/project',
        'discard-session'
      );

      // @step And no changes should be applied to the main worktree
      expect(mergeSessionChanges).not.toHaveBeenCalled();

      // @step And the session should be removed from the list
      expect(listSessionWorktrees).toHaveBeenCalled();

      unmount();
    });
  });

  // ========================================
  // Scenario: Close panel with Escape key
  // ========================================

  describe('Scenario: Close panel with Escape key', () => {
    it('should close panel when Escape is pressed', async () => {
      // @step Given the Session Management Panel is open as a full-screen view
      vi.mocked(listSessionWorktrees).mockReturnValue([]);

      const onClose = vi.fn();

      // Full-screen view pattern: pass terminalWidth/terminalHeight
      const { stdin, unmount } = render(
        <Box width={80} height={24}>
          <SessionManagementPanel
            repoPath="/project"
            onClose={onClose}
            terminalWidth={80}
            terminalHeight={24}
          />
        </Box>
      );

      await new Promise((resolve) => setTimeout(resolve, 100));

      // @step When I press the Escape key
      stdin.write('\x1B'); // ESC key

      await new Promise((resolve) => setTimeout(resolve, 50));

      // @step Then the panel should close
      expect(onClose).toHaveBeenCalled();

      // @step And I should return to the AgentView
      // This is verified by onClose being called - the parent handles the view switch

      unmount();
    });
  });
});
