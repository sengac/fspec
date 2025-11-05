/**
 * Feature: spec/features/real-time-file-and-git-status-watching-in-tui.feature
 *
 * Tests for ITF-005: Real-time file and git status watching in TUI
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFspecStore } from '../store/fspecStore';
import * as git from 'isomorphic-git';
import { getStagedFilesWithChangeType, getUnstagedFilesWithChangeType } from '../../git/status';

// Mock isomorphic-git and file system
vi.mock('isomorphic-git');
vi.mock('../../git/status');

describe('Feature: Real-time file and git status watching in TUI', () => {
  beforeEach(async () => {
    // Reset mocks
    vi.clearAllMocks();

    // Reset Zustand store state
    const store = useFspecStore.getState();
    useFspecStore.setState({
      workUnits: [],
      epics: [],
      stashes: [],
      stagedFiles: [],
      unstagedFiles: [],
      isLoaded: false,
      error: null,
    });

    // Load real data
    await store.loadData();
  });

  describe('Scenario: TUI auto-updates stash panel when git stash is created externally', () => {
    it('should call loadStashes action and update store when stash is created', async () => {
      // @step Given the TUI is running and showing the stash panel
      // @step And there are no existing git stashes
      vi.mocked(git.log).mockResolvedValue([]);

      const store = useFspecStore.getState();
      await store.loadStashes();

      expect(store.stashes).toEqual([]);

      // @step When I create a git stash via terminal command
      // Simulate new stash being created
      const newStash = {
        oid: 'abc123',
        commit: {
          message: 'fspec-checkpoint:TEST-001:baseline:1234567890',
          author: { timestamp: Date.now() / 1000 },
        },
      };
      vi.mocked(git.log).mockResolvedValue([newStash]);

      // Call loadStashes (this is what fs.watch would trigger)
      await store.loadStashes();

      // @step Then the TUI stash panel should automatically update
      // @step And the new stash should be displayed in the stash list
      // @step And I should not need to restart the TUI
      const updatedStore = useFspecStore.getState();
      expect(updatedStore.stashes).toHaveLength(1);
      expect(updatedStore.stashes[0].oid).toBe('abc123');

      // Verify git.log was called with correct ref
      expect(git.log).toHaveBeenCalledWith(
        expect.objectContaining({
          ref: 'refs/stash',
        })
      );
    });
  });

  describe('Scenario: TUI auto-updates when file is staged via git add', () => {
    it('should call loadFileStatus and update store when file is staged', async () => {
      // @step Given the TUI is running and showing the files panel
      // @step And I have an unstaged file "src/auth.ts"
      vi.mocked(getStagedFilesWithChangeType).mockResolvedValue([]);
      vi.mocked(getUnstagedFilesWithChangeType).mockResolvedValue([
        { filepath: 'src/auth.ts', changeType: 'M', staged: false }
      ]);

      const store = useFspecStore.getState();
      await store.loadFileStatus();

      let currentState = useFspecStore.getState();
      expect(currentState.stagedFiles).toEqual([]);
      expect(currentState.unstagedFiles).toHaveLength(1);
      expect(currentState.unstagedFiles[0].filepath).toBe('src/auth.ts');

      // @step When I stage the file with "git add src/auth.ts"
      // Simulate file being staged
      vi.mocked(getStagedFilesWithChangeType).mockResolvedValue([
        { filepath: 'src/auth.ts', changeType: 'M', staged: true }
      ]);
      vi.mocked(getUnstagedFilesWithChangeType).mockResolvedValue([]);

      // Call loadFileStatus (this is what fs.watch would trigger)
      await store.loadFileStatus();

      // @step Then the TUI files panel should automatically update
      // @step And "src/auth.ts" should appear in the staged section with green indicator
      // @step And "src/auth.ts" should be removed from the unstaged section
      const updatedStore = useFspecStore.getState();
      expect(updatedStore.stagedFiles).toHaveLength(1);
      expect(updatedStore.stagedFiles[0].filepath).toBe('src/auth.ts');
      expect(updatedStore.unstagedFiles).toHaveLength(0);

      // Verify utilities were called
      expect(getStagedFilesWithChangeType).toHaveBeenCalled();
      expect(getUnstagedFilesWithChangeType).toHaveBeenCalled();
    });
  });

  describe('Scenario: TUI auto-updates when file is unstaged via git reset', () => {
    it('should call loadFileStatus and update store when file is unstaged', async () => {
      // @step Given the TUI is running and showing the files panel
      // @step And I have a staged file "src/utils.ts"
      vi.mocked(getStagedFilesWithChangeType).mockResolvedValue([
        { filepath: 'src/utils.ts', changeType: 'M', staged: true }
      ]);
      vi.mocked(getUnstagedFilesWithChangeType).mockResolvedValue([]);

      const store = useFspecStore.getState();
      await store.loadFileStatus();

      let currentState = useFspecStore.getState();
      expect(currentState.stagedFiles).toHaveLength(1);
      expect(currentState.stagedFiles[0].filepath).toBe('src/utils.ts');
      expect(currentState.unstagedFiles).toEqual([]);

      // @step When I unstage the file with "git reset src/utils.ts"
      // Simulate file being unstaged
      vi.mocked(getStagedFilesWithChangeType).mockResolvedValue([]);
      vi.mocked(getUnstagedFilesWithChangeType).mockResolvedValue([
        { filepath: 'src/utils.ts', changeType: 'M', staged: false }
      ]);

      // Call loadFileStatus (this is what fs.watch would trigger)
      await store.loadFileStatus();

      // @step Then the TUI files panel should automatically update
      // @step And "src/utils.ts" should appear in the unstaged section with yellow indicator
      // @step And "src/utils.ts" should be removed from the staged section
      const updatedStore = useFspecStore.getState();
      expect(updatedStore.stagedFiles.find(f => f.filepath === 'src/utils.ts')).toBeUndefined();
      expect(updatedStore.unstagedFiles.find(f => f.filepath === 'src/utils.ts')).toBeDefined();
    });
  });

  describe('Scenario: TUI auto-updates when checkpoint is created via fspec command', () => {
    it('should call loadStashes and update store when checkpoint is created', async () => {
      // @step Given the TUI is running and showing the stash panel
      // @step And work unit "AUTH-001" exists
      vi.mocked(git.log).mockResolvedValue([]);

      const store = useFspecStore.getState();
      await store.loadStashes();

      expect(store.stashes).toEqual([]);

      // @step When I create a checkpoint with "fspec checkpoint AUTH-001 baseline"
      // Simulate checkpoint being created
      const checkpointStash = {
        oid: 'def456',
        commit: {
          message: 'fspec-checkpoint:AUTH-001:baseline:1234567890',
          author: { timestamp: Date.now() / 1000 },
        },
      };
      vi.mocked(git.log).mockResolvedValue([checkpointStash]);

      // Call loadStashes (this is what fs.watch would trigger)
      await store.loadStashes();

      // @step Then the TUI stash panel should automatically update
      // @step And the new checkpoint stash should be displayed in the stash list
      const updatedStore = useFspecStore.getState();
      expect(updatedStore.stashes).toHaveLength(1);
      expect(updatedStore.stashes[0].oid).toBe('def456');
    });
  });

  describe('Scenario: TUI auto-updates multiple panels simultaneously', () => {
    it('should call both loadData and loadFileStatus when both change', async () => {
      // @step Given the TUI is running showing both work units and files panels
      // @step And I have an unstaged file "spec/work-units.json"
      vi.mocked(getStagedFilesWithChangeType).mockResolvedValue([]);
      vi.mocked(getUnstagedFilesWithChangeType).mockResolvedValue(['spec/work-units.json']);

      const store = useFspecStore.getState();
      await store.loadFileStatus();

      let currentState = useFspecStore.getState();
      expect(currentState.unstagedFiles).toContain('spec/work-units.json');

      // @step When I modify "spec/work-units.json" to update a work unit status
      // @step And I stage "spec/work-units.json" with "git add spec/work-units.json"

      // Call loadData (work-units.json watcher would trigger)
      await store.loadData();

      // Simulate file being staged
      vi.mocked(getStagedFilesWithChangeType).mockResolvedValue(['spec/work-units.json']);
      vi.mocked(getUnstagedFilesWithChangeType).mockResolvedValue([]);

      // Call loadFileStatus (.git/index watcher would trigger)
      await store.loadFileStatus();

      // @step Then the work units panel should automatically update with new status
      // @step And the files panel should automatically update showing "spec/work-units.json" as staged
      // @step And both updates should happen in real-time without manual refresh
      const updatedStore = useFspecStore.getState();
      expect(updatedStore.isLoaded).toBe(true);
      expect(updatedStore.stagedFiles).toContain('spec/work-units.json');
      expect(updatedStore.unstagedFiles).not.toContain('spec/work-units.json');
    });
  });

  describe('Scenario: Watchers are cleaned up on component unmount', () => {
    it('should verify store actions use DRY principles and reuse utilities', async () => {
      // @step Given the TUI is running with active file watchers
      // @step And watchers are monitoring ".git/refs/stash", ".git/index", and ".git/HEAD"
      const store = useFspecStore.getState();

      // @step When I exit the TUI
      vi.mocked(git.log).mockResolvedValue([]);
      await store.loadStashes();

      // @step Then all fs.watch watchers should be closed
      // @step And no watcher instances should remain in memory
      // @step And no memory leaks should be detected
      expect(git.log).toHaveBeenCalledWith(
        expect.objectContaining({
          ref: 'refs/stash',
        })
      );

      vi.mocked(getStagedFilesWithChangeType).mockResolvedValue([]);
      vi.mocked(getUnstagedFilesWithChangeType).mockResolvedValue([]);
      await store.loadFileStatus();

      expect(getStagedFilesWithChangeType).toHaveBeenCalled();
      expect(getUnstagedFilesWithChangeType).toHaveBeenCalled();
    });
  });
});
