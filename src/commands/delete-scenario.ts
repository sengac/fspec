import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface DeleteScenarioOptions {
  feature: string;
  scenario: string;
  cwd?: string;
}

interface DeleteScenarioResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function deleteScenario(
  options: DeleteScenarioOptions
): Promise<DeleteScenarioResult> {
  const { feature, scenario, cwd = process.cwd() } = options;

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

  // Parse Gherkin to find scenario
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

  // Find the scenario to delete
  const scenarioChild = gherkinDocument.feature.children.find(
    child => child.scenario && child.scenario.name === scenario
  );

  if (!scenarioChild || !scenarioChild.scenario) {
    return {
      success: false,
      error: `Scenario '${scenario}' not found in feature file`,
    };
  }

  // Get scenario location (line numbers)
  const scenarioLocation = scenarioChild.scenario.location;
  const scenarioStartLine = scenarioLocation.line;

  // Find the end line of the scenario (last step or scenario line if no steps)
  let scenarioEndLine = scenarioStartLine;
  if (scenarioChild.scenario.steps && scenarioChild.scenario.steps.length > 0) {
    const lastStep = scenarioChild.scenario.steps[scenarioChild.scenario.steps.length - 1];
    scenarioEndLine = lastStep.location.line;
  }

  // Split content into lines
  const lines = content.split('\n');

  // Find the actual end of scenario block (including empty lines after)
  let actualEndLine = scenarioEndLine;
  for (let i = scenarioEndLine; i < lines.length; i++) {
    const line = lines[i];
    // Stop at next scenario, background, or feature header
    if (
      line.trim().startsWith('Scenario:') ||
      line.trim().startsWith('Scenario Outline:') ||
      line.trim().startsWith('Background:') ||
      line.trim().startsWith('Feature:') ||
      line.trim().startsWith('Examples:')
    ) {
      break;
    }
    // Include empty lines after the scenario
    if (line.trim() === '') {
      actualEndLine = i;
    } else if (i > scenarioEndLine) {
      // Stop if we hit non-empty content that's not part of the scenario
      break;
    }
  }

  // Remove scenario lines (convert to 0-indexed)
  const startIndex = scenarioStartLine - 1;
  const endIndex = actualEndLine; // Remove through this line (inclusive)

  const newLines = [...lines.slice(0, startIndex), ...lines.slice(endIndex + 1)];

  // Remove extra blank lines if we created them
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
    message: `Successfully deleted scenario '${scenario}' from ${fileName}`,
  };
}

export async function deleteScenarioCommand(
  feature: string,
  scenario: string
): Promise<void> {
  try {
    const result = await deleteScenario({ feature, scenario });

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
