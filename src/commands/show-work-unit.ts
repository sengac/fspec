import { readFile } from 'fs/promises';
import { join } from 'path';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import type { WorkUnitsData, QuestionItem } from '../types';
import { extractWorkUnitTags } from '../utils/work-unit-tags';
import {
  getMissingEstimateReminder,
  getEmptyExampleMappingReminder,
  getLongDurationReminder,
} from '../utils/system-reminder';

interface ShowWorkUnitOptions {
  workUnitId: string;
  output?: 'json' | 'text';
  cwd?: string;
}

interface LinkedFeature {
  file: string;
  scenarios: Array<{
    name: string;
    line: number;
    file: string;
  }>;
}

interface WorkUnitDetails {
  id: string;
  title: string;
  status: string;
  description?: string;
  estimate?: number;
  epic?: string;
  parent?: string;
  children?: string[];
  blockedBy?: string[];
  rules?: string[];
  examples?: string[];
  questions?: string[];
  assumptions?: string[];
  createdAt: string;
  updatedAt: string;
  linkedFeatures?: LinkedFeature[];
  systemReminders?: string[];
  [key: string]: unknown;
}

export async function showWorkUnit(
  options: ShowWorkUnitOptions
): Promise<WorkUnitDetails> {
  const cwd = options.cwd || process.cwd();

  // Read work units
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
  const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);

  // Check if work unit exists
  if (!workUnitsData.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[options.workUnitId];

  // Scan feature files for linked scenarios
  const linkedFeatures: LinkedFeature[] = [];

  try {
    // Find all feature files
    const featureFiles = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: false,
    });

    // Parse each feature file and check for work unit tags
    for (const featureFile of featureFiles) {
      const featurePath = join(cwd, featureFile);
      const featureContent = await readFile(featurePath, 'utf-8');

      // Parse Gherkin
      const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      let gherkinDocument;
      try {
        gherkinDocument = parser.parse(featureContent);
      } catch {
        // Skip invalid feature files
        continue;
      }

      // Extract work unit tags
      const workUnitTags = extractWorkUnitTags(gherkinDocument);

      // Find scenarios for this work unit
      const matchingTag = workUnitTags.find(
        tag => tag.id === options.workUnitId
      );

      if (matchingTag && matchingTag.scenarios.length > 0) {
        linkedFeatures.push({
          file: featureFile,
          scenarios: matchingTag.scenarios.map(scenario => ({
            name: scenario.name,
            line: scenario.line,
            file: featureFile,
          })),
        });
      }
    }
  } catch {
    // If features directory doesn't exist, just return empty linked features
  }

  // Filter questions to show only unselected ones, and extract text
  let unselectedQuestions: string[] | undefined;
  if (workUnit.questions && workUnit.questions.length > 0) {
    unselectedQuestions = workUnit.questions
      .map((q, index) => {
        if (typeof q === 'string') {
          throw new Error(
            'Invalid question format. Questions must be QuestionItem objects.'
          );
        }
        return { index, ...q };
      })
      .filter(q => !q.selected)
      .map(q => `[${q.index}] ${q.text}`);

    if (unselectedQuestions.length === 0) {
      unselectedQuestions = undefined;
    }
  }

  // Generate system reminders
  const systemReminders: string[] = [];

  // Check for missing estimate
  const hasEstimate = workUnit.estimate !== undefined;
  const estimateReminder = getMissingEstimateReminder(
    options.workUnitId,
    hasEstimate
  );
  if (estimateReminder) {
    systemReminders.push(estimateReminder);
  }

  // Check for empty Example Mapping in specifying phase
  if (workUnit.status === 'specifying') {
    const hasRules = workUnit.rules && workUnit.rules.length > 0;
    const hasExamples = workUnit.examples && workUnit.examples.length > 0;
    const exampleMappingReminder = getEmptyExampleMappingReminder(
      options.workUnitId,
      hasRules,
      hasExamples
    );
    if (exampleMappingReminder) {
      systemReminders.push(exampleMappingReminder);
    }
  }

  // Check for long duration in current phase
  if (workUnit.stateHistory && workUnit.stateHistory.length > 0) {
    const lastStateChange =
      workUnit.stateHistory[workUnit.stateHistory.length - 1];
    const durationMs =
      Date.now() - new Date(lastStateChange.timestamp).getTime();
    const durationHours = durationMs / (1000 * 60 * 60);

    const durationReminder = getLongDurationReminder(
      options.workUnitId,
      workUnit.status as any,
      durationHours
    );
    if (durationReminder) {
      systemReminders.push(durationReminder);
    }
  }

  return {
    id: workUnit.id,
    title: workUnit.title,
    status: workUnit.status,
    ...(workUnit.description && { description: workUnit.description }),
    ...(workUnit.estimate !== undefined && { estimate: workUnit.estimate }),
    ...(workUnit.epic && { epic: workUnit.epic }),
    ...(workUnit.parent && { parent: workUnit.parent }),
    ...(workUnit.children && { children: workUnit.children }),
    ...(workUnit.blockedBy && { blockedBy: workUnit.blockedBy }),
    ...(workUnit.rules && { rules: workUnit.rules }),
    ...(workUnit.examples && { examples: workUnit.examples }),
    ...(unselectedQuestions && { questions: unselectedQuestions }),
    ...(workUnit.assumptions && { assumptions: workUnit.assumptions }),
    createdAt: workUnit.createdAt,
    updatedAt: workUnit.updatedAt,
    linkedFeatures,
    ...(systemReminders.length > 0 && { systemReminders }),
  };
}

// CLI wrapper function for Commander.js
export async function showWorkUnitCommand(
  workUnitId: string,
  options: { format?: string }
): Promise<void> {
  const chalk = await import('chalk').then(m => m.default);
  try {
    const result = await showWorkUnit({ workUnitId, format: options.format });

    if (options.format === 'json') {
      console.log(JSON.stringify(result, null, 2));
    } else {
      console.log(chalk.bold(`\n${result.id}`));
      console.log(chalk.gray(`Status: ${result.status}`));
      console.log('');
      console.log(chalk.bold(result.title));
      if (result.description) {
        console.log(chalk.gray(result.description));
      }
      console.log('');

      if (result.epic) {
        console.log(chalk.gray('Epic:'), result.epic);
      }
      if (result.parent) {
        console.log(chalk.gray('Parent:'), result.parent);
      }
      if (result.children && result.children.length > 0) {
        console.log(chalk.gray('Children:'), result.children.join(', '));
      }

      if (result.rules && result.rules.length > 0) {
        console.log(chalk.cyan('\nRules:'));
        result.rules.forEach((rule, idx) => {
          console.log(`  ${idx + 1}. ${rule}`);
        });
      }

      if (result.examples && result.examples.length > 0) {
        console.log(chalk.cyan('\nExamples:'));
        result.examples.forEach((example, idx) => {
          console.log(`  ${idx + 1}. ${example}`);
        });
      }

      if (result.questions && result.questions.length > 0) {
        console.log(chalk.cyan('\nQuestions:'));
        result.questions.forEach(question => {
          console.log(`  ${question}`);
        });
      }

      if (result.assumptions && result.assumptions.length > 0) {
        console.log(chalk.cyan('\nAssumptions:'));
        result.assumptions.forEach((assumption, idx) => {
          console.log(`  ${idx + 1}. ${assumption}`);
        });
      }

      if (result.linkedFeatures && result.linkedFeatures.length > 0) {
        console.log(chalk.cyan('\nLinked Features:'));
        for (const feature of result.linkedFeatures) {
          console.log(`\n  ${chalk.bold(feature.file)}`);
          for (const scenario of feature.scenarios) {
            console.log(
              `    ${chalk.gray(`${scenario.file}:${scenario.line}`)} - ${scenario.name}`
            );
          }
        }
      }

      console.log('');
      console.log(
        chalk.gray('Created:'),
        new Date(result.createdAt).toLocaleString()
      );
      console.log(
        chalk.gray('Updated:'),
        new Date(result.updatedAt).toLocaleString()
      );
      console.log('');

      // Display system reminders if any
      if (result.systemReminders && result.systemReminders.length > 0) {
        for (const reminder of result.systemReminders) {
          console.log(reminder);
          console.log('');
        }
      }
    }

    process.exit(0);
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error(chalk.red('Error:'), error.message);
    } else {
      console.error(chalk.red('Error: Unknown error occurred'));
    }
    process.exit(1);
  }
}
