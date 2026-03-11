/**
 * Feature: spec/features/checkpoint-viewer-loading.feature
 *
 * This test file validates that the CheckpointViewer loads checkpoint metadata
 * eagerly but defers expensive diff-file computation until a checkpoint is
 * actually selected (lazy loading). This prevents the TUI from hanging when
 * there are many checkpoints.
 *
 * @step comments map to the loading contract, not a Gherkin feature file.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { CheckpointViewer } from '../CheckpointViewer';
import * as gitCheckpoint from '../../../utils/git-checkpoint';
import * as checkpointIndex from '../../../utils/checkpoint-index';
import * as ipc from '../../../utils/ipc';
import { resolveRef } from '@sengac/codelet-napi';
import { join } from 'path';

// Mock dependencies
vi.mock('../../../utils/git-checkpoint');
vi.mock('../../../utils/checkpoint-index');
vi.mock('../../../utils/ipc');
vi.mock('@sengac/codelet-napi');

// Import store for proper mocking
import { useFspecStore } from '../../store/fspecStore';

describe('Feature: Checkpoint viewer lazy loading', () => {
  const mockCwd = '/test/project';

  beforeEach(() => {
    vi.clearAllMocks();

    useFspecStore.setState({
      stagedFiles: [],
      unstagedFiles: [],
      workUnits: [],
      epics: [],
      stashes: [],
      isLoaded: false,
      error: null,
      cwd: mockCwd,
    });

    vi.mocked(ipc.sendIPCMessage).mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Checkpoint metadata loads without computing diff files', () => {
    it('should NOT call getCheckpointFilesChangedFromHead for non-selected checkpoints during initial load', async () => {
      const onExit = vi.fn();
      const now = Date.now();

      // @step Given there are 50 checkpoints across 5 work units
      const workUnits = ['AUTH-001', 'TUI-001', 'BUG-002', 'GIT-003', 'HOOK-004'];
      const indexFiles = workUnits.map(wu => `${wu}.json`);

      vi.mocked(checkpointIndex.listCheckpointIndexFiles).mockResolvedValue(indexFiles);

      // Each work unit has 10 checkpoints
      for (const wu of workUnits) {
        vi.mocked(checkpointIndex.readCheckpointIndexFile).mockResolvedValueOnce({
          checkpoints: Array.from({ length: 10 }, (_, i) => ({
            name: `${wu}-checkpoint-${i}`,
            message: `fspec-checkpoint:${wu}:${wu}-checkpoint-${i}:${now - i * 1000}`,
          })),
        });
      }

      vi.mocked(resolveRef).mockReturnValue('mock-oid');

      // @step And getCheckpointFilesChangedFromHead is mocked to track calls
      const diffFilesMock = vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead);
      diffFilesMock.mockResolvedValue(['src/file.ts']);

      // @step When the checkpoint viewer renders
      render(React.createElement(CheckpointViewer, { onExit }));

      // @step And we wait for the initial metadata load to complete
      await new Promise(resolve => setTimeout(resolve, 150));

      // @step Then getCheckpointFilesChangedFromHead should only be called
      //       for the first selected checkpoint (lazy load), not all 50
      // The first checkpoint in sorted order gets auto-selected, so exactly 1 call
      expect(diffFilesMock.mock.calls.length).toBeLessThanOrEqual(1);

      // @step And resolveRef should be called for all 50 checkpoints (it's fast metadata)
      expect(vi.mocked(resolveRef).mock.calls.length).toBe(50);
    });
  });

  describe('Scenario: Diff files are loaded lazily when a checkpoint is selected', () => {
    it('should call getCheckpointFilesChangedFromHead only for the selected checkpoint', async () => {
      const onExit = vi.fn();
      const now = Date.now();

      // @step Given there are 3 checkpoints
      vi.mocked(checkpointIndex.listCheckpointIndexFiles).mockResolvedValue(['AUTH-001.json']);
      vi.mocked(checkpointIndex.readCheckpointIndexFile).mockResolvedValue({
        checkpoints: [
          { name: 'cp-a', message: `fspec-checkpoint:AUTH-001:cp-a:${now}` },
          { name: 'cp-b', message: `fspec-checkpoint:AUTH-001:cp-b:${now - 1000}` },
          { name: 'cp-c', message: `fspec-checkpoint:AUTH-001:cp-c:${now - 2000}` },
        ],
      });

      vi.mocked(resolveRef).mockReturnValue('mock-oid');

      const diffFilesMock = vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead);
      diffFilesMock.mockResolvedValue(['src/file.ts']);

      // @step When the viewer renders
      const { lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      // @step And we wait for initial load + first checkpoint's lazy file load
      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then only 1 call should have been made (for the auto-selected first checkpoint)
      expect(diffFilesMock).toHaveBeenCalledTimes(1);
      expect(diffFilesMock).toHaveBeenCalledWith(mockCwd, 'AUTH-001', 'cp-a');
    });
  });

  describe('Scenario: Viewer shows checkpoints list immediately without waiting for diff computation', () => {
    it('should exit the loading state and show checkpoint names before diff files are loaded', async () => {
      const onExit = vi.fn();
      const now = Date.now();

      // @step Given there are 2 checkpoints
      vi.mocked(checkpointIndex.listCheckpointIndexFiles).mockResolvedValue(['AUTH-001.json']);
      vi.mocked(checkpointIndex.readCheckpointIndexFile).mockResolvedValue({
        checkpoints: [
          { name: 'my-baseline', timestamp: new Date(now).toISOString() },
          { name: 'my-experiment', timestamp: new Date(now - 1000).toISOString() },
        ],
      });

      vi.mocked(resolveRef).mockReturnValue('mock-oid');

      // @step And getCheckpointFilesChangedFromHead is artificially slow
      let resolveDiffFiles: ((value: string[]) => void) | null = null;
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockReturnValue(
        new Promise(resolve => {
          resolveDiffFiles = resolve;
        })
      );

      // @step When the viewer renders
      const { lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      // @step And we wait for metadata to load (but NOT for diff files)
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the viewer should show checkpoint names (not "Loading checkpoints...")
      const output = lastFrame();
      expect(output).not.toContain('Loading checkpoints...');
      expect(output).toContain('my-baseline');

      // @step And the files pane should show "Loading files..."
      expect(output).toContain('Loading files...');

      // @step When the diff file computation completes
      resolveDiffFiles!(['src/auth.ts', 'src/config.ts']);
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the files pane should show the actual files
      const updatedOutput = lastFrame();
      expect(updatedOutput).toContain('src/auth.ts');
      expect(updatedOutput).not.toContain('Loading files...');
    });
  });

  describe('Scenario: New checkpoint format with sha and timestamp fields loads correctly', () => {
    it('should handle checkpoint entries with sha+timestamp instead of legacy message field', async () => {
      const onExit = vi.fn();

      // @step Given a checkpoint uses the new format (sha + timestamp, no message)
      vi.mocked(checkpointIndex.listCheckpointIndexFiles).mockResolvedValue(['GIT-039.json']);
      vi.mocked(checkpointIndex.readCheckpointIndexFile).mockResolvedValue({
        checkpoints: [
          {
            name: 'post-isomorphic-git-removal',
            sha: '298649891e5b4c931f6bddd342ca6446d010d27a',
            timestamp: '2026-03-11T08:52:55.782Z',
          },
        ],
      });

      vi.mocked(resolveRef).mockReturnValue('298649891e5b4c931f6bddd342ca6446d010d27a');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue([
        'src/utils/git-checkpoint.ts',
      ]);

      // @step When the viewer renders
      const { lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then the checkpoint should appear with its name
      const output = lastFrame();
      expect(output).toContain('post-isomorphic-git-removal');
      // @step And it should NOT show "Loading checkpoints..." or error
      expect(output).not.toContain('Loading checkpoints...');
      expect(output).not.toContain('ERROR');
      expect(output).not.toContain('No checkpoints');
    });
  });

  describe('Scenario: Legacy checkpoint format with message field loads correctly', () => {
    it('should handle checkpoint entries with legacy message field', async () => {
      const onExit = vi.fn();
      const epoch = 1762578801117;

      // @step Given a checkpoint uses the legacy format (message with encoded timestamp)
      vi.mocked(checkpointIndex.listCheckpointIndexFiles).mockResolvedValue(['AGENT-001.json']);
      vi.mocked(checkpointIndex.readCheckpointIndexFile).mockResolvedValue({
        checkpoints: [
          {
            name: 'AGENT-001-auto-specifying',
            message: `fspec-checkpoint:AGENT-001:AGENT-001-auto-specifying:${epoch}`,
          },
        ],
      });

      vi.mocked(resolveRef).mockReturnValue('5a62912b69a7cff10d37c4ca0ec1a630f5247851');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue([
        'src/agent.ts',
      ]);

      // @step When the viewer renders
      const { lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then the checkpoint should appear (auto-checkpoints show "AGENT-001: Specifying")
      const output = lastFrame();
      expect(output).toContain('AGENT-001');
      expect(output).not.toContain('Loading checkpoints...');
      expect(output).not.toContain('ERROR');
      expect(output).not.toContain('No checkpoints');
    });
  });

  describe('Scenario: Checkpoint with unresolvable ref is silently skipped', () => {
    it('should skip checkpoints whose git ref cannot be resolved', async () => {
      const onExit = vi.fn();

      // @step Given one checkpoint has a valid ref and one has a dangling ref
      vi.mocked(checkpointIndex.listCheckpointIndexFiles).mockResolvedValue(['TEST-001.json']);
      vi.mocked(checkpointIndex.readCheckpointIndexFile).mockResolvedValue({
        checkpoints: [
          { name: 'good-checkpoint', timestamp: new Date().toISOString() },
          { name: 'dangling-checkpoint', timestamp: new Date().toISOString() },
        ],
      });

      // First call resolves, second throws (dangling ref)
      vi.mocked(resolveRef)
        .mockReturnValueOnce('valid-oid')
        .mockImplementationOnce(() => {
          throw new Error('reference not found');
        });

      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue([
        'src/test.ts',
      ]);

      // @step When the viewer renders
      const { lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then only the good checkpoint should appear
      const output = lastFrame();
      expect(output).toContain('good-checkpoint');
      expect(output).not.toContain('dangling-checkpoint');
      expect(output).not.toContain('ERROR');
    });
  });

  describe('Scenario: Empty checkpoint index directory', () => {
    it('should show "No checkpoints available" when no index files exist', async () => {
      const onExit = vi.fn();

      // @step Given the checkpoint index directory is empty
      vi.mocked(checkpointIndex.listCheckpointIndexFiles).mockResolvedValue([]);

      // @step When the viewer renders
      const { lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then it should show "No checkpoints available"
      const output = lastFrame();
      expect(output).toContain('No checkpoints');

      // @step And getCheckpointFilesChangedFromHead should never be called
      expect(gitCheckpoint.getCheckpointFilesChangedFromHead).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Failed diff file load shows empty file list', () => {
    it('should show empty files when getCheckpointFilesChangedFromHead throws', async () => {
      const onExit = vi.fn();

      // @step Given a checkpoint exists but diff computation will fail
      vi.mocked(checkpointIndex.listCheckpointIndexFiles).mockResolvedValue(['ERR-001.json']);
      vi.mocked(checkpointIndex.readCheckpointIndexFile).mockResolvedValue({
        checkpoints: [
          { name: 'broken-diff', timestamp: new Date().toISOString() },
        ],
      });

      vi.mocked(resolveRef).mockReturnValue('valid-oid');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockRejectedValue(
        new Error('NAPI diff computation failed')
      );

      // @step When the viewer renders and selects the checkpoint
      const { lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then the checkpoint name should still appear
      const output = lastFrame();
      expect(output).toContain('broken-diff');
      // @step And the files pane should not show "Loading files..."
      expect(output).not.toContain('Loading files...');
    });
  });
});
