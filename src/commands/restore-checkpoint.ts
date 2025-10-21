/**
 * Restore checkpoint with conflict detection and AI-assisted resolution
 */

import chalk from 'chalk';
import {
  restoreCheckpoint as restoreCheckpointUtil,
  isWorkingDirectoryDirty,
} from '../utils/git-checkpoint.js';

export interface RestoreCheckpointOptions {
  workUnitId: string;
  checkpointName: string;
  cwd: string;
  workingDirectoryDirty?: boolean;
  userChoice?: string;
}

export interface RestoreCheckpointResult {
  success: boolean;
  conflictsDetected: boolean;
  conflictedFiles: string[];
  systemReminder: string;
  requiresTestValidation: boolean;
  promptShown?: boolean;
  options?: Array<{
    name: string;
    riskLevel: string;
    description: string;
  }>;
  requiresUserChoice?: boolean;
}

export async function restoreCheckpoint(
  options: RestoreCheckpointOptions
): Promise<RestoreCheckpointResult> {
  const { workUnitId, checkpointName, cwd, workingDirectoryDirty, userChoice } = options;

  try {
    // Check if working directory is dirty
    const isDirty = workingDirectoryDirty ?? (await isWorkingDirectoryDirty(cwd));

    if (isDirty && !userChoice) {
      // Show interactive prompt with risk explanations
      const promptOptions = [
        {
          name: 'Commit changes first',
          riskLevel: 'Low',
          description: 'Safest option. Commits current changes before restoration.',
        },
        {
          name: 'Stash changes and restore',
          riskLevel: 'Medium',
          description:
            'Temporarily saves changes. Can restore later, but may cause conflicts.',
        },
        {
          name: 'Force restore with merge',
          riskLevel: 'High',
          description:
            'Attempts to merge changes. May result in conflicts requiring manual resolution.',
        },
      ];

      console.log(
        chalk.yellow('⚠️  Working directory has uncommitted changes')
      );
      console.log(chalk.cyan('\nChoose how to proceed:'));
      promptOptions.forEach((opt, idx) => {
        const riskColor =
          opt.riskLevel === 'Low'
            ? chalk.green
            : opt.riskLevel === 'Medium'
              ? chalk.yellow
              : chalk.red;
        console.log(
          `  ${idx + 1}. ${opt.name} ${riskColor(`[${opt.riskLevel} risk]`)}`
        );
        console.log(chalk.gray(`     ${opt.description}`));
      });

      return {
        success: false,
        conflictsDetected: false,
        conflictedFiles: [],
        systemReminder: 'User choice required',
        requiresTestValidation: false,
        promptShown: true,
        options: promptOptions,
        requiresUserChoice: true,
      };
    }

    // Restore checkpoint
    const result = await restoreCheckpointUtil({
      workUnitId,
      checkpointName,
      cwd,
      force: isDirty && userChoice === 'Force restore with merge',
    });

    if (result.conflictsDetected) {
      console.error(chalk.red('✗ Merge conflicts detected during restoration'));
      console.log(chalk.yellow('\nConflicted files:'));
      result.conflictedFiles.forEach((file) => {
        console.log(chalk.yellow(`  - ${file}`));
      });
      console.log(
        chalk.cyan(
          '\n💡 Resolve conflicts using Read and Edit tools, then run tests'
        )
      );

      // Emit system-reminder for AI
      if (result.systemReminder) {
        console.log(result.systemReminder);
      }
    } else {
      console.log(
        chalk.green(`✓ Restored checkpoint "${checkpointName}" for ${workUnitId}`)
      );
    }

    return {
      success: result.success,
      conflictsDetected: result.conflictsDetected,
      conflictedFiles: result.conflictedFiles,
      systemReminder: result.systemReminder,
      requiresTestValidation: result.requiresTestValidation,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error(chalk.red(`✗ Failed to restore checkpoint: ${errorMessage}`));
    throw error;
  }
}
