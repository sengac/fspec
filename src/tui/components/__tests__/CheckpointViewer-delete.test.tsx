/**
 * Feature: spec/features/delete-checkpoint-or-all-checkpoints-from-checkpoint-viewer-with-confirmation.feature
 *
 * This test file validates checkpoint deletion acceptance criteria.
 * Tests use @step comments to map to Gherkin scenarios.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { CheckpointViewer } from '../CheckpointViewer';
import * as gitCheckpoint from '../../../utils/git-checkpoint';
import * as ipc from '../../../utils/ipc';
import fs from 'fs';
import { join } from 'path';

// Mock dependencies
vi.mock('../../../utils/git-checkpoint');
vi.mock('../../../utils/ipc');
vi.mock('fs');

describe('Feature: Delete checkpoint or all checkpoints from checkpoint viewer with confirmation', () => {
  const mockCwd = '/test/project';

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Delete single checkpoint with D key and Y/N confirmation', () => {
    it('should delete checkpoint after D key and Y confirmation', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And there is a checkpoint named "AUTH-001-baseline" selected
      const mockCheckpoints = [
        {
          name: 'AUTH-001-baseline',
          workUnitId: 'AUTH-001',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/AUTH-001/AUTH-001-baseline',
          isAutomatic: false,
          files: ['src/auth.ts'],
          fileCount: 1,
        },
        {
          name: 'AUTH-001-experiment',
          workUnitId: 'AUTH-001',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/AUTH-001/AUTH-001-experiment',
          isAutomatic: false,
          files: ['src/auth.ts'],
          fileCount: 1,
        },
      ];

      // Mock checkpoint loading
      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['AUTH-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({ name: cp.name, message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}` }))
        })
      );

      vi.mocked(gitCheckpoint.getCheckpointChangedFiles).mockResolvedValue(['src/auth.ts']);

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      // Wait for checkpoints to load
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I press the D key
      stdin.write('d');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then a yellow confirmation dialog should appear with message "Delete checkpoint 'AUTH-001-baseline'?"
      // @step And the dialog should use yesno confirmation mode
      // @step And the dialog should have medium risk level
      const output = lastFrame();
      expect(output).toContain('Delete checkpoint');
      expect(output).toContain('AUTH-001-baseline');

      // @step When I press the Y key
      stdin.write('y');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the checkpoint "AUTH-001-baseline" should be deleted
      // @step And the git ref "refs/fspec-checkpoints/AUTH-001/AUTH-001-baseline" should be removed
      // @step And the checkpoint entry should be removed from the index file
      // Implementation not yet complete - these will fail
      expect(vi.mocked(gitCheckpoint.deleteCheckpoint)).toHaveBeenCalledWith({
        workUnitId: 'AUTH-001',
        checkpointName: 'AUTH-001-baseline',
        cwd: mockCwd,
      });

      // @step And the viewer should select the next checkpoint in the list
      // Verify second checkpoint is now selected
      const finalOutput = lastFrame();
      expect(finalOutput).toBeTruthy();

      // @step And an IPC message with type "checkpoint-changed" should be sent
      expect(vi.mocked(ipc.sendIPCMessage)).toHaveBeenCalledWith({
        type: 'checkpoint-changed',
      });
    });
  });

  describe('Scenario: Cancel single checkpoint deletion with N key', () => {
    it('should cancel deletion when N key is pressed', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And there is a checkpoint named "AUTH-001-experiment" selected
      const mockCheckpoints = [
        {
          name: 'AUTH-001-experiment',
          workUnitId: 'AUTH-001',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/AUTH-001/AUTH-001-experiment',
          isAutomatic: false,
          files: ['src/auth.ts'],
          fileCount: 1,
        },
      ];

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['AUTH-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({ name: cp.name, message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}` }))
        })
      );
      vi.mocked(gitCheckpoint.getCheckpointChangedFiles).mockResolvedValue(['src/auth.ts']);

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I press the D key
      stdin.write('d');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then a yellow confirmation dialog should appear
      const output = lastFrame();
      expect(output).toContain('Delete checkpoint');

      // @step When I press the N key
      stdin.write('n');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the deletion should be cancelled
      expect(vi.mocked(gitCheckpoint.deleteCheckpoint)).not.toHaveBeenCalled();

      // @step And the checkpoint "AUTH-001-experiment" should still exist
      // @step And the viewer should return to normal state
      const finalOutput = lastFrame();
      expect(finalOutput).toContain('AUTH-001');
    });
  });

  describe('Scenario: Delete ALL checkpoints with Shift+D and typed confirmation', () => {
    it('should delete all checkpoints after typing DELETE ALL', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And there are 47 checkpoints for work unit "AUTH-001"
      const mockCheckpoints = Array.from({ length: 47 }, (_, i) => ({
        name: `AUTH-001-checkpoint-${i}`,
        workUnitId: 'AUTH-001',
        timestamp: new Date().toISOString(),
        stashRef: `refs/fspec-checkpoints/AUTH-001/AUTH-001-checkpoint-${i}`,
        isAutomatic: true,
        files: ['src/auth.ts'],
        fileCount: 1,
      }));

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['AUTH-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({ name: cp.name, message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}` }))
        })
      );
      vi.mocked(gitCheckpoint.getCheckpointChangedFiles).mockResolvedValue(['src/auth.ts']);

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I press Shift+D
      stdin.write('\x1b[1;2D'); // Shift+D escape sequence (this may need adjustment)
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then a red confirmation dialog should appear with message "Delete ALL 47 checkpoints for AUTH-001?"
      // @step And the dialog should use typed confirmation mode
      // @step And the dialog should require typing "DELETE ALL"
      // @step And the dialog should have high risk level
      const output = lastFrame();
      expect(output).toContain('Delete ALL');
      expect(output).toContain('47 checkpoints');

      // @step When I type "DELETE ALL" and press Enter
      for (const char of 'DELETE ALL') {
        stdin.write(char);
      }
      stdin.write('\r');
      await new Promise(resolve => setTimeout(resolve, 200));

      // @step Then all 47 checkpoints for "AUTH-001" should be deleted
      // @step And all git refs under "refs/fspec-checkpoints/AUTH-001/" should be removed
      // @step And the index file for "AUTH-001" should be deleted
      expect(vi.mocked(gitCheckpoint.deleteAllCheckpoints)).toHaveBeenCalledWith({
        workUnitId: 'AUTH-001',
        cwd: mockCwd,
      });

      // @step And the viewer should exit to the board
      expect(onExit).toHaveBeenCalled();

      // @step And an IPC message with type "checkpoint-changed" should be sent
      expect(vi.mocked(ipc.sendIPCMessage)).toHaveBeenCalledWith({
        type: 'checkpoint-changed',
      });
    });
  });

  describe('Scenario: Delete last remaining checkpoint exits to board', () => {
    it('should exit to board when deleting the only checkpoint', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And there is only 1 checkpoint remaining
      const mockCheckpoints = [
        {
          name: 'AUTH-001-last',
          workUnitId: 'AUTH-001',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/AUTH-001/AUTH-001-last',
          isAutomatic: false,
          files: ['src/auth.ts'],
          fileCount: 1,
        },
      ];

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['AUTH-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({ name: cp.name, message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}` }))
        })
      );
      vi.mocked(gitCheckpoint.getCheckpointChangedFiles).mockResolvedValue(['src/auth.ts']);

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I press the D key
      stdin.write('d');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And I press the Y key to confirm
      stdin.write('y');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the last checkpoint should be deleted
      expect(vi.mocked(gitCheckpoint.deleteCheckpoint)).toHaveBeenCalled();

      // @step And the viewer should automatically exit to the board
      expect(onExit).toHaveBeenCalled();
    });
  });

  describe('Scenario: Cancel delete ALL with ESC key', () => {
    it('should cancel delete ALL when ESC is pressed', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And there are 50 checkpoints for work unit "TUI-001"
      const mockCheckpoints = Array.from({ length: 50 }, (_, i) => ({
        name: `TUI-001-checkpoint-${i}`,
        workUnitId: 'TUI-001',
        timestamp: new Date().toISOString(),
        stashRef: `refs/fspec-checkpoints/TUI-001/TUI-001-checkpoint-${i}`,
        isAutomatic: true,
        files: ['src/tui.ts'],
        fileCount: 1,
      }));

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['TUI-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({ name: cp.name, message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}` }))
        })
      );
      vi.mocked(gitCheckpoint.getCheckpointChangedFiles).mockResolvedValue(['src/tui.ts']);

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I press Shift+D
      stdin.write('\x1b[1;2D');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then a red confirmation dialog should appear
      const output = lastFrame();
      expect(output).toContain('Delete ALL');

      // @step When I press the ESC key
      stdin.write('\x1b');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the deletion should be cancelled
      expect(vi.mocked(gitCheckpoint.deleteAllCheckpoints)).not.toHaveBeenCalled();

      // @step And all 50 checkpoints should still exist
      // @step And the viewer should return to normal state
      const finalOutput = lastFrame();
      expect(finalOutput).toContain('TUI-001');
    });
  });
});
