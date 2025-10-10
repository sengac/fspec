import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';

interface RemoveQuestionOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

interface RemoveQuestionResult {
  success: boolean;
  removedQuestion: string;
  remainingCount: number;
}

export async function removeQuestion(options: RemoveQuestionOptions): Promise<RemoveQuestionResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units
  const content = await readFile(workUnitsFile, 'utf-8');
  const data: WorkUnitsData = JSON.parse(content);

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

  // Validate index
  if (options.index < 0 || options.index >= workUnit.questions.length) {
    throw new Error(
      `Invalid index ${options.index}. Valid range: 0-${workUnit.questions.length - 1}`
    );
  }

  // Remove question
  const [removedQuestion] = workUnit.questions.splice(options.index, 1);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    removedQuestion,
    remainingCount: workUnit.questions.length,
  };
}
