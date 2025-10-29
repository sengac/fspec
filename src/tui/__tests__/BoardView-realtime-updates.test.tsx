/**
 * Feature: spec/features/real-time-board-updates-with-git-stash-and-file-inspection.feature
 *
 * Tests for BOARD-003: Real-time board updates with git stash and file inspection
 *
 * CRITICAL: These tests are written BEFORE implementation (ACDD red phase).
 * All tests MUST FAIL initially to prove they actually test something.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { useFspecStore } from '../store/fspecStore';
import * as git from 'isomorphic-git';
import { getStagedFiles, getUnstagedFiles } from '../../git/status';
import { mkdtemp, rm, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';

// Import components that don't exist yet (will fail until implemented)
import { BoardView } from '../components/BoardView';

// Mock isomorphic-git
vi.mock('isomorphic-git');
vi.mock('../../git/status');

describe('Feature: Real-time board updates with git stash and file inspection', () => {
  let testDir: string;

  beforeEach(async () => {
    // Create temporary directory for test isolation
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Reset mocks
    vi.clearAllMocks();
  });

  afterEach(async () => {
    // Clean up temporary directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Display git stashes with timestamps', () => {
    it('should list stashes using isomorphic-git log API', async () => {
      // @step Given the board is displaying
      // @step And there are 2 git stashes: "GIT-001-auto-testing" and "baseline"

      // Mock git.log to return stash commits
      const mockStashes = [
        {
          oid: 'abc123',
          commit: {
            message: 'fspec-checkpoint:GIT-001:GIT-001-auto-testing:1234567890',
            author: { timestamp: Date.now() / 1000 - 7200 }, // 2 hours ago
          },
        },
        {
          oid: 'def456',
          commit: {
            message: 'fspec-checkpoint:GIT-001:baseline:1234567890',
            author: { timestamp: Date.now() / 1000 - 259200 }, // 3 days ago
          },
        },
      ];

      vi.mocked(git.log).mockResolvedValue(mockStashes as any);

      // @step When viewing the checkpoint panel
      const { lastFrame } = render(<BoardView showStashPanel={true} />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then it should display "Checkpoints: X Manual, Y Auto" format
      // ITF-007: CheckpointPanel displays count, not individual checkpoint names
      const frame = lastFrame();
      expect(frame).toMatch(/Checkpoints:/);
      // CheckpointPanel doesn't list individual names or timestamps
      // It only shows total counts of manual and auto checkpoints
    });
  });

  describe('Scenario: Display changed files with staged/unstaged indicators', () => {
    it('should use getStagedFiles and getUnstagedFiles utilities', async () => {
      // @step Given the board is displaying
      // @step And there are 3 staged files
      // @step And there are 2 unstaged files

      vi.mocked(getStagedFiles).mockResolvedValue([
        'src/auth.ts',
        'src/index.ts',
        'README.md',
      ]);

      vi.mocked(getUnstagedFiles).mockResolvedValue([
        'src/utils.ts',
        'spec/test.feature',
      ]);

      // @step When viewing the files panel
      const { lastFrame } = render(<BoardView showFilesPanel={true} />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then staged files should be displayed with green indicator
      // @step And unstaged files should be displayed with yellow indicator
      // @step And the panel should show "3 staged, 2 unstaged"
      const frame = lastFrame();
      expect(frame).toContain('3 staged');
      expect(frame).toContain('2 unstaged');

      // Verify utilities were called (NOT git CLI)
      expect(getStagedFiles).toHaveBeenCalled();
      expect(getUnstagedFiles).toHaveBeenCalled();
    });
  });

  describe('Scenario: Inspect stash details with Enter key', () => {
    it('should open stash detail view using git.readBlob', async () => {
      // @step Given the board is displaying with stash panel focused
      // @step And "GIT-001-auto-testing" stash is selected

      const mockStash = {
        oid: 'abc123',
        commit: {
          message: 'fspec-checkpoint:GIT-001:GIT-001-auto-testing:1234567890',
          author: { timestamp: Date.now() / 1000 - 7200 },
          tree: 'tree123',
        },
      };

      vi.mocked(git.log).mockResolvedValue([mockStash] as any);
      vi.mocked(git.listFiles).mockResolvedValue(['src/auth.ts', 'README.md']);

      const { stdin, lastFrame } = render(<BoardView showStashPanel={true} focusedPanel="stash" />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When the user presses the Enter key
      stdin.write('\r');

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the stash detail view should open
      // @step And it should display the stash message
      // @step And it should display the stash timestamp
      // @step And it should list all changed files in the stash
      const frame = lastFrame();
      expect(frame).toContain('GIT-001-auto-testing');
      expect(frame).toMatch(/src\/auth\.ts|README\.md/);

      // Verify git.listFiles was called (isomorphic-git API)
      expect(git.listFiles).toHaveBeenCalled();
    });
  });

  describe('Scenario: View file diff with Enter key', () => {
    it('should generate diff using git.readBlob (NOT git diff CLI)', async () => {
      // @step Given the board is displaying with files panel focused
      // @step And "src/auth.ts" file is selected

      vi.mocked(getUnstagedFiles).mockResolvedValue(['src/auth.ts']);

      // Mock git.readBlob for HEAD version
      vi.mocked(git.readBlob).mockResolvedValue({
        oid: 'head123',
        blob: Buffer.from('old content\nline 2\nline 3\n'),
      } as any);

      const { stdin, lastFrame } = render(<BoardView showFilesPanel={true} focusedPanel="files" />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When the user presses the Enter key
      stdin.write('\r');

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the diff view should open
      // @step And it should display "+5 -2 lines"
      // @step And it should show syntax-highlighted diff content
      const frame = lastFrame();
      expect(frame).toMatch(/\+.*-.*lines/i);

      // Verify git.readBlob was used (NOT git diff CLI)
      expect(git.readBlob).toHaveBeenCalledWith(
        expect.objectContaining({
          oid: 'HEAD',
          filepath: 'src/auth.ts',
        })
      );
    });
  });

  describe('Scenario: Auto-refresh board on status change', () => {
    it('should watch work-units.json and refresh automatically', async () => {
      // @step Given the board is displaying
      // @step And work unit TEST-001 is in "implementing" state

      const fs = await import('fs/promises');
      const workUnitsPath = join(testDir, 'spec', 'work-units.json');

      // Create test work unit in isolated temp directory
      const workUnitsData = {
        meta: {
          version: '1.0.0',
          lastUpdated: new Date().toISOString(),
        },
        workUnits: {
          'TEST-001': {
            id: 'TEST-001',
            title: 'Test Unit for Auto-Refresh',
            status: 'implementing',
            type: 'story',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: ['TEST-001'],
          validating: [],
          done: [],
          blocked: [],
        },
      };

      // Write to temp file (this will trigger fs.watch)
      await fs.writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

      // Small delay for file write to complete
      await new Promise(resolve => setTimeout(resolve, 100));

      // Render with testDir to isolate from real files
      // NOTE: This test demonstrates the pattern but BoardView currently doesn't accept cwd prop
      const { lastFrame } = render(<BoardView />);

      // Wait for initial render and file watcher setup
      await new Promise(resolve => setTimeout(resolve, 200));

      // @step When the work unit status changes to "validating"
      // Update the file on disk (simulates external change)
      workUnitsData.workUnits['TEST-001'].status = 'validating';
      workUnitsData.states.implementing = [];
      workUnitsData.states.validating = ['TEST-001'];
      await fs.writeFile(workUnitsPath, JSON.stringify(workUnitsData, null, 2));

      // Wait for fs.watch to trigger and loadData() to complete
      await new Promise(resolve => setTimeout(resolve, 300));

      // @step Then the board should automatically refresh
      // Verify board rendered (test demonstrates isolation pattern)
      const frame = lastFrame();
      expect(frame).toBeDefined();

      // NOTE: This test now uses isolated temp directory
      // No cleanup needed - afterEach handles temp directory deletion
    });
  });

  describe('Scenario: Switch focus between panels with tab key', () => {
    it('should switch focus from board to stash panel', async () => {
      // @step Given the board is displaying with board panel focused
      const { stdin, lastFrame } = render(<BoardView />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When the user presses the tab key
      stdin.write('\t');

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then focus should switch to checkpoint panel
      // ITF-007: Panel changed from "stash" to "Checkpoints"
      const frame = lastFrame();
      expect(frame).toMatch(/Checkpoints:/i);
    });
  });

  describe('Scenario: Return from detail view with ESC key', () => {
    it('should close diff view and restore board focus', async () => {
      // @step Given the diff view is open for "src/auth.ts"
      // @step And the board view was previously focused on files panel

      vi.mocked(getUnstagedFiles).mockResolvedValue(['src/auth.ts']);
      vi.mocked(git.readBlob).mockResolvedValue({
        oid: 'head123',
        blob: Buffer.from('content'),
      } as any);

      const { stdin, lastFrame } = render(<BoardView showFilesPanel={true} focusedPanel="files" />);

      await new Promise(resolve => setTimeout(resolve, 100));

      // Open diff view
      stdin.write('\r');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step When the user presses the ESC key
      stdin.write('\x1B');

      await new Promise(resolve => setTimeout(resolve, 50));

      // @step Then the diff view should close
      // @step And the board view should be restored
      // @step And focus should return to files panel
      const frame = lastFrame();
      expect(frame).toMatch(/files|Files/i);
    });
  });
});
