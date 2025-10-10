import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { WorkUnitsData } from '../types';

interface AnswerQuestionOptions {
  workUnitId: string;
  index: number;
  answer?: string;
  addTo?: 'rule' | 'assumption' | 'rules' | 'assumptions' | 'none';
  cwd?: string;
}

interface AnswerQuestionResult {
  success: boolean;
  removedQuestion: string;
  addedTo?: 'rules' | 'assumptions';
  addedContent?: string;
}

export async function answerQuestion(
  options: AnswerQuestionOptions
): Promise<AnswerQuestionResult> {
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
      `Can only answer questions during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Validate questions array exists
  if (!workUnit.questions || workUnit.questions.length === 0) {
    throw new Error(`Work unit ${options.workUnitId} has no questions`);
  }

  // Validate index
  if (options.index < 0 || options.index >= workUnit.questions.length) {
    throw new Error(
      `Invalid question index ${options.index}. Valid range: 0-${workUnit.questions.length - 1}`
    );
  }

  // Remove question
  const [removedQuestion] = workUnit.questions.splice(options.index, 1);

  let addedTo: 'rules' | 'assumptions' | undefined;
  let addedContent: string | undefined;

  // Add answer to appropriate array
  if (options.answer && options.addTo && options.addTo !== 'none') {
    if (options.addTo === 'rule' || options.addTo === 'rules') {
      if (!workUnit.rules) {
        workUnit.rules = [];
      }
      workUnit.rules.push(options.answer);
      addedTo = 'rules';
      addedContent = options.answer;
    } else if (options.addTo === 'assumption' || options.addTo === 'assumptions') {
      if (!workUnit.assumptions) {
        workUnit.assumptions = [];
      }
      workUnit.assumptions.push(options.answer);
      addedTo = 'assumptions';
      addedContent = options.answer;
    }
  }

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    removedQuestion,
    ...(addedTo && { addedTo }),
    ...(addedContent && { addedContent }),
  };
}
