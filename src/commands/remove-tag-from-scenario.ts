import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface RemoveTagFromScenarioOptions {
  cwd?: string;
}

interface RemoveTagFromScenarioResult {
  success: boolean;
  valid: boolean;
  message?: string;
  error?: string;
}

export async function removeTagFromScenario(
  featureFilePath: string,
  scenarioName: string,
  tags: string[],
  options: RemoveTagFromScenarioOptions = {}
): Promise<RemoveTagFromScenarioResult> {
  const cwd = options.cwd || process.cwd();
  const filePath = join(cwd, featureFilePath);

  // Read feature file
  let content: string;
  try {
    content = await readFile(filePath, 'utf-8');
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      return {
        success: false,
        valid: false,
        error: `File not found: ${featureFilePath}`,
      };
    }
    throw error;
  }

  // Parse Gherkin to find the scenario
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
      error: `Invalid Gherkin syntax: ${error.message}`,
    };
  }

  if (!gherkinDocument.feature) {
    return {
      success: false,
      valid: false,
      error: 'File does not contain a valid Feature',
    };
  }

  // Find the scenario by name
  const scenarios = gherkinDocument.feature.children.filter(
    child => child.scenario && child.scenario.keyword === 'Scenario'
  );

  const targetScenario = scenarios.find(
    child => child.scenario?.name === scenarioName
  );

  if (!targetScenario || !targetScenario.scenario) {
    // Idempotent behavior: succeed if scenario not found (no work to do)
    return {
      success: true,
      valid: true,
      message: `Scenario '${scenarioName}' not found in ${featureFilePath} - no changes made`,
    };
  }

  // Get existing scenario tags
  const existingTags = targetScenario.scenario.tags.map(t => t.name);

  // Check if any tags exist (filter out nonexistent tags for idempotent behavior)
  const tagsToActuallyRemove = tags.filter(tag => existingTags.includes(tag));

  if (tagsToActuallyRemove.length === 0) {
    // Idempotent behavior: succeed even if no tags to remove
    return {
      success: true,
      valid: true,
      message: `No changes made - none of the specified tags found on scenario '${scenarioName}'`,
    };
  }

  // Find the scenario line in the content
  const lines = content.split('\n');
  let scenarioLineIndex = -1;

  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    if (trimmed === `Scenario: ${scenarioName}`) {
      scenarioLineIndex = i;
      break;
    }
  }

  if (scenarioLineIndex === -1) {
    return {
      success: false,
      valid: false,
      error: `Could not find Scenario line for "${scenarioName}"`,
    };
  }

  // Find and remove the tags that belong to this scenario
  // Tags are on lines immediately before the Scenario line
  const tagsToRemove = new Set(tagsToActuallyRemove);
  const linesToKeep: string[] = [];
  let i = 0;

  while (i < lines.length) {
    if (i < scenarioLineIndex) {
      // Check if this is a tag line belonging to our scenario
      const trimmed = lines[i].trim();
      if (trimmed.startsWith('@')) {
        // Check if this tag is before our scenario and after any previous scenario/feature
        let belongsToTargetScenario = false;
        let foundTargetScenario = false;

        for (let j = i + 1; j < lines.length; j++) {
          const nextTrimmed = lines[j].trim();
          if (nextTrimmed === `Scenario: ${scenarioName}`) {
            belongsToTargetScenario = true;
            foundTargetScenario = true;
            break;
          }
          if (
            nextTrimmed.startsWith('Scenario:') ||
            nextTrimmed.startsWith('Feature:')
          ) {
            break;
          }
        }

        if (belongsToTargetScenario && tagsToRemove.has(trimmed)) {
          // Skip this line (remove the tag)
          i++;
          continue;
        }
      }
    }

    linesToKeep.push(lines[i]);
    i++;
  }

  const newContent = linesToKeep.join('\n');

  // Validate the result is valid Gherkin
  let valid = true;
  try {
    const testBuilder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const testMatcher = new Gherkin.GherkinClassicTokenMatcher();
    const testParser = new Gherkin.Parser(testBuilder, testMatcher);
    testParser.parse(newContent);
  } catch {
    valid = false;
  }

  // Write file
  await writeFile(filePath, newContent, 'utf-8');

  const tagList = tagsToActuallyRemove.join(', ');
  return {
    success: true,
    valid,
    message: `Removed ${tagList} from scenario '${scenarioName}'`,
  };
}

export async function removeTagFromScenarioCommand(
  featureFilePath: string,
  scenarioName: string,
  tags: string[]
): Promise<void> {
  try {
    const result = await removeTagFromScenario(
      featureFilePath,
      scenarioName,
      tags
    );

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

export function registerRemoveTagFromScenarioCommand(program: Command): void {
  program
    .command('remove-tag-from-scenario')
    .description('Remove one or more tags from a specific scenario')
    .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
    .argument(
      '<scenario>',
      'Scenario name (e.g., "Login with valid credentials")'
    )
    .argument('<tags...>', 'Tag(s) to remove (e.g., @wip @deprecated)')
    .action(async (file: string, scenario: string, tags: string[]) => {
      await removeTagFromScenarioCommand(file, scenario, tags);
    });
}
