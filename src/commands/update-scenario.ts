import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface UpdateScenarioOptions {
  feature: string;
  oldName: string;
  newName: string;
  cwd?: string;
}

interface UpdateScenarioResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function updateScenario(
  options: UpdateScenarioOptions
): Promise<UpdateScenarioResult> {
  const { feature, oldName, newName, cwd = process.cwd() } = options;

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

  // Find the scenario to rename
  const scenarioToRename = gherkinDocument.feature.children.find(
    child => child.scenario && child.scenario.name === oldName
  );

  if (!scenarioToRename || !scenarioToRename.scenario) {
    return {
      success: false,
      error: `Scenario '${oldName}' not found in feature file`,
    };
  }

  // Check if new name already exists
  const duplicateScenario = gherkinDocument.feature.children.find(
    child => child.scenario && child.scenario.name === newName
  );

  if (duplicateScenario) {
    return {
      success: false,
      error: `Scenario '${newName}' already exists in this feature`,
    };
  }

  // Get scenario location
  const scenarioLine = scenarioToRename.scenario.location.line;

  // Split content into lines
  const lines = content.split('\n');

  // Find the scenario header line (convert to 0-indexed)
  const lineIndex = scenarioLine - 1;
  const scenarioHeaderLine = lines[lineIndex];

  // Replace the scenario name while preserving indentation and keyword
  // Match pattern: (optional tags line)(whitespace)Scenario:(whitespace)(old name)
  const scenarioKeywordMatch = scenarioHeaderLine.match(
    /^(\s*)(Scenario|Scenario Outline):\s*(.+)$/
  );

  if (!scenarioKeywordMatch) {
    return {
      success: false,
      error: 'Could not parse scenario header line',
    };
  }

  const indentation = scenarioKeywordMatch[1];
  const keyword = scenarioKeywordMatch[2];
  const newScenarioLine = `${indentation}${keyword}: ${newName}`;

  // Replace the line
  const newLines = [...lines];
  newLines[lineIndex] = newScenarioLine;

  const newContent = newLines.join('\n');

  // Validate the new content is still valid Gherkin
  try {
    const newBuilder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const newMatcher = new Gherkin.GherkinClassicTokenMatcher();
    const newParser = new Gherkin.Parser(newBuilder, newMatcher);
    newParser.parse(newContent);
  } catch (error: any) {
    return {
      success: false,
      error: `Renaming would result in invalid Gherkin: ${error.message}`,
    };
  }

  // Write the updated content
  await writeFile(featurePath, newContent, 'utf-8');

  // Update coverage file to rename scenario entry
  const coverageFilePath = `${featurePath}.coverage`;
  try {
    const coverageContent = await readFile(coverageFilePath, 'utf-8');
    const coverage = JSON.parse(coverageContent);

    // Find and rename the scenario entry (preserving test mappings)
    const scenarioEntry = coverage.scenarios.find(
      (s: any) => s.name === oldName
    );
    if (scenarioEntry) {
      scenarioEntry.name = newName;

      // Write updated coverage
      await writeFile(
        coverageFilePath,
        JSON.stringify(coverage, null, 2),
        'utf-8'
      );
    }
  } catch (error: any) {
    // Coverage file doesn't exist or invalid - skip rename but still succeed
  }

  const fileName = featurePath.split('/').pop();
  return {
    success: true,
    message: `Successfully renamed scenario to '${newName}' in ${fileName}`,
  };
}

export async function updateScenarioCommand(
  feature: string,
  oldName: string,
  newName: string
): Promise<void> {
  try {
    const result = await updateScenario({ feature, oldName, newName });

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

export function registerUpdateScenarioCommand(program: Command): void {
  program
    .command('update-scenario')
    .description('Rename a scenario in a feature file')
    .argument('<feature>', 'Feature file name or path')
    .argument('<old-name>', 'Current scenario name')
    .argument('<new-name>', 'New scenario name')
    .action(updateScenarioCommand);
}
