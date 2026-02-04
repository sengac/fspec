/**
 * Cleanup old checkpoints, keeping most recent N
 */

import chalk from 'chalk';
import type { Command } from 'commander';
import { cleanupCheckpoints as cleanupCheckpointsUtil } from '../utils/git-checkpoint';
import { sendIPCMessage } from '../utils/ipc';

import { output } from '../utils/output';
export interface CleanupCheckpointsOptions {
  workUnitId: string;
  keepLast: number;
  cwd: string;
}

export async function cleanupCheckpoints(
  options: CleanupCheckpointsOptions
): Promise<{
  deletedCount: number;
  preservedCount: number;
  summary: {
    deleted: Array<{ name: string; timestamp: string }>;
    preserved: Array<{ name: string; timestamp: string }>;
  };
}> {
  const { workUnitId, keepLast, cwd } = options;

  try {
    const result = await cleanupCheckpointsUtil(workUnitId, cwd, keepLast);

    output.log(
      chalk.cyan(
        `\nCleaning up checkpoints for ${workUnitId} (keeping last ${keepLast})...\n`
      )
    );

    if (result.deletedCount > 0) {
      output.log(chalk.red(`Deleted ${result.deletedCount} checkpoint(s):`));
      result.deleted.forEach(cp => {
        output.log(chalk.gray(`  - ${cp.name} (${cp.timestamp})`));
      });
      output.log('');
    }

    if (result.preservedCount > 0) {
      output.log(
        chalk.green(`Preserved ${result.preservedCount} checkpoint(s):`)
      );
      result.preserved.forEach(cp => {
        output.log(chalk.gray(`  - ${cp.name} (${cp.timestamp})`));
      });
      output.log('');
    }

    output.log(
      chalk.green(
        `✓ Cleanup complete: ${result.deletedCount} deleted, ${result.preservedCount} preserved`
      )
    );

    // Notify TUI of checkpoint change via IPC
    await sendIPCMessage({ type: 'checkpoint-changed' });

    return {
      deletedCount: result.deletedCount,
      preservedCount: result.preservedCount,
      summary: {
        deleted: result.deleted.map(cp => ({
          name: cp.name,
          timestamp: cp.timestamp,
        })),
        preserved: result.preserved.map(cp => ({
          name: cp.name,
          timestamp: cp.timestamp,
        })),
      },
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    output.error(chalk.red(`✗ Failed to cleanup checkpoints: ${errorMessage}`));
    throw error;
  }
}

async function cleanupCheckpointsCommand(
  workUnitId: string,
  options: { keepLast: string }
): Promise<void> {
  try {
    const keepLast = parseInt(options.keepLast, 10);

    if (isNaN(keepLast) || keepLast < 1) {
      throw new Error('--keep-last must be a positive number');
    }

    await cleanupCheckpoints({
      workUnitId,
      keepLast,
      cwd: process.cwd(),
    });

    process.exit(0);
  } catch (error: unknown) {
    if (error instanceof Error) {
      output.error(chalk.red('Error:'), error.message);
    } else {
      output.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}

export function registerCleanupCheckpointsCommand(program: Command): void {
  program
    .command('cleanup-checkpoints')
    .description('Cleanup old checkpoints, keeping most recent N')
    .argument('<work-unit-id>', 'Work unit ID (e.g., AUTH-001)')
    .requiredOption('--keep-last <number>', 'Number of checkpoints to keep')
    .action(cleanupCheckpointsCommand);
}
