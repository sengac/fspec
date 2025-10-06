import { readFile, writeFile, access } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

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
}

export async function showFeature(options: ShowFeatureOptions): Promise<ShowFeatureResult> {
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
      const files = await glob(['spec/features/**/*.feature'], { cwd, absolute: false });
      const matchingFile = files.find((f) => {
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

    // Format output
    let outputContent: string;

    if (format === 'json') {
      // Output as JSON
      outputContent = JSON.stringify(gherkinDocument, null, 2);
    } else {
      // Output as plain text (original content)
      outputContent = content;
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
      console.log(chalk.green('✓'), `Feature content written to ${options.output}`);
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
