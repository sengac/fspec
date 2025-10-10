import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';

interface AddQuestionOptions {
  workUnitId: string;
  question: string;
  cwd?: string;
}

interface AddQuestionResult {
  success: boolean;
  questionCount: number;
  mentionedPeople?: string[];
}

export async function addQuestion(options: AddQuestionOptions): Promise<AddQuestionResult> {
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
      `Can only add questions during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Initialize questions array if it doesn't exist
  if (!workUnit.questions) {
    workUnit.questions = [];
  }

  // Add question
  workUnit.questions.push(options.question);

  // Extract mentioned people (@mentions)
  const mentionedPeople = (options.question.match(/@\w+/g) || []).map(mention => mention.slice(1));

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    questionCount: workUnit.questions.length,
    ...(mentionedPeople.length > 0 && { mentionedPeople }),
  };
}
