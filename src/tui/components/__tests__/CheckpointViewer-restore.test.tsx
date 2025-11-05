/**
 * Feature: spec/features/restore-individual-files-or-all-files-from-checkpoint-in-checkpoint-viewer.feature
 *
 * This test file validates checkpoint restore acceptance criteria.
 * Tests use @step comments to map to Gherkin scenarios.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import React from 'react';
import { CheckpointViewer } from '../CheckpointViewer';
import * as gitCheckpoint from '../../../utils/git-checkpoint';
import * as ipc from '../../../utils/ipc';
import * as git from 'isomorphic-git';
import fs from 'fs';
import { join } from 'path';

// Mock dependencies
vi.mock('../../../utils/git-checkpoint');
vi.mock('../../../utils/ipc');
vi.mock('isomorphic-git');
vi.mock('fs');

describe('Feature: Restore individual files or all files from checkpoint in checkpoint viewer', () => {
  const mockCwd = '/test/project';

  beforeEach(() => {
    vi.clearAllMocks();

    // Mock sendIPCMessage
    vi.mocked(ipc.sendIPCMessage).mockResolvedValue(undefined);

    // Set up default mock for restoreCheckpointFile if it exists
    try {
      (gitCheckpoint.restoreCheckpointFile as any) = vi.fn().mockResolvedValue({
        success: true,
        conflictDetected: false,
        systemReminder: '',
      });
    } catch (e) {
      // Function doesn't exist yet, will be mocked in individual tests
    }

    // Set up default mock for restoreCheckpoint if it exists
    try {
      (gitCheckpoint.restoreCheckpoint as any) = vi.fn().mockResolvedValue({
        success: true,
        conflictsDetected: false,
        conflictedFiles: [],
        systemReminder: '',
        requiresTestValidation: false,
      });
    } catch (e) {
      // Function doesn't exist yet, will be mocked in individual tests
    }
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Restore single file from checkpoint with R key', () => {
    it('should restore single file when R key pressed and confirmed', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And a checkpoint "AUTH-001-baseline" is selected with 3 files
      const mockCheckpoints = [
        {
          name: 'AUTH-001-baseline',
          workUnitId: 'AUTH-001',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/AUTH-001/AUTH-001-baseline',
          isAutomatic: false,
          files: ['src/auth.ts', 'src/login.ts', 'src/utils.ts'],
          fileCount: 3,
        },
      ];

      // Mock checkpoint loading
      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['AUTH-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({
            name: cp.name,
            message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}`,
          })),
        })
      );

      vi.mocked(git.resolveRef).mockResolvedValue('mock-checkpoint-oid-123');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue([
        'src/auth.ts',
        'src/login.ts',
        'src/utils.ts',
      ]);

      // Mock restoreCheckpointFile (NEW function to be implemented)
      vi.mocked(gitCheckpoint.restoreCheckpointFile).mockResolvedValue({
        success: true,
        conflictDetected: false,
        systemReminder: '',
      });

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      // Wait for checkpoints to load
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the files pane is focused with "src/auth.ts" selected
      // Navigate to files pane (Tab key)
      stdin.write('\t');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step When I press the R key
      stdin.write('r');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then a confirmation dialog should appear with message "Restore src/auth.ts from checkpoint 'AUTH-001-baseline'?"
      // @step And the dialog should use yesno confirmation mode
      // @step And the dialog should have medium risk level
      const output = lastFrame();
      expect(output).toContain('Restore src/auth.ts');
      expect(output).toContain('AUTH-001-baseline');

      // @step When I press the Y key to confirm
      stdin.write('y');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the file "src/auth.ts" should be restored from the checkpoint
      expect(gitCheckpoint.restoreCheckpointFile).toHaveBeenCalledWith(
        expect.objectContaining({
          checkpointOid: 'mock-checkpoint-oid-123',
          filepath: 'src/auth.ts',
          force: true,
        })
      );

      // @step And the diff pane should refresh showing new comparison
      // @step And a success message should be displayed
      // @step And the viewer should stay open
      expect(onExit).not.toHaveBeenCalled();

      // @step And an IPC message with type "checkpoint-changed" should be sent
      expect(vi.mocked(ipc.sendIPCMessage)).toHaveBeenCalledWith({
        type: 'checkpoint-changed',
      });
    });
  });

  describe('Scenario: Restore all files from checkpoint with T key', () => {
    it('should restore all files when T key pressed and confirmed', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And a checkpoint "TUI-001-auto-testing" is selected with 15 files
      const mockFiles = Array.from({ length: 15 }, (_, i) => `src/file${i}.ts`);
      const mockCheckpoints = [
        {
          name: 'TUI-001-auto-testing',
          workUnitId: 'TUI-001',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/TUI-001/TUI-001-auto-testing',
          isAutomatic: true,
          files: mockFiles,
          fileCount: 15,
        },
      ];

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['TUI-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({
            name: cp.name,
            message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}`,
          })),
        })
      );

      vi.mocked(git.resolveRef).mockResolvedValue('mock-checkpoint-oid-456');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue(mockFiles);

      // Mock restoreCheckpointFile (now called for each file to show progress)
      vi.mocked(gitCheckpoint.restoreCheckpointFile).mockResolvedValue({
        success: true,
        conflictDetected: false,
        systemReminder: '',
      });

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I press the T key
      stdin.write('t');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then a confirmation dialog should appear with message "Restore ALL 15 files from checkpoint 'TUI-001-auto-testing'?"
      // @step And the dialog should use yesno confirmation mode
      // @step And the dialog should have high risk level
      const output = lastFrame();
      expect(output).toContain('Restore ALL');
      expect(output).toContain('15 files');
      expect(output).toContain('TUI-001-auto-testing');

      // @step When I press the Y key to confirm
      stdin.write('y');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then all 15 files should be restored from the checkpoint (one by one for progress)
      expect(gitCheckpoint.restoreCheckpointFile).toHaveBeenCalledTimes(15);
      // Verify each file was restored
      mockFiles.forEach(filepath => {
        expect(gitCheckpoint.restoreCheckpointFile).toHaveBeenCalledWith(
          expect.objectContaining({
            checkpointOid: 'mock-checkpoint-oid-456',
            filepath,
            force: true,
          })
        );
      });

      // @step And the diff pane should refresh showing new comparison
      // @step And a success message should be displayed
      // @step And the viewer should stay open
      expect(onExit).not.toHaveBeenCalled();

      // @step And an IPC message with type "checkpoint-changed" should be sent
      expect(vi.mocked(ipc.sendIPCMessage)).toHaveBeenCalledWith({
        type: 'checkpoint-changed',
      });
    });
  });

  describe('Scenario: Restore deleted file creates file and parent directories', () => {
    it('should create file and directories when restoring deleted file', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And a checkpoint "REFACTOR-003" is selected
      const mockCheckpoints = [
        {
          name: 'REFACTOR-003',
          workUnitId: 'REFACTOR',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/REFACTOR/REFACTOR-003',
          isAutomatic: false,
          files: ['src/deleted-file.ts'],
          fileCount: 1,
        },
      ];

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['REFACTOR.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({
            name: cp.name,
            message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}`,
          })),
        })
      );

      vi.mocked(git.resolveRef).mockResolvedValue('mock-checkpoint-oid-789');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue(['src/deleted-file.ts']);

      // @step And the file "src/deleted-file.ts" exists in checkpoint but not in working directory
      vi.mocked(gitCheckpoint.restoreCheckpointFile).mockResolvedValue({
        success: true,
        conflictDetected: false,
        systemReminder: '',
      });

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the files pane is focused with "src/deleted-file.ts" selected
      stdin.write('\t'); // Navigate to files pane
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step When I press the R key
      stdin.write('r');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And I confirm the restoration
      stdin.write('y');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the file "src/deleted-file.ts" should be created
      // @step And all parent directories should be created automatically
      expect(gitCheckpoint.restoreCheckpointFile).toHaveBeenCalledWith(
        expect.objectContaining({
          filepath: 'src/deleted-file.ts',
          force: true,
        })
      );

      // @step And the diff pane should refresh showing the restored file
      // @step And a success message should be displayed
      const finalOutput = lastFrame();
      expect(finalOutput).toBeTruthy();
    });
  });

  describe('Scenario: Restore file with uncommitted changes shows warning', () => {
    it('should show high-risk warning when restoring file with uncommitted changes', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And a checkpoint "CONFIG-005" is selected
      const mockCheckpoints = [
        {
          name: 'CONFIG-005',
          workUnitId: 'CONFIG',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/CONFIG/CONFIG-005',
          isAutomatic: false,
          files: ['src/config.ts'],
          fileCount: 1,
        },
      ];

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['CONFIG.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({
            name: cp.name,
            message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}`,
          })),
        })
      );

      vi.mocked(git.resolveRef).mockResolvedValue('mock-checkpoint-oid-999');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue(['src/config.ts']);

      // @step And the file "src/config.ts" has uncommitted changes in working directory
      // Mock conflict detection
      vi.mocked(gitCheckpoint.restoreCheckpointFile).mockResolvedValueOnce({
        success: false,
        conflictDetected: true,
        systemReminder: 'Warning: uncommitted changes will be lost',
      });

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the files pane is focused with "src/config.ts" selected
      stdin.write('\t');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step When I press the R key
      stdin.write('r');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then a confirmation dialog should appear with message "Overwrite src/config.ts? Current changes will be LOST."
      // @step And the dialog should have high risk level
      // @step And the dialog should show warning about data loss
      const output = lastFrame();
      expect(output).toContain('Overwrite');
      expect(output).toContain('src/config.ts');
      expect(output).toContain('LOST');

      // @step When I press the Y key to confirm
      // Mock successful restore after force
      vi.mocked(gitCheckpoint.restoreCheckpointFile).mockResolvedValueOnce({
        success: true,
        conflictDetected: false,
        systemReminder: '',
      });

      stdin.write('y');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the file "src/config.ts" should be overwritten with checkpoint version
      // @step And the uncommitted changes should be lost
      expect(gitCheckpoint.restoreCheckpointFile).toHaveBeenCalledWith(
        expect.objectContaining({
          filepath: 'src/config.ts',
          force: true,
        })
      );

      // @step And the diff pane should refresh showing new comparison
      // @step And a success message should be displayed
      expect(onExit).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Cancel single file restore with N key', () => {
    it('should cancel restoration when N key pressed', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And a checkpoint "AUTH-001-baseline" is selected
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
      ];

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['AUTH-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({
            name: cp.name,
            message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}`,
          })),
        })
      );

      vi.mocked(git.resolveRef).mockResolvedValue('mock-checkpoint-oid-123');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue(['src/auth.ts']);

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the files pane is focused with "src/auth.ts" selected
      stdin.write('\t');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step When I press the R key
      stdin.write('r');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then a confirmation dialog should appear
      const output = lastFrame();
      expect(output).toContain('Restore');

      // @step When I press the N key
      stdin.write('n');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the restoration should be cancelled
      expect(vi.mocked(gitCheckpoint.restoreCheckpointFile)).not.toHaveBeenCalled();

      // @step And the file "src/auth.ts" should not be modified
      // @step And the viewer should return to normal state
      const finalOutput = lastFrame();
      expect(finalOutput).toContain('AUTH-001');
    });
  });

  describe('Scenario: Cancel restore all files with N key', () => {
    it('should cancel restore all when N key pressed', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And a checkpoint "TUI-001-auto-testing" is selected with 15 files
      const mockFiles = Array.from({ length: 15 }, (_, i) => `src/file${i}.ts`);
      const mockCheckpoints = [
        {
          name: 'TUI-001-auto-testing',
          workUnitId: 'TUI-001',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/TUI-001/TUI-001-auto-testing',
          isAutomatic: true,
          files: mockFiles,
          fileCount: 15,
        },
      ];

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['TUI-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({
            name: cp.name,
            message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}`,
          })),
        })
      );

      vi.mocked(git.resolveRef).mockResolvedValue('mock-checkpoint-oid-456');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue(mockFiles);

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step When I press the T key
      stdin.write('t');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then a confirmation dialog should appear
      const output = lastFrame();
      expect(output).toContain('Restore ALL');

      // @step When I press the N key
      stdin.write('n');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then the restoration should be cancelled
      expect(vi.mocked(gitCheckpoint.restoreCheckpoint)).not.toHaveBeenCalled();

      // @step And no files should be modified
      // @step And the viewer should return to normal state
      const finalOutput = lastFrame();
      expect(finalOutput).toContain('TUI-001');
    });
  });

  describe('Scenario: R key only works when files pane is focused', () => {
    it('should not show restore dialog when R pressed in checkpoints pane', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And a checkpoint "AUTH-001-baseline" is selected
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
      ];

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['AUTH-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({
            name: cp.name,
            message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}`,
          })),
        })
      );

      vi.mocked(git.resolveRef).mockResolvedValue('mock-checkpoint-oid-123');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue(['src/auth.ts']);

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the checkpoints pane is focused
      // (checkpoints pane is focused by default)

      // @step When I press the R key
      stdin.write('r');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then no restore dialog should appear
      const output = lastFrame();
      expect(output).not.toContain('Restore src/auth.ts');

      // @step And the viewer should remain unchanged
      expect(vi.mocked(gitCheckpoint.restoreCheckpointFile)).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: T key only works when checkpoints or files pane is focused', () => {
    it('should not show restore dialog when T pressed in diff pane', async () => {
      const onExit = vi.fn();

      // @step Given I am in the checkpoint viewer
      // @step And a checkpoint "TUI-001-auto-testing" is selected
      const mockCheckpoints = [
        {
          name: 'TUI-001-auto-testing',
          workUnitId: 'TUI-001',
          timestamp: new Date().toISOString(),
          stashRef: 'refs/fspec-checkpoints/TUI-001/TUI-001-auto-testing',
          isAutomatic: true,
          files: ['src/file.ts'],
          fileCount: 1,
        },
      ];

      vi.mocked(fs.existsSync).mockReturnValue(true);
      vi.mocked(fs.readdirSync).mockReturnValue(['TUI-001.json'] as any);
      vi.mocked(fs.readFileSync).mockReturnValue(
        JSON.stringify({
          checkpoints: mockCheckpoints.map(cp => ({
            name: cp.name,
            message: `fspec-checkpoint:${cp.workUnitId}:${cp.name}:${Date.now()}`,
          })),
        })
      );

      vi.mocked(git.resolveRef).mockResolvedValue('mock-checkpoint-oid-456');
      vi.mocked(gitCheckpoint.getCheckpointFilesChangedFromHead).mockResolvedValue(['src/file.ts']);

      const { stdin, lastFrame } = render(
        React.createElement(CheckpointViewer, { onExit })
      );

      await new Promise(resolve => setTimeout(resolve, 100));

      // @step And the diff pane is focused
      // Navigate to diff pane (Tab twice: checkpoints -> files -> diff)
      stdin.write('\t');
      await new Promise(resolve => setTimeout(resolve, 50));
      stdin.write('\t');
      await new Promise(resolve => setTimeout(resolve, 50));

      // @step When I press the T key
      stdin.write('t');
      await new Promise(resolve => setTimeout(resolve, 100));

      // @step Then no restore dialog should appear
      const output = lastFrame();
      expect(output).not.toContain('Restore ALL');

      // @step And the viewer should remain unchanged
      expect(vi.mocked(gitCheckpoint.restoreCheckpoint)).not.toHaveBeenCalled();
    });
  });
});
