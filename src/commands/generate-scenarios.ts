import { writeFile, mkdir } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join, dirname } from 'path';
import { existsSync } from 'fs';
import type { WorkUnitsData, QuestionItem } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { readFile } from 'fs/promises';
import {
  getUnansweredQuestionsReminder,
  getEmptyExampleMappingReminder,
  getPostGenerationReminder,
} from '../utils/system-reminder';
import { extractStepsFromExample } from '../utils/step-extraction';
import { detectPrefill } from '../utils/prefill-detection';

interface GenerateScenariosOptions {
  workUnitId: string;
  feature?: string;
  template?: 'given-when-then' | 'basic';
  cwd?: string;
}

interface GenerateScenariosResult {
  success: boolean;
  featureFile: string;
  scenariosCount: number;
  systemReminders?: string[];
}

function generateBasicScenario(
  workUnitId: string,
  example: string,
  index: number
): string {
  const steps = extractStepsFromExample(example);

  return `  Scenario: ${example}
    Given ${steps.given || '[precondition]'}
    When ${steps.when || '[action]'}
    Then ${steps.then || '[expected outcome]'}
`;
}

function generateGivenWhenThenScenario(
  workUnitId: string,
  example: string,
  index: number
): string {
  // Parse example to extract Given/When/Then if structured that way
  // For now, use example as scenario name with placeholder steps
  return `  Scenario: ${example}
    Given [precondition from example]
    When [action from example]
    Then [expected outcome from example]
`;
}

export async function generateScenarios(
  options: GenerateScenariosOptions
): Promise<GenerateScenariosResult> {
  const cwd = options.cwd || process.cwd();

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Check for unanswered questions (BEFORE generation)
  let unansweredCount = 0;
  if (workUnit.questions && workUnit.questions.length > 0) {
    unansweredCount = workUnit.questions.filter(q => {
      const questionItem = q as QuestionItem;
      return !questionItem.selected;
    }).length;

    if (unansweredCount > 0) {
      const reminder = getUnansweredQuestionsReminder(
        options.workUnitId,
        unansweredCount
      );
      if (reminder) {
        throw new Error(
          `Cannot generate scenarios: ${unansweredCount} unanswered question${
            unansweredCount > 1 ? 's' : ''
          } found.\n\n${reminder}\n\nAnswer questions with 'fspec answer-question ${options.workUnitId} <index>' before generating.`
        );
      }
    }
  }

  // Check for empty Example Mapping
  const hasRules = workUnit.rules && workUnit.rules.length > 0;
  const hasExamples = workUnit.examples && workUnit.examples.length > 0;
  if (!hasRules && !hasExamples) {
    const reminder = getEmptyExampleMappingReminder(
      options.workUnitId,
      hasRules,
      hasExamples
    );
    if (reminder) {
      throw new Error(
        `Cannot generate scenarios: No Example Mapping data found.\n\n${reminder}\n\nComplete Example Mapping before generating scenarios.`
      );
    }
  }

  // Validate examples exist
  if (!workUnit.examples || workUnit.examples.length === 0) {
    throw new Error(
      `Work unit ${options.workUnitId} has no examples to generate scenarios from`
    );
  }

  // Determine feature file path
  let featureFile: string;
  if (options.feature) {
    // Remove .feature extension if provided
    const featureName = options.feature.replace(/\.feature$/, '');
    featureFile = join(cwd, 'spec/features', `${featureName}.feature`);
  } else {
    // Default: use work unit title (capability-based naming)
    if (!workUnit.title) {
      throw new Error(
        `Cannot determine feature file name. Work unit ${options.workUnitId} has no title.\n` +
          `Suggestion: Use --feature flag with a capability-based name (e.g., --feature=user-authentication)`
      );
    }
    // Convert title to kebab-case for feature file name
    const kebabCase = workUnit.title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '');
    featureFile = join(cwd, 'spec/features', `${kebabCase}.feature`);
  }

  // Ensure directory exists
  await mkdir(dirname(featureFile), { recursive: true });

  // Generate scenarios from examples
  const template = options.template || 'basic';
  const scenarios = workUnit.examples.map((example, index) => {
    if (template === 'given-when-then') {
      return generateGivenWhenThenScenario(options.workUnitId, example, index);
    }
    return generateBasicScenario(options.workUnitId, example, index);
  });

  // Check if feature file exists
  const fileExists = existsSync(featureFile);

  if (fileExists) {
    // Append to existing feature file
    const existingContent = await readFile(featureFile, 'utf-8');
    const updatedContent =
      existingContent.trimEnd() + '\n\n' + scenarios.join('\n') + '\n';
    await writeFile(featureFile, updatedContent);
  } else {
    // Create new feature file
    const title = workUnit.title || options.workUnitId;

    // Generate Background section from userStory if available
    let backgroundSection: string;
    if (workUnit.userStory) {
      backgroundSection = `  Background: User Story
    As a ${workUnit.userStory.role}
    I want to ${workUnit.userStory.action}
    So that ${workUnit.userStory.benefit}`;
    } else {
      backgroundSection = `  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]`;
    }

    const featureContent = `@${options.workUnitId}
Feature: ${title}

${backgroundSection}

${scenarios.join('\n')}`;
    await writeFile(featureFile, featureContent);
  }

  // Check for prefill in generated/updated file
  const finalContent = await readFile(featureFile, 'utf-8');
  const prefillResult = detectPrefill(finalContent);

  // Generate post-generation reminder
  const systemReminders: string[] = [];
  const postGenReminder = getPostGenerationReminder(
    options.workUnitId,
    featureFile
  );
  if (postGenReminder) {
    systemReminders.push(postGenReminder);
  }

  // Add prefill reminder if detected
  if (prefillResult.hasPrefill && prefillResult.systemReminder) {
    systemReminders.push(prefillResult.systemReminder);
  }

  return {
    success: true,
    featureFile,
    scenariosCount: scenarios.length,
    ...(systemReminders.length > 0 && { systemReminders }),
  };
}

export function registerGenerateScenariosCommand(program: Command): void {
  program
    .command('generate-scenarios')
    .description('Generate Gherkin scenarios from example mapping in work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .option(
      '--feature <name>',
      'Feature file name (without .feature extension). Defaults to work unit title in kebab-case.'
    )
    .action(async (workUnitId: string, options: { feature?: string }) => {
      try {
        const result = await generateScenarios({
          workUnitId,
          feature: options.feature,
        });
        console.log(
          chalk.green(
            `✓ Generated ${result.scenariosCount} scenarios in ${result.featureFile}`
          )
        );
        // Display system reminders if any
        if (result.systemReminders && result.systemReminders.length > 0) {
          for (const reminder of result.systemReminders) {
            console.log('\n' + reminder);
          }
        }
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to generate scenarios:'),
          error.message
        );
        process.exit(1);
      }
    });
}
