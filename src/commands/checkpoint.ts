/**
 * Create manual checkpoint for experimentation
 */

import chalk from 'chalk';
import { createCheckpoint as createCheckpointUtil } from '../utils/git-checkpoint.js';

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
}> {
  const { workUnitId, checkpointName, cwd } = options;

  try {
    const result = await createCheckpointUtil({
      workUnitId,
      checkpointName,
      cwd,
      includeUntracked: true,
    });

    console.log(
      chalk.green(`✓ Created checkpoint "${checkpointName}" for ${workUnitId}`)
    );
    console.log(
      chalk.gray(`  Captured ${result.capturedFiles.length} file(s)`)
    );

    return {
      success: result.success,
      checkpointName: result.checkpointName,
      stashMessage: result.stashMessage,
      includedUntracked: result.includedUntracked,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error(chalk.red(`✗ Failed to create checkpoint: ${errorMessage}`));
    throw error;
  }
}
