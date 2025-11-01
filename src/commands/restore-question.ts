import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

interface RestoreQuestionOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

interface RestoreQuestionResult {
  success: boolean;
  restoredQuestion: string;
  activeCount: number;
  message?: string; // For idempotent operations
}

export async function restoreQuestion(
  options: RestoreQuestionOptions
): Promise<RestoreQuestionResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Validate work unit is in specifying state
  if (workUnit.status !== 'specifying') {
    throw new Error(
      `Can only restore questions during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Validate questions array exists
  if (!workUnit.questions || workUnit.questions.length === 0) {
    throw new Error(`Work unit ${options.workUnitId} has no questions`);
  }

  // Find question by ID (index is now treated as ID for stable indices)
  const question = workUnit.questions.find(q => q.id === options.index);

  if (!question) {
    throw new Error(`Question with ID ${options.index} not found`);
  }

  // If already active, return idempotent success
  if (!question.deleted) {
    return {
      success: true,
      restoredQuestion: question.text,
      activeCount: workUnit.questions.filter(q => !q.deleted).length,
      message: `Item ID ${options.index} already active`,
    };
  }

  // Restore: clear deleted flag and timestamp
  question.deleted = false;
  delete question.deletedAt;

  const restoredQuestion = question.text;

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    restoredQuestion,
    activeCount: workUnit.questions.filter(q => !q.deleted).length,
  };
}

export function registerRestoreQuestionCommand(program: Command): void {
  program
    .command('restore-question')
    .description('Restore a soft-deleted question by ID')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<index>', 'Question ID (0-based)')
    .action(async (workUnitId: string, index: string) => {
      try {
        const result = await restoreQuestion({
          workUnitId,
          index: parseInt(index, 10),
        });
        console.log(
          chalk.green(`✓ Restored question: "${result.restoredQuestion}"`)
        );
        if (result.message) {
          console.log(chalk.dim(`  ${result.message}`));
        }
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to restore question:'),
          error.message
        );
        process.exit(1);
      }
    });
}
