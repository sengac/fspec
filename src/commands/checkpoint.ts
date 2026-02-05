/**
 * Create manual checkpoint for experimentation
 */

import chalk from 'chalk';
import type { Command } from 'commander';
import { createCheckpoint as createCheckpointUtil } from '../utils/git-checkpoint';
import { sendIPCMessage } from '../utils/ipc';

import { output } from '../utils/output';
export interface CheckpointOptions {
  workUnitId: string;
  checkpointName: string;
  cwd: string;
}

export async function checkpoint(options: CheckpointOptions): Promise<{
  success: boolean;
  checkpointName: string;
  stashMessage: string;
  includedUntracked: boolean;
  capturedFiles: string[];
}> {
  const { workUnitId, checkpointName, cwd } = options;

  try {
    const result = await createCheckpointUtil({
      workUnitId,
      checkpointName,
      cwd,
      includeUntracked: true,
    });

    output.log(`✓ Created checkpoint "${checkpointName}" for ${workUnitId}`);
    output.log(`  Captured ${result.capturedFiles.length} file(s)`);

    // Notify TUI of checkpoint change via IPC
    await sendIPCMessage({ type: 'checkpoint-changed' });

    return {
      success: result.success,
      checkpointName: result.checkpointName,
      stashMessage: result.stashMessage,
      includedUntracked: result.includedUntracked,
      capturedFiles: result.capturedFiles,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    output.error(`✗ Failed to create checkpoint: ${errorMessage}`);
    throw error;
  }
}

async function checkpointCommand(
  workUnitId: string,
  checkpointName: string
): Promise<void> {
  try {
    const result = await checkpoint({
      workUnitId,
      checkpointName,
      cwd: process.cwd(),
    });

    if (result.success) {
      process.exit(0);
    } else {
      process.exit(1);
    }
  } catch (error: unknown) {
    if (error instanceof Error) {
      output.error('Error:', error.message);
    } else {
      output.error('Error: Unknown error occurred');
    }
    process.exit(1);
  }
}

export function registerCheckpointCommand(program: Command): void {
  program
    .command('checkpoint')
    .description('Create a manual checkpoint for safe experimentation')
    .argument('<work-unit-id>', 'Work unit ID (e.g., AUTH-001)')
    .argument(
      '<checkpoint-name>',
      'Checkpoint name (e.g., baseline, before-refactor)'
    )
    .action(checkpointCommand);
}
