import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface AddStepOptions {
  cwd?: string;
  dryRun?: boolean;
}

interface AddStepResult {
  success: boolean;
  valid: boolean;
  error?: string;
  suggestion?: string;
}

const VALID_STEP_TYPES = ['given', 'when', 'then', 'and', 'but'];

export async function addStep(
  featureIdentifier: string,
  scenarioName: string,
  stepType: string,
  stepText: string,
  options: AddStepOptions = {}
): Promise<AddStepResult> {
  const cwd = options.cwd || process.cwd();

  // Normalize and validate step type
  const normalizedStepType = stepType.toLowerCase();
  if (!VALID_STEP_TYPES.includes(normalizedStepType)) {
    return {
      success: false,
      valid: false,
      error: `Invalid step type: "${stepType}"`,
      suggestion: `Valid step types are: ${VALID_STEP_TYPES.join(', ')}`,
    };
  }

  // Capitalize step keyword
  const stepKeyword =
    normalizedStepType.charAt(0).toUpperCase() + normalizedStepType.slice(1);

  // Resolve feature file path
  let featurePath: string;
  if (featureIdentifier.endsWith('.feature')) {
    featurePath = join(cwd, featureIdentifier);
  } else if (featureIdentifier.startsWith('spec/features/')) {
    featurePath = join(cwd, featureIdentifier);
  } else {
    featurePath = join(cwd, 'spec/features', `${featureIdentifier}.feature`);
  }

  // Check if file exists
  let content: string;
  try {
    content = await readFile(featurePath, 'utf-8');
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      return {
        success: false,
        valid: false,
        error: `Feature file not found: ${featurePath}`,
        suggestion: `Use 'fspec create-feature' to create a new feature file`,
      };
    }
    throw error;
  }

  // Validate existing Gherkin syntax
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
      valid: false,
      error: `Feature file has invalid Gherkin syntax: ${error.message}`,
      suggestion: `Run 'fspec validate ${featureIdentifier}' to see syntax errors`,
    };
  }

  if (!gherkinDocument.feature) {
    return {
      success: false,
      valid: false,
      error: 'Feature file does not contain a valid Feature',
      suggestion: `Run 'fspec validate ${featureIdentifier}' to see syntax errors`,
    };
  }

  // Find the scenario
  const scenarios = gherkinDocument.feature.children.filter(
    child => child.scenario && child.scenario.keyword === 'Scenario'
  );

  const targetScenario = scenarios.find(
    child => child.scenario?.name === scenarioName
  );

  if (!targetScenario || !targetScenario.scenario) {
    const availableScenarios = scenarios
      .map(s => s.scenario?.name)
      .filter(Boolean)
      .join(', ');
    return {
      success: false,
      valid: false,
      error: `Scenario not found: "${scenarioName}"`,
      suggestion: `Available scenarios: ${availableScenarios || 'none'}`,
    };
  }

  // Get scenario location
  const scenarioLocation = targetScenario.scenario.location;
  const lines = content.split('\n');

  // Find the line where scenario starts
  const scenarioLineIndex = scenarioLocation.line - 1; // 0-indexed

  // Determine indentation from existing steps
  let stepIndentation = '    '; // Default 4 spaces
  if (
    targetScenario.scenario.steps &&
    targetScenario.scenario.steps.length > 0
  ) {
    const firstStep = targetScenario.scenario.steps[0];
    const firstStepLineIndex = firstStep.location.line - 1;
    const firstStepLine = lines[firstStepLineIndex];
    const match = firstStepLine.match(/^(\s+)/);
    if (match) {
      stepIndentation = match[1];
    }
  }

  // Find insertion point (after last step, but before data tables/doc strings)
  let insertIndex = scenarioLineIndex + 1;

  // Find the last step of this scenario
  if (
    targetScenario.scenario.steps &&
    targetScenario.scenario.steps.length > 0
  ) {
    const lastStep =
      targetScenario.scenario.steps[targetScenario.scenario.steps.length - 1];
    const lastStepLineIndex = lastStep.location.line - 1; // 0-indexed

    // Start searching from the line after the last step keyword
    insertIndex = lastStep.location.line;

    // If the last step has a data table or doc string, we need to insert BEFORE it
    // So we insert right after the step keyword line itself
    // Check if next line is a data table (starts with |) or doc string (""")
    if (lastStepLineIndex + 1 < lines.length) {
      const nextLine = lines[lastStepLineIndex + 1].trim();
      if (nextLine.startsWith('|') || nextLine.startsWith('"""')) {
        // Insert before the data table/doc string
        insertIndex = lastStepLineIndex + 1;
      }
    }
  } else {
    // No existing steps, insert after scenario line
    // Look for the line after scenario declaration
    for (let i = scenarioLineIndex + 1; i < lines.length; i++) {
      const trimmed = lines[i].trim();
      if (
        trimmed.startsWith('Scenario:') ||
        trimmed.startsWith('Scenario Outline:') ||
        trimmed === ''
      ) {
        insertIndex = i;
        break;
      }
      insertIndex = i + 1;
    }
  }

  // Create new step
  const newStep = `${stepIndentation}${stepKeyword} ${stepText}`;

  // Insert step
  const newContent = [
    ...lines.slice(0, insertIndex),
    newStep,
    ...lines.slice(insertIndex),
  ].join('\n');

  // Validate result is valid Gherkin
  let valid = true;
  try {
    const testBuilder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const testMatcher = new Gherkin.GherkinClassicTokenMatcher();
    const testParser = new Gherkin.Parser(testBuilder, testMatcher);
    testParser.parse(newContent);
  } catch {
    valid = false;
  }

  // Write file if not dry run
  if (!options.dryRun) {
    await writeFile(featurePath, newContent, 'utf-8');
  }

  return {
    success: true,
    valid,
  };
}

export async function addStepCommand(
  featureIdentifier: string,
  scenarioName: string,
  stepType: string,
  stepText: string
): Promise<void> {
  try {
    const result = await addStep(
      featureIdentifier,
      scenarioName,
      stepType,
      stepText
    );

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      if (result.suggestion) {
        console.log(chalk.yellow('Suggestion:'), result.suggestion);
      }
      process.exit(1);
    }

    console.log(
      chalk.green(`✓ Added ${stepType} step to scenario "${scenarioName}"`)
    );
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
