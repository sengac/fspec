import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import * as lockfile from 'proper-lockfile';
import type { WorkUnitsData, QuestionItem } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface AnswerQuestionOptions {
  workUnitId: string;
  index: number;
  answer?: string;
  addTo?: 'rule' | 'assumption' | 'rules' | 'assumptions' | 'none';
  cwd?: string;
}

interface AnswerQuestionResult {
  success: boolean;
  question: string;
  addedTo?: 'rules' | 'assumptions';
  addedContent?: string;
}

export async function answerQuestion(
  options: AnswerQuestionOptions
): Promise<AnswerQuestionResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Ensure file exists first
  await ensureWorkUnitsFile(cwd);

  // Acquire lock to prevent concurrent modifications
  const release = await lockfile.lock(workUnitsFile, {
    retries: {
      retries: 10,
      minTimeout: 10,
      maxTimeout: 100,
    },
  });

  try {
    // Read work units with lock held
    const data: WorkUnitsData = JSON.parse(
      await readFile(workUnitsFile, 'utf-8')
    );

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

    // Get the question (must be QuestionItem format)
    const question = workUnit.questions[options.index] as QuestionItem;

    if (!question || typeof question === 'string') {
      throw new Error(
        'Question format is invalid. Expected QuestionItem object.'
      );
    }

    const questionText = question.text;

    // Mark question as selected and add answer
    question.selected = true;
    if (options.answer) {
      question.answer = options.answer;
    }

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
      } else if (
        options.addTo === 'assumption' ||
        options.addTo === 'assumptions'
      ) {
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
      question: questionText,
      ...(addedTo && { addedTo }),
      ...(addedContent && { addedContent }),
    };
  } finally {
    // Always release the lock
    await release();
  }
}

export function registerAnswerQuestionCommand(program: Command): void {
  program
    .command('answer-question')
    .description('Answer a question and optionally add to rules or assumptions')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<index>', 'Question index (0-based)')
    .option('--answer <answer>', 'Answer text')
    .option('--add-to <type>', 'Add answer to: rule, assumption, or none', 'none')
    .action(
      async (
        workUnitId: string,
        index: string,
        options: { answer?: string; addTo?: string }
      ) => {
        try {
          const result = await answerQuestion({
            workUnitId,
            index: parseInt(index, 10),
            answer: options.answer,
            addTo: options.addTo as 'rule' | 'assumption' | 'none',
          });
          console.log(chalk.green(`✓ Answered question: "${result.question}"`));
          if (options.answer) {
            console.log(chalk.dim(`  Answer: "${options.answer}"`));
          }
          if (result.addedTo && result.addedContent) {
            console.log(
              chalk.cyan(`  Added to ${result.addedTo}: "${result.addedContent}"`)
            );
          }
        } catch (error: any) {
          console.error(chalk.red('✗ Failed to answer question:'), error.message);
          process.exit(1);
        }
      }
    );
}
