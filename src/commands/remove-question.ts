import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

import { output } from '../utils/output';
interface RemoveQuestionOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

interface RemoveQuestionResult {
  success: boolean;
  removedQuestion: string;
  remainingCount: number;
  message?: string; // For idempotent operations
}

export async function removeQuestion(
  options: RemoveQuestionOptions
): Promise<RemoveQuestionResult> {
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
      `Can only remove questions during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
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

  // If already deleted, return idempotent success
  if (question.deleted) {
    return {
      success: true,
      removedQuestion: question.text,
      remainingCount: workUnit.questions.filter(q => !q.deleted).length,
      message: `Item ID ${options.index} already deleted`,
    };
  }

  // Soft-delete: set deleted flag and timestamp
  question.deleted = true;
  question.deletedAt = new Date().toISOString();

  const removedQuestion = question.text;

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    removedQuestion,
    remainingCount: workUnit.questions.filter(q => !q.deleted).length,
  };
}

export function registerRemoveQuestionCommand(program: Command): void {
  program
    .command('remove-question')
    .description('Remove a question from a work unit by index')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<index>', 'Question index (0-based)')
    .action(async (workUnitId: string, index: string) => {
      try {
        const result = await removeQuestion({
          workUnitId,
          index: parseInt(index, 10),
        });
        output.log(
          chalk.green(`✓ Removed question: "${result.removedQuestion}"`)
        );
      } catch (error: any) {
        output.error('✗ Failed to remove question:', error.message);
        process.exit(1);
      }
    });
}
