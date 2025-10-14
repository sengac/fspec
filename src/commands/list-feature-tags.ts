import { readFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface ListFeatureTagsOptions {
  cwd?: string;
  showCategories?: boolean;
}

interface ListFeatureTagsResult {
  success: boolean;
  tags: string[];
  message?: string;
  error?: string;
  categorizedTags?: Array<{ tag: string; category: string }>;
}

export async function listFeatureTags(
  featureFilePath: string,
  options: ListFeatureTagsOptions = {}
): Promise<ListFeatureTagsResult> {
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
        tags: [],
        error: `File not found: ${featureFilePath}`,
      };
    }
    throw error;
  }

  // Parse Gherkin to get tags
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
      tags: [],
      error: `Invalid Gherkin syntax: ${error.message}`,
    };
  }

  if (!gherkinDocument.feature) {
    return {
      success: false,
      tags: [],
      error: 'File does not contain a valid Feature',
    };
  }

  // Get feature-level tags
  const tags = gherkinDocument.feature.tags.map(t => t.name);

  if (tags.length === 0) {
    return {
      success: true,
      tags: [],
      message: 'No tags found on this feature',
    };
  }

  // If showCategories is enabled, load tag registry
  if (options.showCategories) {
    try {
      const tagsJsonPath = join(cwd, 'spec', 'tags.json');
      const tagsJson = JSON.parse(await readFile(tagsJsonPath, 'utf-8'));
      const tagToCategory = new Map<string, string>();

      for (const category of tagsJson.categories) {
        for (const tag of category.tags) {
          tagToCategory.set(tag.name, category.name);
        }
      }

      const categorizedTags = tags.map(tag => ({
        tag,
        category: tagToCategory.get(tag) || 'Unknown',
      }));

      return {
        success: true,
        tags,
        categorizedTags,
      };
    } catch (error: any) {
      // If we can't load categories, just return tags without categories
      return {
        success: true,
        tags,
      };
    }
  }

  return {
    success: true,
    tags,
  };
}

export async function listFeatureTagsCommand(
  featureFilePath: string,
  options: { showCategories?: boolean } = {}
): Promise<void> {
  try {
    const result = await listFeatureTags(featureFilePath, options);

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    if (result.message) {
      console.log(chalk.yellow(result.message));
      process.exit(0);
    }

    if (options.showCategories && result.categorizedTags) {
      console.log(chalk.bold('Tags on this feature:\n'));
      console.log(
        chalk.gray(`${chalk.bold('Tag').padEnd(20)} ${chalk.bold('Category')}`)
      );
      console.log(chalk.gray('─'.repeat(50)));

      for (const { tag, category } of result.categorizedTags) {
        console.log(`${chalk.cyan(tag.padEnd(20))} ${chalk.gray(category)}`);
      }
    } else {
      console.log(chalk.bold('Tags on this feature:\n'));
      for (const tag of result.tags) {
        console.log(chalk.cyan(`  ${tag}`));
      }
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerListFeatureTagsCommand(program: Command): void {
  program
    .command('list-feature-tags')
    .description('List all tags on a specific feature file')
    .argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')
    .option('--show-categories', 'Show tag categories from registry')
    .action(async (file: string, options: { showCategories?: boolean }) => {
      await listFeatureTagsCommand(file, options);
    });
}
