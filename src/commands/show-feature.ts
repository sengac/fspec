import { readFile, writeFile, access } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import {
  extractWorkUnitTags,
  loadWorkUnitsData,
  enrichWorkUnitTags,
  type WorkUnitInfo,
} from '../utils/work-unit-tags';

interface ShowFeatureOptions {
  feature: string;
  format?: 'text' | 'json';
  output?: string;
  cwd?: string;
}

interface ShowFeatureResult {
  success: boolean;
  content?: string;
  format?: 'text' | 'json';
  validated?: boolean;
  error?: string;
  workUnits?: WorkUnitInfo[];
}

export async function showFeature(
  options: ShowFeatureOptions
): Promise<ShowFeatureResult> {
  const { feature, format = 'text', output, cwd = process.cwd() } = options;

  try {
    // Find the feature file
    let featurePath: string;

    // Check if it's a direct path
    if (feature.endsWith('.feature')) {
      featurePath = join(cwd, feature);
      try {
        await access(featurePath);
      } catch {
        return {
          success: false,
          error: `Feature file not found: ${feature}`,
        };
      }
    } else {
      // Search for the feature file by name
      const files = await glob(['spec/features/**/*.feature'], {
        cwd,
        absolute: false,
      });
      const matchingFile = files.find(f => {
        const basename = f.split('/').pop()?.replace('.feature', '');
        return basename === feature;
      });

      if (!matchingFile) {
        return {
          success: false,
          error: `Feature file not found: ${feature}`,
        };
      }

      featurePath = join(cwd, matchingFile);
    }

    // Read the feature file
    const content = await readFile(featurePath, 'utf-8');

    // Parse and validate Gherkin
    const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
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

    // Extract work unit tags
    const workUnitTags = extractWorkUnitTags(gherkinDocument);
    const workUnitsData = await loadWorkUnitsData(cwd);
    const workUnits = enrichWorkUnitTags(workUnitTags, workUnitsData);

    // Format output
    let outputContent: string;

    if (format === 'json') {
      // Output as JSON with work units
      const jsonOutput = {
        ...gherkinDocument,
        workUnits: workUnits.map(wu => ({
          id: wu.id,
          title: wu.title,
          status: wu.status,
          level: wu.level,
          scenarios: wu.scenarios,
        })),
      };
      outputContent = JSON.stringify(jsonOutput, null, 2);
    } else {
      // Output as plain text with work units section
      let textOutput = content;

      if (workUnits.length > 0) {
        textOutput += '\n\n';
        textOutput += 'Work Units:\n';
        for (const wu of workUnits) {
          const levelText =
            wu.level === 'feature' ? 'feature-level' : 'scenario-level';
          textOutput += `\n  ${wu.id} (${levelText}) - ${wu.title}\n`;
          for (const scenario of wu.scenarios) {
            const fileName = featurePath.split('/').pop() || '';
            textOutput += `    ${fileName}:${scenario.line} - ${scenario.name}\n`;
          }
        }
      } else {
        textOutput += '\n\n';
        textOutput += 'Work Units: None\n';
      }

      outputContent = textOutput;
    }

    // Write to file if output specified
    if (output) {
      await writeFile(output, outputContent, 'utf-8');
    }

    return {
      success: true,
      content: outputContent,
      format,
      validated: true,
      workUnits,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function showFeatureCommand(
  feature: string,
  options: {
    format?: 'text' | 'json';
    output?: string;
  }
): Promise<void> {
  try {
    const result = await showFeature({
      feature,
      format: options.format,
      output: options.output,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    // Only output to stdout if no output file specified
    if (!options.output) {
      console.log(result.content);
    } else {
      console.log(
        chalk.green('✓'),
        `Feature content written to ${options.output}`
      );
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerShowFeatureCommand(program: Command): void {
  program
    .command('show-feature')
    .description('Display feature file contents')
    .argument(
      '<feature>',
      'Feature file name or path (e.g., "login" or "spec/features/login.feature")'
    )
    .option('--format <format>', 'Output format: text or json', 'text')
    .option('--output <file>', 'Write output to file')
    .action(showFeatureCommand);
}
