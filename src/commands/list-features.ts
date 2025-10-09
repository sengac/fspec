import { readFile, access } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

export interface FeatureInfo {
  file: string;
  name: string;
  scenarioCount: number;
  tags: string[];
}

export interface ListFeaturesOptions {
  cwd?: string;
  tag?: string;
}

export interface ListFeaturesResult {
  features: FeatureInfo[];
}

export async function listFeatures(
  options: ListFeaturesOptions = {}
): Promise<ListFeaturesResult> {
  const cwd = options.cwd || process.cwd();
  const featuresDir = join(cwd, 'spec', 'features');

  // Check if directory exists
  try {
    await access(featuresDir);
  } catch (error) {
    throw new Error(`Directory not found: spec/features/`);
  }

  // Find all feature files
  const files = await glob(['spec/features/**/*.feature'], {
    cwd,
    absolute: false,
  });

  if (files.length === 0) {
    return { features: [] };
  }

  // Parse each feature file
  const features: FeatureInfo[] = [];

  for (const file of files) {
    const filePath = join(cwd, file);
    const content = await readFile(filePath, 'utf-8');

    try {
      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      const gherkinDocument = parser.parse(content);

      if (gherkinDocument.feature) {
        const tags = gherkinDocument.feature.tags.map(tag => tag.name);

        // Filter by tag if specified
        if (options.tag && !tags.includes(options.tag)) {
          continue;
        }

        // Count scenarios
        const scenarioCount = gherkinDocument.feature.children.filter(
          child => child.scenario !== undefined
        ).length;

        features.push({
          file,
          name: gherkinDocument.feature.name,
          scenarioCount,
          tags,
        });
      }
    } catch (error) {
      // Skip files that fail to parse
      console.warn(chalk.yellow(`Warning: Could not parse ${file}`));
    }
  }

  // Sort alphabetically by file path
  features.sort((a, b) => a.file.localeCompare(b.file));

  return { features };
}

export async function listFeaturesCommand(options?: {
  tag?: string;
}): Promise<void> {
  try {
    const result = await listFeatures({ tag: options?.tag });

    if (result.features.length === 0) {
      console.log(chalk.yellow('No feature files found in spec/features/'));
      process.exit(0);
    }

    // Display features
    for (const feature of result.features) {
      const tagsStr =
        feature.tags.length > 0 ? ` [${feature.tags.join(' ')}]` : '';
      console.log(
        `  ${chalk.blue(feature.file)} - ${feature.name} ${chalk.gray(`(${feature.scenarioCount} scenarios)`)}${chalk.gray(tagsStr)}`
      );
    }

    // Summary
    console.log('');
    if (options?.tag) {
      console.log(
        chalk.green(
          `Found ${result.features.length} feature files matching ${options.tag}`
        )
      );
    } else {
      console.log(chalk.green(`Found ${result.features.length} feature files`));
    }

    process.exit(0);
  } catch (error: any) {
    if (error.message.includes('Directory not found')) {
      console.error(chalk.red(error.message));
      console.log(
        chalk.gray(
          "  Suggestion: Run 'fspec create-feature' to create your first feature"
        )
      );
      process.exit(2);
    }
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
