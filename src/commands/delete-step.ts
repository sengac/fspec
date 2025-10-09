import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface DeleteStepOptions {
  feature: string;
  scenario: string;
  step: string;
  cwd?: string;
}

interface DeleteStepResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function deleteStep(
  options: DeleteStepOptions
): Promise<DeleteStepResult> {
  const { feature, scenario, step, cwd = process.cwd() } = options;

  // Resolve feature file path
  let featurePath: string;
  if (feature.endsWith('.feature')) {
    featurePath = join(cwd, feature);
  } else if (feature.startsWith('spec/features/')) {
    featurePath = join(cwd, feature);
  } else {
    featurePath = join(cwd, 'spec/features', `${feature}.feature`);
  }

  // Read feature file
  let content: string;
  try {
    content = await readFile(featurePath, 'utf-8');
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      return {
        success: false,
        error: `Feature file not found: ${featurePath}`,
      };
    }
    throw error;
  }

  // Parse Gherkin to find scenario and step
  const uuidFn = Messages.IdGenerator.uuid();
  const builder = new Gherkin.AstBuilder(uuidFn);
  const matcher = new Gherkin.GherkinClassicTokenMatcher();
  const parser = new Gherkin.Parser(builder, matcher);

  let gherkinDocument;
  try {
    gherkinDocument = parser.parse(content);
  } catch (error: any) {
    return {
      success: false,
      error: `Invalid Gherkin syntax: ${error.message}`,
    };
  }

  if (!gherkinDocument.feature) {
    return {
      success: false,
      error: 'Feature file does not contain a valid Feature',
    };
  }

  // Find the scenario
  const scenarioChild = gherkinDocument.feature.children.find(
    child => child.scenario && child.scenario.name === scenario
  );

  if (!scenarioChild || !scenarioChild.scenario) {
    return {
      success: false,
      error: `Scenario '${scenario}' not found in feature file`,
    };
  }

  // Find the step by matching text (without keyword)
  const stepToDelete = scenarioChild.scenario.steps.find(s => {
    // Match both with and without keyword
    const stepText = s.text;
    const fullStepText = `${s.keyword}${s.text}`;
    return stepText === step || fullStepText.trim() === step.trim();
  });

  if (!stepToDelete) {
    return {
      success: false,
      error: `Step '${step}' not found in scenario '${scenario}'`,
    };
  }

  // Get step location
  const stepLine = stepToDelete.location.line;

  // Split content into lines
  const lines = content.split('\n');

  // Remove the step line (convert to 0-indexed)
  const lineIndex = stepLine - 1;
  const newLines = [
    ...lines.slice(0, lineIndex),
    ...lines.slice(lineIndex + 1),
  ];

  // Clean up extra blank lines if created
  const trimmedLines = [];
  let consecutiveEmpty = 0;
  for (const line of newLines) {
    if (line.trim() === '') {
      consecutiveEmpty++;
      if (consecutiveEmpty <= 2) {
        trimmedLines.push(line);
      }
    } else {
      consecutiveEmpty = 0;
      trimmedLines.push(line);
    }
  }

  const newContent = trimmedLines.join('\n');

  // Validate the new content is still valid Gherkin
  try {
    const newBuilder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const newMatcher = new Gherkin.GherkinClassicTokenMatcher();
    const newParser = new Gherkin.Parser(newBuilder, newMatcher);
    newParser.parse(newContent);
  } catch (error: any) {
    return {
      success: false,
      error: `Deletion would result in invalid Gherkin: ${error.message}`,
    };
  }

  // Write the updated content
  await writeFile(featurePath, newContent, 'utf-8');

  const fileName = featurePath.split('/').pop();
  return {
    success: true,
    message: `Successfully deleted step from scenario '${scenario}' in ${fileName}`,
  };
}

export async function deleteStepCommand(
  feature: string,
  scenario: string,
  step: string
): Promise<void> {
  try {
    const result = await deleteStep({ feature, scenario, step });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green(`✓ ${result.message}`));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
