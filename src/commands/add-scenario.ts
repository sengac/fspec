import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface AddScenarioOptions {
  cwd?: string;
  dryRun?: boolean;
}

interface AddScenarioResult {
  success: boolean;
  valid: boolean;
  error?: string;
  suggestion?: string;
  warning?: string;
}

export async function addScenario(
  featureIdentifier: string,
  scenarioName: string,
  options: AddScenarioOptions = {}
): Promise<AddScenarioResult> {
  const cwd = options.cwd || process.cwd();

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

  // Check for duplicate scenario names
  let warning: string | undefined;
  const existingScenarios = gherkinDocument.feature.children.filter(
    child => child.scenario && child.scenario.keyword === 'Scenario'
  );
  const duplicateExists = existingScenarios.some(
    child => child.scenario?.name === scenarioName
  );
  if (duplicateExists) {
    warning = `A scenario named "${scenarioName}" already exists in this feature`;
  }

  // Create new scenario template
  const scenarioTemplate = `
  Scenario: ${scenarioName}
    Given [precondition]
    When [action]
    Then [expected outcome]
`;

  // Find insertion point (before Scenario Outline, or at end)
  const lines = content.split('\n');
  let insertIndex = lines.length;

  // Look for Scenario Outline
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    if (
      trimmed.startsWith('Scenario Outline:') ||
      trimmed.startsWith('Scenario Template:')
    ) {
      insertIndex = i;
      break;
    }
  }

  // Insert scenario
  const newContent =
    lines.slice(0, insertIndex).join('\n') +
    scenarioTemplate +
    '\n' +
    lines.slice(insertIndex).join('\n');

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
    warning,
  };
}

export async function addScenarioCommand(
  featureIdentifier: string,
  scenarioName: string
): Promise<void> {
  try {
    const result = await addScenario(featureIdentifier, scenarioName);

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      if (result.suggestion) {
        console.log(chalk.yellow('Suggestion:'), result.suggestion);
      }
      process.exit(1);
    }

    if (result.warning) {
      console.log(chalk.yellow('⚠'), result.warning);
    }

    console.log(chalk.green(`✓ Added scenario "${scenarioName}"`));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
