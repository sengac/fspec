import { writeFile, mkdir } from 'fs/promises';
import { join, dirname } from 'path';
import { existsSync } from 'fs';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { readFile } from 'fs/promises';

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
}

function generateBasicScenario(workUnitId: string, example: string, index: number): string {
  return `  @${workUnitId}
  Scenario: ${example}
    Given [precondition]
    When [action]
    Then [expected outcome]
`;
}

function generateGivenWhenThenScenario(
  workUnitId: string,
  example: string,
  index: number
): string {
  // Parse example to extract Given/When/Then if structured that way
  // For now, use example as scenario name with placeholder steps
  return `  @${workUnitId}
  Scenario: ${example}
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

  // Validate examples exist
  if (!workUnit.examples || workUnit.examples.length === 0) {
    throw new Error(`Work unit ${options.workUnitId} has no examples to generate scenarios from`);
  }

  // Determine feature file path
  let featureFile: string;
  if (options.feature) {
    // Remove .feature extension if provided
    const featureName = options.feature.replace(/\.feature$/, '');
    featureFile = join(cwd, 'spec/features', `${featureName}.feature`);
  } else {
    // Default: create feature file based on work unit ID
    const kebabCase = options.workUnitId.toLowerCase().replace(/_/g, '-');
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
    const updatedContent = existingContent.trimEnd() + '\n\n' + scenarios.join('\n') + '\n';
    await writeFile(featureFile, updatedContent);
  } else {
    // Create new feature file
    const title = workUnit.title || options.workUnitId;
    const featureContent = `Feature: ${title}

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

${scenarios.join('\n')}`;
    await writeFile(featureFile, featureContent);
  }

  return {
    success: true,
    featureFile,
    scenariosCount: scenarios.length,
  };
}
